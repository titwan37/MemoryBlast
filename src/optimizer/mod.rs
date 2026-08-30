//! Proactive Optimization Assistant
//!
//! Generates actionable recommendations based on behavioral analytics and
//! provides low-level Win32 memory reclamation mechanisms:
//! - Working set trimming via Win32 `K32EmptyWorkingSet`
//! - Process termination and tab reclamation
//! - Automatic ranking of high-impact optimization opportunities

use serde::{Deserialize, Serialize};
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::ProcessStatus::K32EmptyWorkingSet;
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_SET_QUOTA,
};

use crate::analytics::ProcessAnomalyMetrics;
use crate::knowledge::{BehaviorCluster, OomForecast, ProcessLifespanProfile};
use crate::proc::kill_process;

/// Type of recommended optimization action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionType {
    TrimWorkingSet,
    HibernateTab,
    RestartProcess,
    InspectLeak,
}

/// A specific, actionable optimization recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRecommendation {
    pub id: String,
    pub title: String,
    pub description: String,
    pub target_pid: u32,
    pub target_name: String,
    pub action_type: ActionType,
    pub potential_reclaim_bytes: u64,
    pub confidence_score: f32, // 0.0 - 1.0
    pub urgency: &'static str,  // "INFO", "RECOMMENDED", "URGENT"
}

/// Assistant generating recommendations and executing memory actions
pub struct OptimizationAssistant;

impl OptimizationAssistant {
    /// Generates ranked recommendations based on cognitive learner profiles and anomaly metrics
    pub fn generate_recommendations(
        profiles: &[ProcessLifespanProfile],
        anomalies: &[ProcessAnomalyMetrics],
        forecast: &OomForecast,
    ) -> Vec<OptimizationRecommendation> {
        let mut recommendations = Vec::new();

        // 1. Process Anomalies & Leak Suspects
        for anomaly in anomalies {
            if anomaly.leak_velocity_mb_min > 15.0 {
                recommendations.push(OptimizationRecommendation {
                    id: format!("leak-{}", anomaly.pid),
                    title: format!("Memory Leak Suspect: {}", anomaly.name),
                    description: format!(
                        "Process is leaking memory at +{:.1} MB/min (Z-Score: +{:.1}). Inspect or terminate to prevent exhaustion.",
                        anomaly.leak_velocity_mb_min, anomaly.z_score
                    ),
                    target_pid: anomaly.pid,
                    target_name: anomaly.name.clone(),
                    action_type: ActionType::InspectLeak,
                    potential_reclaim_bytes: anomaly.current_bytes.saturating_sub((anomaly.rolling_mean_bytes as u64) / 2),
                    confidence_score: 0.88,
                    urgency: if anomaly.leak_velocity_mb_min > 50.0 { "URGENT" } else { "RECOMMENDED" },
                });
            }
        }

        // 2. Stale Background Tabs & Apps
        for profile in profiles {
            if profile.cluster == BehaviorCluster::StaleBackgroundTab {
                let dwell_mins = profile.dwell_duration_secs / 60.0;
                let mb = profile.current_working_set_bytes as f64 / (1024.0 * 1024.0);

                if profile.is_tab {
                    recommendations.push(OptimizationRecommendation {
                        id: format!("tab-{}", profile.pid),
                        title: format!("Hibernate Inactive Tab: {}", profile.display_title),
                        description: format!(
                            "Tab has been idle for {:.0} minutes consuming {:.0} MB. Trimming working set can release memory to standby pool.",
                            dwell_mins, mb
                        ),
                        target_pid: profile.pid,
                        target_name: profile.name.clone(),
                        action_type: ActionType::TrimWorkingSet,
                        potential_reclaim_bytes: (profile.current_working_set_bytes as f64 * 0.70) as u64,
                        confidence_score: 0.92,
                        urgency: if mb > 400.0 { "RECOMMENDED" } else { "INFO" },
                    });
                } else if mb > 300.0 {
                    recommendations.push(OptimizationRecommendation {
                        id: format!("trim-{}", profile.pid),
                        title: format!("Trim Idle Application: {}", profile.name),
                        description: format!(
                            "Application has been idle for {:.0} minutes with {:.0} MB resident memory. Empty working set to free pages.",
                            dwell_mins, mb
                        ),
                        target_pid: profile.pid,
                        target_name: profile.name.clone(),
                        action_type: ActionType::TrimWorkingSet,
                        potential_reclaim_bytes: (profile.current_working_set_bytes as f64 * 0.60) as u64,
                        confidence_score: 0.85,
                        urgency: "INFO",
                    });
                }
            }
        }

        // 3. System Commit Warning
        if forecast.risk_level == "CRITICAL" || forecast.risk_level == "WARNING" {
            if let Some(sec) = forecast.estimated_seconds_to_oom {
                recommendations.push(OptimizationRecommendation {
                    id: "system-commit-warning".to_string(),
                    title: "System Commit Limit Approaching".to_string(),
                    description: format!(
                        "Host commit charge is {:.1}% and increasing by +{:.1} MB/min. Potential memory exhaustion in ~{:.0} minutes.",
                        forecast.current_commit_pct,
                        forecast.velocity_mb_min,
                        sec / 60.0
                    ),
                    target_pid: 0,
                    target_name: "System".to_string(),
                    action_type: ActionType::InspectLeak,
                    potential_reclaim_bytes: 0,
                    confidence_score: 0.95,
                    urgency: forecast.risk_level,
                });
            }
        }

        // Sort by urgency and potential reclaimed bytes
        recommendations.sort_by(|a, b| {
            let u_a = match a.urgency {
                "CRITICAL" | "URGENT" => 3,
                "RECOMMENDED" => 2,
                _ => 1,
            };
            let u_b = match b.urgency {
                "CRITICAL" | "URGENT" => 3,
                "RECOMMENDED" => 2,
                _ => 1,
            };
            u_b.cmp(&u_a)
                .then_with(|| b.potential_reclaim_bytes.cmp(&a.potential_reclaim_bytes))
        });

        recommendations
    }

    /// Trims the working set of a target process using Win32 `K32EmptyWorkingSet`
    pub fn trim_working_set(pid: u32) -> Result<(), String> {
        unsafe {
            let handle = OpenProcess(
                PROCESS_QUERY_INFORMATION | PROCESS_SET_QUOTA,
                false,
                pid,
            )
            .map_err(|e| format!("Failed to open process {pid} for working set trimming: {e}"))?;

            let success = K32EmptyWorkingSet(handle);
            let _ = CloseHandle(handle);

            if success.as_bool() {
                Ok(())
            } else {
                Err(format!("K32EmptyWorkingSet returned false for PID {pid}"))
            }
        }
    }

    /// Terminates a process
    pub fn terminate_process(pid: u32) -> Result<(), String> {
        kill_process(pid)
    }
}
