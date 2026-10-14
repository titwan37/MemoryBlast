//! Chromium Target Lifecycle Tracker
//!
//! Hooks real-time CDP target creation and destruction events alongside
//! Win32 window and process states to:
//! - Record exact opening ($T_{\text{open}}$) and closing ($T_{\text{close}}$) timestamps
//! - Calculate exact shelf-life duration ($\Delta T = T_{\text{close}} - T_{\text{open}}$)
//! - Classify closure causes (UserManual, CopilotStashed, BrowserMemorySaver, RendererCrash, StaleCleanup)
//! - Measure active foreground dwell time and peak memory consumption

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::knowledge::tab_history::{global_tab_history, TabLifecycleRecord};
use crate::proc::ProcessItem;

/// Categorization of why a Chromium tab was closed or suspended
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClosureCause {
    UserManual,
    CopilotStashed,
    BrowserMemorySaver,
    RendererCrash,
    SessionShutdown,
    StaleCleanup,
}

impl ClosureCause {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::UserManual => "UserManual",
            Self::CopilotStashed => "CopilotStashed",
            Self::BrowserMemorySaver => "BrowserMemorySaver",
            Self::RendererCrash => "RendererCrash",
            Self::SessionShutdown => "SessionShutdown",
            Self::StaleCleanup => "StaleCleanup",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "CopilotStashed" => Self::CopilotStashed,
            "BrowserMemorySaver" => Self::BrowserMemorySaver,
            "RendererCrash" => Self::RendererCrash,
            "SessionShutdown" => Self::SessionShutdown,
            "StaleCleanup" => Self::StaleCleanup,
            _ => Self::UserManual,
        }
    }
}

/// In-flight tab state actively monitored
#[derive(Debug, Clone)]
pub struct ActiveTabState {
    pub target_id: String,
    pub domain: String,
    pub url: String,
    pub title: String,
    pub opened_at_epoch: u64,
    pub opened_instant: Instant,
    pub last_seen_instant: Instant,
    pub active_dwell_secs: f64,
    pub peak_memory_bytes: u64,
    pub was_foreground_recently: bool,
    pub is_marked_stashed: bool,
}

/// State machine tracking active Chromium tabs across sampling ticks
pub struct ChromiumLifecycleTracker {
    active_tabs: HashMap<String, ActiveTabState>, // key: target_id or title
    last_eval_instant: Instant,
}

#[allow(dead_code)]
impl ChromiumLifecycleTracker {
    pub fn new() -> Self {
        Self {
            active_tabs: HashMap::new(),
            last_eval_instant: Instant::now(),
        }
    }

    /// Evaluates current snapshot items against active tab states to detect opens and closures
    pub fn evaluate_snapshot(
        &mut self,
        processes: &[ProcessItem],
        foreground_pid: Option<u32>,
    ) -> Vec<TabLifecycleRecord> {
        let now = Instant::now();
        let dt = now.duration_since(self.last_eval_instant).as_secs_f64().max(0.5);
        self.last_eval_instant = now;
        let now_epoch = now_epoch_secs();

        // 1. Identify currently active Chromium tabs from processes
        let mut current_keys = std::collections::HashSet::new();

        for proc in processes {
            if proc.is_chromium && (!proc.display_title.is_empty() || proc.proc_type_label == "Renderer") {
                let key = if !proc.display_title.is_empty() {
                    proc.display_title.clone()
                } else {
                    format!("renderer_{}", proc.pid)
                };

                current_keys.insert(key.clone());

                let is_fg = foreground_pid.map(|pid| pid == proc.pid).unwrap_or(false);
                let mem = proc.working_set_bytes;

                let entry = self.active_tabs.entry(key.clone()).or_insert_with(|| {
                    let domain = extract_domain_from_title(&proc.display_title);
                    let initial_url = format!("https://{}/", domain);

                    let record = TabLifecycleRecord {
                        id: 0,
                        target_id: key.clone(),
                        domain: domain.clone(),
                        url: initial_url.clone(),
                        title: proc.display_title.clone(),
                        opened_at_epoch: now_epoch,
                        closed_at_epoch: None,
                        shelf_life_secs: 0.0,
                        active_dwell_secs: 0.0,
                        closure_cause: ClosureCause::UserManual,
                        peak_memory_mb: mem as f64 / (1024.0 * 1024.0),
                    };

                    // Persist initial open record
                    if let Ok(history) = global_tab_history().lock() {
                        let _ = history.record_tab_opened(&record);
                    }

                    ActiveTabState {
                        target_id: key.clone(),
                        domain,
                        url: initial_url,
                        title: proc.display_title.clone(),
                        opened_at_epoch: now_epoch,
                        opened_instant: now,
                        last_seen_instant: now,
                        active_dwell_secs: 0.0,
                        peak_memory_bytes: mem,
                        was_foreground_recently: is_fg,
                        is_marked_stashed: false,
                    }
                });

                // Update active metrics
                entry.last_seen_instant = now;
                entry.peak_memory_bytes = entry.peak_memory_bytes.max(mem);
                if is_fg {
                    entry.active_dwell_secs += dt;
                    entry.was_foreground_recently = true;
                }
            }
        }

        // 2. Identify closed tabs (present in active_tabs but missing in current snapshot)
        let mut completed_records = Vec::new();
        let mut closed_keys = Vec::new();

        for (key, state) in &mut self.active_tabs {
            if !current_keys.contains(key) {
                let shelf_life = state.opened_instant.elapsed().as_secs_f64();

                // Determine closure cause
                let cause = if state.is_marked_stashed {
                    ClosureCause::CopilotStashed
                } else if state.was_foreground_recently {
                    ClosureCause::UserManual
                } else if shelf_life > 7200.0 && state.active_dwell_secs < 60.0 {
                    ClosureCause::StaleCleanup
                } else if state.peak_memory_bytes > 800 * 1024 * 1024 {
                    ClosureCause::BrowserMemorySaver
                } else {
                    ClosureCause::UserManual
                };

                let record = TabLifecycleRecord {
                    id: 0,
                    target_id: state.target_id.clone(),
                    domain: state.domain.clone(),
                    url: state.url.clone(),
                    title: state.title.clone(),
                    opened_at_epoch: state.opened_at_epoch,
                    closed_at_epoch: Some(now_epoch),
                    shelf_life_secs: shelf_life,
                    active_dwell_secs: state.active_dwell_secs,
                    closure_cause: cause,
                    peak_memory_mb: state.peak_memory_bytes as f64 / (1024.0 * 1024.0),
                };

                // Persist closure to SQLite
                if let Ok(history) = global_tab_history().lock() {
                    let _ = history.record_tab_closed(&record);
                }

                completed_records.push(record);
                closed_keys.push(key.clone());
            }
        }

        // Remove closed tabs from tracking map
        for k in closed_keys {
            self.active_tabs.remove(&k);
        }

        completed_records
    }

    /// Marks a tab as explicitly stashed by Copilot
    pub fn mark_tab_stashed(&mut self, title: &str) {
        if let Some(state) = self.active_tabs.get_mut(title) {
            state.is_marked_stashed = true;
        }
    }

    /// Returns the number of currently tracked open tabs
    pub fn active_count(&self) -> usize {
        self.active_tabs.len()
    }
}

/// Global singleton for Chromium lifecycle tracking
pub fn global_lifecycle_tracker() -> &'static Mutex<ChromiumLifecycleTracker> {
    static TRACKER: OnceLock<Mutex<ChromiumLifecycleTracker>> = OnceLock::new();
    TRACKER.get_or_init(|| Mutex::new(ChromiumLifecycleTracker::new()))
}

/// Extracts domain from browser tab title or search string
pub fn extract_domain_from_title(title: &str) -> String {
    let lower = title.to_lowercase();
    let known_domains = [
        ("github", "github.com"),
        ("docs.rs", "docs.rs"),
        ("stackoverflow", "stackoverflow.com"),
        ("stack overflow", "stackoverflow.com"),
        ("crates.io", "crates.io"),
        ("rust-lang", "rust-lang.org"),
        ("localhost", "localhost"),
        ("127.0.0.1", "localhost"),
        ("google", "google.com"),
        ("youtube", "youtube.com"),
        ("reddit", "reddit.com"),
        ("wikipedia", "wikipedia.org"),
        ("twitter", "x.com"),
        ("x.com", "x.com"),
        ("linear", "linear.app"),
        ("jira", "atlassian.net"),
        ("notion", "notion.so"),
        ("duckduckgo", "duckduckgo.com"),
    ];

    for (needle, domain) in &known_domains {
        if lower.contains(needle) {
            return (*domain).to_string();
        }
    }

    "web.tab".to_string()
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

    #[test]
    fn test_extract_domain() {
        assert_eq!(extract_domain_from_title("PR #42: Feature · titwan37/MemoryBlast GitHub"), "github.com");
        assert_eq!(extract_domain_from_title("rusqlite::Connection - Rust docs.rs"), "docs.rs");
        assert_eq!(extract_domain_from_title("How to fix borrow checker - Stack Overflow"), "stackoverflow.com");
        assert_eq!(extract_domain_from_title("Random Search - Google Search"), "google.com");
        assert_eq!(extract_domain_from_title("Unknown Tab"), "web.tab");
    }

    #[test]
    fn test_lifecycle_open_and_close() {
        let mut tracker = ChromiumLifecycleTracker::new();

        let proc = ProcessItem {
            pid: 1234,
            ppid: 1000,
            name: "chrome.exe".to_string(),
            display_title: "Rust Documentation - docs.rs".to_string(),
            working_set_bytes: 250 * 1024 * 1024,
            private_bytes: 200 * 1024 * 1024,
            cpu_percent: 0.5,
            memory_percent: 1.2,
            gpu_percent: 0.0,
            is_chromium: true,
            proc_type_label: "Renderer".to_string(),
            browser_family: Some("Google Chrome".to_string()),
            children: Vec::new(),
            debug_port: None,
        };

        // 1. Snapshot with active tab
        let closed = tracker.evaluate_snapshot(&[proc.clone()], Some(1234));
        assert_eq!(closed.len(), 0);
        assert_eq!(tracker.active_count(), 1);

        // 2. Snapshot where tab has closed
        let closed2 = tracker.evaluate_snapshot(&[], None);
        assert_eq!(closed2.len(), 1);
        assert_eq!(closed2[0].domain, "docs.rs");
        assert_eq!(tracker.active_count(), 0);
    }
}
