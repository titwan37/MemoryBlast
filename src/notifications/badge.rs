//! Copilot Status & Tray Badge State Tracker
//!
//! Maintains high-level status, alert counts, drift metrics, and closure counters
//! for non-intrusive feedback across GUI, system tray, and terminal TUI.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopilotStatus {
    pub is_enabled: bool,
    pub active_tabs_count: usize,
    pub working_memory_cap: usize,
    pub context_drift_index: f64,
    pub drift_severity: &'static str,
    pub pending_alerts_count: usize,
    pub total_tabs_stashed: usize,
    pub total_memory_reclaimed_mb: f64,
    pub last_action_message: Option<String>,
}

impl Default for CopilotStatus {
    fn default() -> Self {
        Self {
            is_enabled: true,
            active_tabs_count: 0,
            working_memory_cap: 12,
            context_drift_index: 0.0,
            drift_severity: "Calm",
            pending_alerts_count: 0,
            total_tabs_stashed: 0,
            total_memory_reclaimed_mb: 0.0,
            last_action_message: None,
        }
    }
}

pub struct CopilotBadgeState {
    status: Arc<RwLock<CopilotStatus>>,
}

impl CopilotBadgeState {
    pub fn new() -> Self {
        Self {
            status: Arc::new(RwLock::new(CopilotStatus::default())),
        }
    }

    pub fn get_status(&self) -> CopilotStatus {
        self.status.read().unwrap().clone()
    }

    pub fn update<F>(&self, mutator: F)
    where
        F: FnOnce(&mut CopilotStatus),
    {
        if let Ok(mut lock) = self.status.write() {
            mutator(&mut *lock);
        }
    }

    pub fn record_stash(&self, tab_count: usize, reclaimed_mb: f64) {
        self.update(|s| {
            s.total_tabs_stashed += tab_count;
            s.total_memory_reclaimed_mb += reclaimed_mb;
            s.last_action_message = Some(format!(
                "Parked {tab_count} tabs, freed {reclaimed_mb:.1} MB RAM"
            ));
        });
    }
}

static BADGE_INSTANCE: OnceLock<Arc<CopilotBadgeState>> = OnceLock::new();

pub fn global_badge() -> Arc<CopilotBadgeState> {
    BADGE_INSTANCE
        .get_or_init(|| Arc::new(CopilotBadgeState::new()))
        .clone()
}
