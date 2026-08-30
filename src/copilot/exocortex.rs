//! Exocortex Coordinator — Unified Cognitive State Machine
//!
//! Central orchestration hub that synthesizes low-level telemetry signals
//! (drift index, clipboard frustration, memory pressure, flow state, Explorer activity)
//! into a single unified `CognitiveState` each telemetry tick.
//!
//! The cognitive state drives:
//! - GUI theme transitions (Calm → DeepFocus → Drifting → Overloaded)
//! - Zen Mode alert suppression during DeepFocus
//! - Evening closure report generation
//! - Adaptive threshold recalibration

use serde::{Deserialize, Serialize};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::copilot::flow_mode::{FlowModeEngine, FlowSession, FlowBreakSummary};
use crate::copilot::drift::DriftAssessment;

/// The four neuro-digital cognitive states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CognitiveState {
    /// Low activity, few tabs, low drift — the restful baseline
    Calm,
    /// Single-project sustained coding (>45 min uninterrupted)
    DeepFocus,
    /// High context switches, growing tab sprawl, but manageable
    Drifting,
    /// Memory pressure + high drift + forgotten loops — red alert territory
    Overloaded,
}

impl CognitiveState {
    /// Returns the CSS theme class name for the GUI
    pub fn css_class(&self) -> &'static str {
        match self {
            CognitiveState::Calm => "cognitive-calm",
            CognitiveState::DeepFocus => "cognitive-focus",
            CognitiveState::Drifting => "cognitive-drifting",
            CognitiveState::Overloaded => "cognitive-overloaded",
        }
    }

    /// Returns a human-readable label
    pub fn label(&self) -> &'static str {
        match self {
            CognitiveState::Calm => "Calm",
            CognitiveState::DeepFocus => "Deep Focus 🧘",
            CognitiveState::Drifting => "Drifting ⚡",
            CognitiveState::Overloaded => "Overloaded 🔴",
        }
    }

    /// Returns the accent color (hex) for this state
    pub fn accent_color(&self) -> &'static str {
        match self {
            CognitiveState::Calm => "#60cdff",
            CognitiveState::DeepFocus => "#7c3aed",
            CognitiveState::Drifting => "#f59e0b",
            CognitiveState::Overloaded => "#ef4444",
        }
    }
}

/// Snapshot of the Exocortex state emitted to the GUI each tick
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExocortexSnapshot {
    pub cognitive_state: CognitiveState,
    pub css_class: String,
    pub state_label: String,
    pub accent_color: String,
    pub flow_session: Option<FlowSession>,
    pub flow_minutes_today: f64,
    pub recent_flow_summaries: Vec<FlowBreakSummary>,
    pub is_zen_mode: bool,
    pub suppressed_alerts_count: usize,
    pub state_duration_secs: f64,
}

/// Input signals consumed each tick by the coordinator
pub struct ExocortexInput {
    pub drift_index: f64,
    pub drift_severity: &'static str,
    pub active_tab_count: usize,
    pub switches_per_min: f64,
    pub forgotten_loop_count: usize,
    pub memory_pressure_pct: f64,
    pub clipboard_frustration: bool,
}

impl ExocortexInput {
    /// Constructs input from a DriftAssessment and supplemental metrics
    pub fn from_assessment(
        assessment: &DriftAssessment,
        memory_pressure_pct: f64,
        clipboard_frustration: bool,
    ) -> Self {
        Self {
            drift_index: assessment.context_drift_index,
            drift_severity: assessment.drift_severity,
            active_tab_count: assessment.active_tab_count,
            switches_per_min: assessment.switches_per_min,
            forgotten_loop_count: assessment.forgotten_loops.len(),
            memory_pressure_pct,
            clipboard_frustration,
        }
    }
}

/// The Exocortex Coordinator — central cognitive state machine
pub struct ExocortexCoordinator {
    state: CognitiveState,
    flow_engine: FlowModeEngine,
    last_state_transition: Instant,
    state_history: Vec<(CognitiveState, u64)>, // (state, epoch)
}

impl ExocortexCoordinator {
    pub fn new() -> Self {
        Self {
            state: CognitiveState::Calm,
            flow_engine: FlowModeEngine::new(),
            last_state_transition: Instant::now(),
            state_history: Vec::new(),
        }
    }

    /// Main tick: evaluates all signals and produces a unified cognitive state snapshot
    pub fn evaluate(&mut self, input: &ExocortexInput) -> ExocortexSnapshot {
        // 1. Update flow mode engine
        let in_flow = self.flow_engine.update(
            input.switches_per_min,
            input.active_tab_count,
            input.drift_index,
        );

        // 2. Compute cognitive state from multi-signal fusion
        let new_state = self.compute_state(input, in_flow);

        // 3. Track state transitions
        if new_state != self.state {
            let now_epoch = now_epoch_secs();
            self.state_history.push((new_state, now_epoch));
            // Keep history bounded
            if self.state_history.len() > 100 {
                self.state_history.drain(0..50);
            }
            self.last_state_transition = Instant::now();
            self.state = new_state;
        }

        // 4. Collect flow summaries (completed sessions since last tick)
        let recent_flow_summaries = self.flow_engine.drain_completed_sessions();

        // 5. Build snapshot
        ExocortexSnapshot {
            cognitive_state: self.state,
            css_class: self.state.css_class().to_string(),
            state_label: self.state.label().to_string(),
            accent_color: self.state.accent_color().to_string(),
            flow_session: self.flow_engine.active_session(),
            flow_minutes_today: self.flow_engine.total_flow_minutes_today(),
            recent_flow_summaries,
            is_zen_mode: self.flow_engine.is_in_flow(),
            suppressed_alerts_count: self.flow_engine.active_session()
                .map(|s| s.suppressed_alerts_count)
                .unwrap_or(0),
            state_duration_secs: self.last_state_transition.elapsed().as_secs_f64(),
        }
    }

    /// Whether alerts should be suppressed (Zen mode active)
    pub fn should_suppress_alerts(&self) -> bool {
        self.flow_engine.is_in_flow()
    }

    /// Records that a non-critical alert was suppressed during flow
    pub fn record_suppressed_alert(&mut self) {
        self.flow_engine.record_suppressed_alert();
    }

    /// Current cognitive state
    pub fn current_state(&self) -> CognitiveState {
        self.state
    }

    /// Multi-signal fusion to determine cognitive state
    fn compute_state(&self, input: &ExocortexInput, in_flow: bool) -> CognitiveState {
        // Priority 1: If flow mode engine detected sustained focus, it's DeepFocus
        if in_flow {
            return CognitiveState::DeepFocus;
        }

        // Priority 2: High-severity overload detection
        // Criteria: high memory pressure + high drift + multiple forgotten loops
        let is_overloaded = (input.memory_pressure_pct > 85.0 && input.drift_index > 5.0)
            || (input.forgotten_loop_count >= 3 && input.drift_index > 6.0)
            || (input.memory_pressure_pct > 90.0 && input.forgotten_loop_count >= 2)
            || (input.clipboard_frustration && input.drift_index > 7.0);

        if is_overloaded {
            return CognitiveState::Overloaded;
        }

        // Priority 3: Drifting — moderate sprawl but not overloaded
        let is_drifting = input.drift_index > 3.5
            || input.switches_per_min > 2.0
            || input.forgotten_loop_count >= 1
            || (input.active_tab_count > 15 && input.drift_index > 2.0);

        if is_drifting {
            return CognitiveState::Drifting;
        }

        // Default: Calm
        CognitiveState::Calm
    }
}

fn now_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_input(drift: f64, switches: f64, tabs: usize, loops: usize, mem_pct: f64) -> ExocortexInput {
        ExocortexInput {
            drift_index: drift,
            drift_severity: "Calm",
            active_tab_count: tabs,
            switches_per_min: switches,
            forgotten_loop_count: loops,
            memory_pressure_pct: mem_pct,
            clipboard_frustration: false,
        }
    }

    #[test]
    fn test_calm_state() {
        let mut coord = ExocortexCoordinator::new();
        let input = make_input(1.0, 0.2, 5, 0, 40.0);
        let snap = coord.evaluate(&input);
        assert_eq!(snap.cognitive_state, CognitiveState::Calm);
        assert_eq!(snap.css_class, "cognitive-calm");
    }

    #[test]
    fn test_drifting_state() {
        let mut coord = ExocortexCoordinator::new();
        let input = make_input(4.5, 2.5, 12, 1, 60.0);
        let snap = coord.evaluate(&input);
        assert_eq!(snap.cognitive_state, CognitiveState::Drifting);
        assert_eq!(snap.css_class, "cognitive-drifting");
    }

    #[test]
    fn test_overloaded_state() {
        let mut coord = ExocortexCoordinator::new();
        let input = make_input(6.0, 3.0, 20, 3, 88.0);
        let snap = coord.evaluate(&input);
        assert_eq!(snap.cognitive_state, CognitiveState::Overloaded);
        assert_eq!(snap.css_class, "cognitive-overloaded");
    }

    #[test]
    fn test_zen_mode_not_entered_without_sustained_focus() {
        let mut coord = ExocortexCoordinator::new();
        // High switch rate should prevent flow entry
        let input = make_input(1.0, 3.0, 8, 0, 40.0);
        let snap = coord.evaluate(&input);
        assert!(!snap.is_zen_mode);
        assert!(!coord.should_suppress_alerts());
    }

    #[test]
    fn test_state_label_and_accent_color() {
        assert_eq!(CognitiveState::Calm.label(), "Calm");
        assert_eq!(CognitiveState::DeepFocus.accent_color(), "#7c3aed");
        assert_eq!(CognitiveState::Drifting.css_class(), "cognitive-drifting");
        assert_eq!(CognitiveState::Overloaded.accent_color(), "#ef4444");
    }
}
