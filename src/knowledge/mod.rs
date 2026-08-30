//! Behavioral Knowledge Management & Cognitive Learner
//!
//! Classifies processes and tabs into behavioral clusters:
//! - Active Task Focus
//! - Stale Background Tab
//! - Memory Leak Suspect
//! - Periodic Background Daemon
//! - Essential System Service
//!
//! Persists historical memory footprints and behavioral archetypes into an
//! embedded SQLite store (`rusqlite`) and provides linear regression OOM forecasting.

pub mod adaptive_engine;
pub mod closure;
pub mod consolidator;
pub mod explorer_history;
pub mod orientation;
pub mod schema;
pub mod tab_analytics;
pub mod tab_history;

#[allow(unused_imports)]
pub use adaptive_engine::{global_adaptive, AdaptiveEngine, AdaptiveProfile};
#[allow(unused_imports)]
pub use consolidator::{global_consolidator, DailyProfile, DailyWorkArchetype, KnowledgeConsolidator};
#[allow(unused_imports)]
pub use explorer_history::{global_explorer_history, ExplorerHistoryStore, ExplorerLifecycleRecord, WorkspaceRootSummary};
pub use orientation::ExecutiveBriefing;
#[allow(unused_imports)]
pub use schema::{initialize_knowledge_schema, open_knowledge_db};
#[allow(unused_imports)]
pub use tab_analytics::{DomainUsageSummary, TabAnalyticsReport, TabMortalityCurve};
#[allow(unused_imports)]
pub use tab_history::{global_tab_history, TabHistoryStore, TabLifecycleRecord};

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::proc::ProcessItem;

/// Semantic behavior clusters for processes and Chromium tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BehaviorCluster {
    ActiveTaskFocus,
    StaleBackgroundTab,
    MemoryLeakSuspect,
    PeriodicBackgroundDaemon,
    EssentialSystemService,
    StandardApp,
}

impl BehaviorCluster {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ActiveTaskFocus => "Active Task Focus",
            Self::StaleBackgroundTab => "Stale Background Tab",
            Self::MemoryLeakSuspect => "Memory Leak Suspect",
            Self::PeriodicBackgroundDaemon => "Periodic Daemon",
            Self::EssentialSystemService => "System Service",
            Self::StandardApp => "Standard Application",
        }
    }
}

/// Profile tracking lifetime behavior of a process or tab
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessLifespanProfile {
    pub pid: u32,
    pub name: String,
    pub display_title: String,
    pub is_tab: bool,
    pub first_seen_ms: u64,
    pub dwell_duration_secs: f64,
    pub peak_working_set_bytes: u64,
    pub current_working_set_bytes: u64,
    pub cumulative_cpu_activity: f64,
    pub cluster: BehaviorCluster,
    pub recommendation: Option<String>,
}

/// OOM Forecast prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OomForecast {
    pub current_commit_pct: f64,
    pub velocity_mb_min: f64,
    pub estimated_seconds_to_oom: Option<f64>,
    pub risk_level: &'static str,
}

/// Embedded SQLite database for persisting behavioral knowledge
pub struct KnowledgeStore {
    db_path: PathBuf,
}

impl KnowledgeStore {
    pub fn new(storage_dir: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        std::fs::create_dir_all(storage_dir)?;
        let db_path = storage_dir.join("knowledge.db");

        let conn = Connection::open(&db_path)?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS process_archetypes (
                name TEXT PRIMARY KEY,
                archetype TEXT NOT NULL,
                typical_bytes INTEGER NOT NULL,
                observation_count INTEGER NOT NULL,
                last_seen_epoch INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS daily_summaries (
                date_str TEXT PRIMARY KEY,
                peak_bytes INTEGER NOT NULL,
                avg_bytes INTEGER NOT NULL,
                optimizations_performed INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS optimization_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp INTEGER NOT NULL,
                action TEXT NOT NULL,
                target_name TEXT NOT NULL,
                pid INTEGER NOT NULL,
                reclaimed_bytes INTEGER NOT NULL
            );
            ",
        )?;

        Ok(Self { db_path })
    }

    pub fn record_optimization(
        &self,
        action: &str,
        target_name: &str,
        pid: u32,
        reclaimed_bytes: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = Connection::open(&self.db_path)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        conn.execute(
            "INSERT INTO optimization_logs (timestamp, action, target_name, pid, reclaimed_bytes)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![now, action, target_name, pid as i64, reclaimed_bytes as i64],
        )?;

        Ok(())
    }
}

/// Cognitive learner continuously updating process behavioral graphs
pub struct CognitiveLearner {
    profiles: HashMap<u32, ProcessLifespanProfile>,
    last_eval: Instant,
}

impl CognitiveLearner {
    pub fn new() -> Self {
        Self {
            profiles: HashMap::new(),
            last_eval: Instant::now(),
        }
    }

    /// Evaluates current snapshot items and updates behavioral clusters
    pub fn evaluate_snapshot(
        &mut self,
        processes: &[ProcessItem],
        leak_velocities: &HashMap<u32, f64>,
        now_ms: u64,
    ) -> Vec<ProcessLifespanProfile> {
        let dt = self.last_eval.elapsed().as_secs_f64().max(0.5);
        self.last_eval = Instant::now();

        let mut active_pids = std::collections::HashSet::new();

        for proc in processes {
            active_pids.insert(proc.pid);
            let leak_v = leak_velocities.get(&proc.pid).copied().unwrap_or(0.0);
            let is_tab = proc.proc_type_label.contains("Tab") || proc.display_title != proc.name;

            let profile = self.profiles.entry(proc.pid).or_insert_with(|| {
                ProcessLifespanProfile {
                    pid: proc.pid,
                    name: proc.name.clone(),
                    display_title: proc.display_title.clone(),
                    is_tab,
                    first_seen_ms: now_ms,
                    dwell_duration_secs: 0.0,
                    peak_working_set_bytes: proc.working_set_bytes,
                    current_working_set_bytes: proc.working_set_bytes,
                    cumulative_cpu_activity: 0.0,
                    cluster: BehaviorCluster::StandardApp,
                    recommendation: None,
                }
            });

            // Update lifespan tracking
            profile.dwell_duration_secs += dt;
            profile.current_working_set_bytes = proc.working_set_bytes;
            profile.peak_working_set_bytes = profile.peak_working_set_bytes.max(proc.working_set_bytes);
            profile.cumulative_cpu_activity += proc.cpu_percent as f64 * dt;
            profile.display_title = proc.display_title.clone();

            // Cluster Classification Heuristics
            let is_idle_cpu = proc.cpu_percent < 0.2;
            let dwell_mins = profile.dwell_duration_secs / 60.0;

            if leak_v > 15.0 && dwell_mins >= 0.5 {
                profile.cluster = BehaviorCluster::MemoryLeakSuspect;
                profile.recommendation = Some(format!(
                    "Memory leak suspected (+{:.1} MB/min). Recommend inspecting or restarting process.",
                    leak_v
                ));
            } else if is_tab {
                if is_idle_cpu && dwell_mins > 3.0 && proc.working_set_bytes > 200 * 1024 * 1024 {
                    profile.cluster = BehaviorCluster::StaleBackgroundTab;
                    profile.recommendation = Some(format!(
                        "Stale idle tab ({:.0}m inactive, {:.0}MB RAM). Hibernate or trim working set to reclaim memory.",
                        dwell_mins,
                        proc.working_set_bytes as f64 / (1024.0 * 1024.0)
                    ));
                } else if proc.cpu_percent > 1.5 {
                    profile.cluster = BehaviorCluster::ActiveTaskFocus;
                    profile.recommendation = None;
                } else {
                    profile.cluster = BehaviorCluster::StandardApp;
                    profile.recommendation = None;
                }
            } else if proc.name.to_lowercase().contains("service")
                || proc.name.to_lowercase().contains("svchost")
            {
                profile.cluster = BehaviorCluster::EssentialSystemService;
                profile.recommendation = None;
            } else if is_idle_cpu && dwell_mins > 5.0 && proc.working_set_bytes > 400 * 1024 * 1024 {
                profile.cluster = BehaviorCluster::StaleBackgroundTab;
                profile.recommendation = Some(
                    "Background app idle with heavy working set. Trim working set?".to_string(),
                );
            } else if proc.cpu_percent > 2.0 {
                profile.cluster = BehaviorCluster::ActiveTaskFocus;
                profile.recommendation = None;
            } else {
                profile.cluster = BehaviorCluster::StandardApp;
                profile.recommendation = None;
            }
        }

        // Drop inactive PIDs
        self.profiles.retain(|pid, _| active_pids.contains(pid));

        self.profiles.values().cloned().collect()
    }

    /// Forecasts Out-Of-Memory conditions based on commit rate
    pub fn forecast_oom(
        commit_total_bytes: u64,
        commit_limit_bytes: u64,
        commit_velocity_mb_min: f64,
    ) -> OomForecast {
        let commit_pct = if commit_limit_bytes > 0 {
            (commit_total_bytes as f64 / commit_limit_bytes as f64) * 100.0
        } else {
            0.0
        };

        let seconds_to_oom = if commit_velocity_mb_min > 5.0 && commit_limit_bytes > commit_total_bytes {
            let remaining_mb = (commit_limit_bytes - commit_total_bytes) as f64 / (1024.0 * 1024.0);
            let mins = remaining_mb / commit_velocity_mb_min;
            Some(mins * 60.0)
        } else {
            None
        };

        let risk_level = if commit_pct > 92.0 || seconds_to_oom.map(|s| s < 300.0).unwrap_or(false) {
            "CRITICAL"
        } else if commit_pct > 80.0 || seconds_to_oom.map(|s| s < 900.0).unwrap_or(false) {
            "WARNING"
        } else {
            "HEALTHY"
        };

        OomForecast {
            current_commit_pct: commit_pct,
            velocity_mb_min: commit_velocity_mb_min,
            estimated_seconds_to_oom: seconds_to_oom,
            risk_level,
        }
    }
}
