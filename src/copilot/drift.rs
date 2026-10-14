//! Task Sprawl & Cognitive Drift Analytics
//!
//! Evaluates task switching velocity, tab dwell times, memory weight, and idle duration.
//! Detects "Forgotten Loops" (>90m idle, >300MB RAM) and monitors the Cognitive Working Set Ceiling.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::proc::ProcessItem;

/// Represents a detected open loop / abandoned research task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgottenLoop {
    pub pid: u32,
    pub title: String,
    pub browser_family: String,
    pub idle_duration_mins: u64,
    pub memory_mb: f64,
    pub cognitive_cost_score: f64,
    pub suggested_prompt: String,
}

/// Historical tracking of window/tab interaction
#[allow(dead_code)]
#[derive(Debug, Clone)]
struct TabTrackingEntry {
    first_seen: Instant,
    last_active: Instant,
    focus_switch_count: u32,
    peak_memory_bytes: u64,
    last_known_memory_bytes: u64,
}

/// State of the Cognitive Drift & Task Sprawl Engine
pub struct ContextDriftDetector {
    tab_history: HashMap<String, TabTrackingEntry>, // keyed by title or "pid_X"
    last_foreground_pid: Option<u32>,
    last_foreground_switch: Instant,
    recent_switches: Vec<Instant>,
    working_memory_cap: usize,
    idle_threshold: Duration,
    memory_threshold_bytes: u64,
}

impl ContextDriftDetector {
    pub fn new(working_memory_cap: usize) -> Self {
        Self {
            tab_history: HashMap::new(),
            last_foreground_pid: None,
            last_foreground_switch: Instant::now(),
            recent_switches: Vec::new(),
            working_memory_cap,
            idle_threshold: Duration::from_secs(90 * 60), // 90 minutes as specified in roadmap
            memory_threshold_bytes: 300 * 1024 * 1024,    // 300 MB as specified in roadmap
        }
    }

    /// Custom thresholds for testing or configuration
    #[allow(dead_code)]
    pub fn with_thresholds(
        working_memory_cap: usize,
        idle_threshold: Duration,
        memory_threshold_bytes: u64,
    ) -> Self {
        Self {
            tab_history: HashMap::new(),
            last_foreground_pid: None,
            last_foreground_switch: Instant::now(),
            recent_switches: Vec::new(),
            working_memory_cap,
            idle_threshold,
            memory_threshold_bytes,
        }
    }

    /// Updates the dynamic working memory cap
    pub fn set_working_memory_cap(&mut self, cap: usize) {
        self.working_memory_cap = cap;
    }

    /// Evaluates current active processes & tabs from latest snapshot
    pub fn evaluate(
        &mut self,
        processes: &[ProcessItem],
        current_foreground_pid: Option<u32>,
    ) -> DriftAssessment {
        let now = Instant::now();

        // 1. Track context switching velocity
        if let Some(fg_pid) = current_foreground_pid {
            if self.last_foreground_pid != Some(fg_pid) {
                self.last_foreground_pid = Some(fg_pid);
                self.last_foreground_switch = now;
                self.recent_switches.push(now);
            }
        }

        // Retain switches within rolling 5 minutes window
        let five_mins_ago = now.checked_sub(Duration::from_secs(300)).unwrap_or(now);
        self.recent_switches.retain(|&t| t > five_mins_ago);

        let switches_per_min = (self.recent_switches.len() as f64) / 5.0;

        // 2. Tally active browser tabs & renderers
        let mut active_tabs = Vec::new();
        let mut forgotten_loops = Vec::new();

        for proc in processes {
            let is_browser_tab = proc.is_chromium
                && (proc.proc_type_label == "Renderer" || !proc.display_title.is_empty());

            if !is_browser_tab {
                continue;
            }

            let key = if !proc.display_title.is_empty() {
                proc.display_title.clone()
            } else {
                format!("pid_{}", proc.pid)
            };

            let entry = self
                .tab_history
                .entry(key.clone())
                .or_insert_with(|| TabTrackingEntry {
                    first_seen: now,
                    last_active: now,
                    focus_switch_count: 0,
                    peak_memory_bytes: proc.working_set_bytes,
                    last_known_memory_bytes: proc.working_set_bytes,
                });

            entry.last_known_memory_bytes = proc.working_set_bytes;
            if proc.working_set_bytes > entry.peak_memory_bytes {
                entry.peak_memory_bytes = proc.working_set_bytes;
            }

            // If this process is the current foreground window, update last_active
            if current_foreground_pid == Some(proc.pid) {
                entry.last_active = now;
                entry.focus_switch_count += 1;
            }

            let idle_duration = now.duration_since(entry.last_active);
            let idle_mins = idle_duration.as_secs() / 60;
            let mem_mb = proc.working_set_bytes as f64 / (1024.0 * 1024.0);

            active_tabs.push((proc.pid, key.clone(), proc.working_set_bytes));

            // Check for "Forgotten Loop" criteria (>90m idle and >300MB, or scaled cognitive cost)
            let is_forgotten = idle_duration >= self.idle_threshold
                && proc.working_set_bytes >= self.memory_threshold_bytes;

            if is_forgotten {
                // Cognitive cost score scales with idle time and memory weight
                let cognitive_cost_score = (idle_mins as f64 / 30.0) * (mem_mb / 250.0);

                let suggested_prompt = format!(
                    "You opened '{}' {} minutes ago consuming {:.0} MB. Ready to wrap it up or park it for tomorrow?",
                    proc.display_title, idle_mins, mem_mb
                );

                forgotten_loops.push(ForgottenLoop {
                    pid: proc.pid,
                    title: proc.display_title.clone(),
                    browser_family: proc
                        .browser_family
                        .clone()
                        .unwrap_or_else(|| "Chromium".to_string()),
                    idle_duration_mins: idle_mins,
                    memory_mb: mem_mb,
                    cognitive_cost_score,
                    suggested_prompt,
                });
            }
        }

        // Clean up tabs no longer running
        let current_keys: std::collections::HashSet<String> = processes
            .iter()
            .map(|p| {
                if !p.display_title.is_empty() {
                    p.display_title.clone()
                } else {
                    format!("pid_{}", p.pid)
                }
            })
            .collect();
        self.tab_history.retain(|k, _| current_keys.contains(k));

        // 3. Compute Context Drift Index
        // Drift = (switch_rate / avg_focus_time_secs) * (active_tabs_count / working_memory_cap)
        let avg_focus_secs = if switches_per_min > 0.0 {
            60.0 / switches_per_min
        } else {
            300.0
        };

        let tab_ratio = (active_tabs.len() as f64) / (self.working_memory_cap.max(1) as f64);
        let context_drift_index = (switches_per_min / (avg_focus_secs.max(5.0))) * tab_ratio * 10.0;

        let severity = if context_drift_index < 0.3 {
            "Calm"
        } else if context_drift_index < 0.7 {
            "Focused Flow"
        } else if context_drift_index < 1.5 {
            "Elevated Drift"
        } else {
            "Task Fragmentation"
        };

        // 4. Cognitive Working Set Ceiling check
        let working_set_alert = if active_tabs.len() >= self.working_memory_cap {
            Some(format!(
                "Working Memory Cap reached: {}/{} tabs active. Park older context to prevent fatigue.",
                active_tabs.len(),
                self.working_memory_cap
            ))
        } else if active_tabs.len() >= self.working_memory_cap.saturating_sub(2) {
            Some(format!(
                "Approaching Working Memory Cap ({}/{} tabs).",
                active_tabs.len(),
                self.working_memory_cap
            ))
        } else {
            None
        };

        DriftAssessment {
            context_drift_index,
            drift_severity: severity,
            active_tab_count: active_tabs.len(),
            working_memory_cap: self.working_memory_cap,
            switches_per_min,
            forgotten_loops,
            working_set_alert,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftAssessment {
    pub context_drift_index: f64,
    pub drift_severity: &'static str,
    pub active_tab_count: usize,
    pub working_memory_cap: usize,
    pub switches_per_min: f64,
    pub forgotten_loops: Vec<ForgottenLoop>,
    pub working_set_alert: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forgotten_loop_detection() {
        let mut detector = ContextDriftDetector::with_thresholds(
            12,
            Duration::from_millis(50),      // short for test
            100 * 1024 * 1024,              // 100 MB
        );

        let dummy_proc = ProcessItem {
            pid: 1234,
            ppid: 1000,
            name: "chrome.exe".to_string(),
            display_title: "Overdue Research Document".to_string(),
            proc_type_label: "Renderer".to_string(),
            is_chromium: true,
            browser_family: Some("Google Chrome".to_string()),
            private_bytes: 120 * 1024 * 1024,
            working_set_bytes: 150 * 1024 * 1024,
            cpu_percent: 0.1,
            memory_percent: 1.0,
            gpu_percent: 0.0,
            children: Vec::new(),
            debug_port: None,
        };

        // First sample - creates entry
        let assessment1 = detector.evaluate(&[dummy_proc.clone()], None);
        assert_eq!(assessment1.forgotten_loops.len(), 0);

        // Sleep past threshold
        std::thread::sleep(Duration::from_millis(60));

        // Second sample without foreground focus - should trigger forgotten loop
        let assessment2 = detector.evaluate(&[dummy_proc.clone()], None);
        assert_eq!(assessment2.forgotten_loops.len(), 1);
        assert_eq!(assessment2.forgotten_loops[0].title, "Overdue Research Document");
    }

    #[test]
    fn test_working_set_cap_alert() {
        let mut detector = ContextDriftDetector::new(5); // Cap = 5

        let procs: Vec<ProcessItem> = (0..6)
            .map(|i| ProcessItem {
                pid: 2000 + i,
                ppid: 1000,
                name: "chrome.exe".to_string(),
                display_title: format!("Research Tab #{i}"),
                proc_type_label: "Renderer".to_string(),
                is_chromium: true,
                browser_family: Some("Google Chrome".to_string()),
                private_bytes: 50 * 1024 * 1024,
                working_set_bytes: 60 * 1024 * 1024,
                cpu_percent: 0.0,
                memory_percent: 0.5,
                gpu_percent: 0.0,
                children: Vec::new(),
                debug_port: None,
            })
            .collect();

        let assessment = detector.evaluate(&procs, None);
        assert_eq!(assessment.active_tab_count, 6);
        assert!(assessment.working_set_alert.is_some());
    }
}
