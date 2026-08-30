//! Executive Function & Focus Copilot
//!
//! Transforms low-level system monitoring and Chromium tab inspection into an empathetic,
//! proactive workflow completion assistant that uses non-intrusive Windows toast notifications,
//! smart triage workflows, and instant session parking (Ctrl+Shift+F).

pub mod anchor;
pub mod clipboard;
pub mod drift;
pub mod exocortex;
pub mod fixation;
pub mod flow_mode;
pub mod stashing;

#[allow(unused_imports)]
pub use anchor::{load_latest_anchor, save_latest_anchor, ResumeAnchor};
#[allow(unused_imports)]
pub use clipboard::{
    global_clipboard, spawn_clipboard_listener, ClipboardCategory, ClipboardEvent, FrustrationAlert,
};
#[allow(unused_imports)]
pub use drift::{ContextDriftDetector, DriftAssessment};
#[allow(unused_imports)]
pub use exocortex::{CognitiveState, ExocortexCoordinator, ExocortexInput, ExocortexSnapshot};
#[allow(unused_imports)]
pub use fixation::{
    global_fixation, BuildFixationAlert, FixationDetector, RabbitHoleAlert,
};
#[allow(unused_imports)]
pub use flow_mode::{FlowModeEngine, FlowSession, FlowBreakSummary};
#[allow(unused_imports)]
pub use stashing::{
    park_current_context, restore_session, spawn_global_hotkey_listener, StashManager,
};

use std::path::Path;
use std::time::{Duration, Instant};
use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;

use crate::chromium::focus_chromium_tab;
use crate::notifications::badge::global_badge;
use crate::notifications::toast::{send_copilot_toast, snooze_alert};
use crate::proc::kill_process;
use crate::sampler::SystemSnapshot;

/// Engine coordinating task sprawl detection, notification dispatch, and triage flows
pub struct CopilotEngine {
    detector: ContextDriftDetector,
    fixation: FixationDetector,
    last_assessment: Option<DriftAssessment>,
    last_toast_sent: Option<Instant>,
    min_toast_interval: Duration,
}

impl CopilotEngine {
    pub fn new(working_memory_cap: usize) -> Self {
        Self {
            detector: ContextDriftDetector::new(working_memory_cap),
            fixation: FixationDetector::new(Path::new("memoryblast_data")),
            last_assessment: None,
            last_toast_sent: None,
            min_toast_interval: Duration::from_secs(180), // 3 minutes between automatic alerts
        }
    }

    /// Evaluates a newly arrived system snapshot and updates telemetry and alerts
    pub fn evaluate_snapshot(&mut self, snapshot: &SystemSnapshot) -> DriftAssessment {
        // Sync dynamic working memory cap from adaptive engine
        if let Ok(adapt) = crate::knowledge::global_adaptive().lock() {
            self.detector.set_working_memory_cap(adapt.get_profile().dynamic_working_memory_cap);
        }

        // Detect current foreground window PID
        let fg_pid = get_current_foreground_pid();

        let assessment = self.detector.evaluate(&snapshot.processes, fg_pid);

        // Check for rabbit hole divergence
        let rabbit_alert = self.fixation.evaluate_tabs(&snapshot.processes);

        // Update badge telemetry
        let badge = global_badge();
        badge.update(|s| {
            s.active_tabs_count = assessment.active_tab_count;
            s.working_memory_cap = assessment.working_memory_cap;
            s.context_drift_index = assessment.context_drift_index;
            s.drift_severity = assessment.drift_severity;
            s.pending_alerts_count = assessment.forgotten_loops.len();
        });

        // Log context switch event to longitudinal consolidator
        if let Ok(cons) = crate::knowledge::global_consolidator().lock() {
            cons.log_context_switch(assessment.switches_per_min, assessment.active_tab_count);
        }

        // Check if we should surface an empathetic toast notification
        let now = Instant::now();
        let can_toast = self
            .last_toast_sent
            .map(|t| now.duration_since(t) >= self.min_toast_interval)
            .unwrap_or(true);

        if can_toast {
            if let Some(ref r_alert) = rabbit_alert {
                if let Ok(sent) = send_copilot_toast(
                    "Curiosity Drawer Opportunity 🐇",
                    &r_alert.suggested_prompt,
                    0,
                    &r_alert.suggested_drawer_name,
                    false,
                ) {
                    if sent {
                        self.last_toast_sent = Some(now);
                    }
                }
            } else if let Some(loop_item) = assessment.forgotten_loops.first() {
                let title = format!("Task Follow-Up: {}", loop_item.browser_family);
                let message = &loop_item.suggested_prompt;

                if let Ok(sent) = send_copilot_toast(
                    &title,
                    message,
                    loop_item.pid,
                    &loop_item.title,
                    false,
                ) {
                    if sent {
                        self.last_toast_sent = Some(now);
                    }
                }
            } else if let Some(ref cap_msg) = assessment.working_set_alert {
                if assessment.active_tab_count >= assessment.working_memory_cap {
                    if let Ok(sent) = send_copilot_toast(
                        "Cognitive Working Set Alert",
                        cap_msg,
                        0,
                        "",
                        false,
                    ) {
                        if sent {
                            self.last_toast_sent = Some(now);
                        }
                    }
                }
            }
        }

        self.last_assessment = Some(assessment.clone());
        assessment
    }

    pub fn latest_assessment(&self) -> Option<&DriftAssessment> {
        self.last_assessment.as_ref()
    }
}

/// Helper to get current foreground process ID via Win32
pub fn get_current_foreground_pid() -> Option<u32> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid != 0 {
            Some(pid)
        } else {
            None
        }
    }
}

/// Executes one-click triage action initiated from interactive toast, GUI, or TUI
pub async fn handle_triage_action(
    action: &str,
    pid: u32,
    tab_title: &str,
    debug_port: Option<u16>,
) -> Result<String, String> {
    match action {
        "focus" => {
            focus_chromium_tab(pid, 0, tab_title, debug_port).await?;
            let badge = global_badge();
            badge.update(|s| {
                s.last_action_message = Some(format!("Focused on '{tab_title}'"));
            });
            Ok(format!("Focused window for '{tab_title}'"))
        }
        "stash" => {
            // Terminate or trim tab and record closure
            let _ = kill_process(pid);
            let badge = global_badge();
            badge.record_stash(1, 150.0);
            snooze_alert(tab_title, 1440); // 24 hours snooze after stash
            Ok(format!("Stashed and closed '{tab_title}'"))
        }
        "snooze" => {
            snooze_alert(tab_title, 30);
            let badge = global_badge();
            badge.update(|s| {
                s.last_action_message = Some(format!("Snoozed alerts for '{tab_title}' (30 mins)"));
            });
            Ok(format!("Snoozed '{tab_title}' for 30 minutes"))
        }
        "park" => {
            let session = park_current_context(&[], &[]).await.map_err(|e| e.to_string())?;
            Ok(format!("Parked context: {} tabs saved", session.tab_count))
        }
        "rubber_duck" => {
            let prompt = format!(
                "🦆 Rubber Duck Debugging Session:\n- Problematic element: {}\n- Hypothesis: Break this into 2 atomic sub-steps.",
                tab_title
            );
            Ok(prompt)
        }
        _ => Err(format!("Unknown action '{action}'")),
    }
}
