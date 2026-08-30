//! Zen / Flow Mode Engine
//!
//! Detects sustained single-project coding sessions (>45 min without context switch)
//! and suppresses non-critical notifications during deep focus periods.
//! On flow break, emits a gentle summary of the uninterrupted session duration.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Minimum sustained focus duration before entering Zen mode
const FLOW_ENTRY_THRESHOLD_SECS: u64 = 45 * 60; // 45 minutes

/// Maximum context switches per minute allowed while maintaining flow state
const FLOW_MAX_SWITCHES_PER_MIN: f64 = 0.5;

/// Maximum tab count growth allowed during flow (opening 3+ new tabs breaks flow)
const FLOW_MAX_TAB_GROWTH: usize = 3;

/// Current state of a Zen / Flow session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowSession {
    pub started_epoch: u64,
    pub duration_secs: f64,
    pub peak_focus_streak_secs: f64,
    pub tabs_at_entry: usize,
    pub suppressed_alerts_count: usize,
}

/// Summary emitted when a flow session ends
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowBreakSummary {
    pub session_duration_mins: f64,
    pub suppressed_alerts: usize,
    pub message: String,
}

/// Internal tracking state for the flow detector
#[derive(Debug, Clone)]
struct FlowTracker {
    /// When the current sustained focus streak started
    focus_streak_start: Instant,
    /// Tab count when the streak began
    tabs_at_streak_start: usize,
    /// Whether we're currently in Zen mode
    in_flow: bool,
    /// When flow mode was formally entered
    flow_entered_at: Option<Instant>,
    /// How many alerts were suppressed during this flow session
    suppressed_count: usize,
    /// Last known switches-per-minute reading
    last_switches_per_min: f64,
    /// Last known tab count
    last_tab_count: usize,
}

/// Flow Mode Engine detecting and managing Zen state
pub struct FlowModeEngine {
    tracker: FlowTracker,
    /// Accumulated completed flow sessions for daily reporting
    completed_sessions: Vec<FlowBreakSummary>,
}

impl FlowModeEngine {
    pub fn new() -> Self {
        Self {
            tracker: FlowTracker {
                focus_streak_start: Instant::now(),
                tabs_at_streak_start: 0,
                in_flow: false,
                flow_entered_at: None,
                suppressed_count: 0,
                last_switches_per_min: 0.0,
                last_tab_count: 0,
            },
            completed_sessions: Vec::new(),
        }
    }

    /// Updates the flow state based on the latest drift assessment metrics.
    /// Returns `true` if currently in Zen / Flow mode.
    pub fn update(
        &mut self,
        switches_per_min: f64,
        active_tab_count: usize,
        drift_index: f64,
    ) -> bool {
        let now = Instant::now();
        let t = &mut self.tracker;

        // Detect flow disruption: high switch rate, large tab growth, or high drift
        let tab_growth = if active_tab_count > t.tabs_at_streak_start {
            active_tab_count - t.tabs_at_streak_start
        } else {
            0
        };

        let is_disrupted = switches_per_min > FLOW_MAX_SWITCHES_PER_MIN
            || tab_growth > FLOW_MAX_TAB_GROWTH
            || drift_index > 7.0;

        if is_disrupted {
            // If we were in flow, end the session
            if t.in_flow {
                let duration = t.flow_entered_at.unwrap().elapsed().as_secs_f64();
                let summary = FlowBreakSummary {
                    session_duration_mins: (duration / 60.0 * 10.0).round() / 10.0,
                    suppressed_alerts: t.suppressed_count,
                    message: format!(
                        "🎯 Flow session complete: {:.0} minutes of uninterrupted focus. {} alerts suppressed.",
                        duration / 60.0,
                        t.suppressed_count,
                    ),
                };
                self.completed_sessions.push(summary);

                t.in_flow = false;
                t.flow_entered_at = None;
                t.suppressed_count = 0;
            }

            // Reset streak
            t.focus_streak_start = now;
            t.tabs_at_streak_start = active_tab_count;
        } else if !t.in_flow {
            // Check if we've sustained focus long enough to enter flow
            let streak_duration = now.duration_since(t.focus_streak_start);
            if streak_duration >= Duration::from_secs(FLOW_ENTRY_THRESHOLD_SECS) {
                t.in_flow = true;
                t.flow_entered_at = Some(t.focus_streak_start);
                t.suppressed_count = 0;
            }
        }

        t.last_switches_per_min = switches_per_min;
        t.last_tab_count = active_tab_count;
        t.in_flow
    }

    /// Whether we're currently in Zen / Flow mode
    pub fn is_in_flow(&self) -> bool {
        self.tracker.in_flow
    }

    /// Records that an alert was suppressed during flow
    pub fn record_suppressed_alert(&mut self) {
        if self.tracker.in_flow {
            self.tracker.suppressed_count += 1;
        }
    }

    /// Returns the active flow session details (if in flow)
    pub fn active_session(&self) -> Option<FlowSession> {
        if !self.tracker.in_flow {
            return None;
        }
        let entered = self.tracker.flow_entered_at?;
        let duration = entered.elapsed().as_secs_f64();
        let now_epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let started_epoch = now_epoch.saturating_sub(duration as u64);

        Some(FlowSession {
            started_epoch,
            duration_secs: duration,
            peak_focus_streak_secs: duration,
            tabs_at_entry: self.tracker.tabs_at_streak_start,
            suppressed_alerts_count: self.tracker.suppressed_count,
        })
    }

    /// Returns and drains completed flow session summaries (for daily closure reports)
    pub fn drain_completed_sessions(&mut self) -> Vec<FlowBreakSummary> {
        std::mem::take(&mut self.completed_sessions)
    }

    /// Total flow minutes accumulated today (from completed sessions + current)
    pub fn total_flow_minutes_today(&self) -> f64 {
        let completed: f64 = self.completed_sessions.iter().map(|s| s.session_duration_mins).sum();
        let current = if let Some(entered) = self.tracker.flow_entered_at {
            entered.elapsed().as_secs_f64() / 60.0
        } else {
            0.0
        };
        completed + current
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_flow_mode_not_entered_on_high_switches() {
        let mut engine = FlowModeEngine::new();

        // High switch rate should prevent flow entry
        for _ in 0..100 {
            let result = engine.update(3.0, 15, 2.0);
            assert!(!result, "Should not enter flow with high switch rate");
        }
        assert!(!engine.is_in_flow());
    }

    #[test]
    fn test_flow_session_summary_on_disruption() {
        let mut engine = FlowModeEngine::new();

        // Manually set flow state for testing
        engine.tracker.in_flow = true;
        engine.tracker.flow_entered_at = Some(Instant::now() - Duration::from_secs(3000)); // 50 min ago
        engine.tracker.suppressed_count = 5;
        engine.tracker.tabs_at_streak_start = 8;

        // Disrupt flow
        let result = engine.update(5.0, 20, 8.0);
        assert!(!result, "Flow should be broken by disruption");
        assert!(!engine.is_in_flow());

        let summaries = engine.drain_completed_sessions();
        assert_eq!(summaries.len(), 1);
        assert!(summaries[0].session_duration_mins >= 49.0);
        assert_eq!(summaries[0].suppressed_alerts, 5);
    }
}
