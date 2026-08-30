//! Startup Orientation & Executive Briefing Generator
//!
//! Synthesizes historical session rollups, learned working archetypes, and
//! parked session backlogs into a high-signal, executive "Where You Left Off" briefing:
//! "Welcome back. Yesterday you parked 6 research tabs and reclaimed 1.4 GB.
//! Your peak focus window is typically 09:30–11:30. Ready to restore your session or start fresh?"

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::knowledge::adaptive_engine::global_adaptive;
use crate::knowledge::consolidator::global_consolidator;

/// Structured Executive Orientation Briefing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutiveBriefing {
    pub greeting: String,
    pub headline: String,
    pub yesterday_summary: String,
    pub peak_focus_window: String,
    pub dynamic_working_memory_cap: usize,
    pub primary_archetype: String,
    pub total_reclaimed_yesterday_mb: f64,
    pub stashed_tabs_waiting: usize,
    pub call_to_action: String,
}

impl ExecutiveBriefing {
    /// Generates a fresh executive briefing from current historical knowledge
    pub fn generate() -> Self {
        let (yesterday_profile, _profiles_count) = {
            let cons = global_consolidator().lock().unwrap();
            let recent = cons.get_recent_profiles(3);
            let first = recent.first().cloned();
            (first, recent.len())
        };

        let adaptive_profile = {
            let mut engine = global_adaptive().lock().unwrap();
            engine.load_or_recalibrate().clone()
        };

        let (stashed_tabs_waiting, stashed_mb) = {
            let stash_res = crate::copilot::StashManager::new(Path::new("memoryblast_data"));
            if let Ok(mgr) = stash_res {
                if let Ok(sessions) = mgr.list_sessions() {
                    let active_stashes: Vec<_> = sessions.into_iter().filter(|s| !s.restored).collect();
                    let cnt = active_stashes.iter().map(|s| s.tab_count).sum();
                    let mb = active_stashes.iter().map(|s| s.total_memory_bytes as f64 / (1024.0 * 1024.0)).sum();
                    (cnt, mb)
                } else {
                    (0, 0.0)
                }
            } else {
                (0, 0.0)
            }
        };

        let greeting = greeting_by_time_of_day();
        let archetype_str = yesterday_profile
            .as_ref()
            .map(|p| p.primary_archetype.as_str())
            .unwrap_or("Balanced Flow")
            .to_string();

        let headline = if stashed_tabs_waiting > 0 {
            format!("{} {} tabs waiting in your parked context shelf.", greeting, stashed_tabs_waiting)
        } else {
            format!("{} Ready for a high-focus flow state today.", greeting)
        };

        let yesterday_summary = if let Some(ref y) = yesterday_profile {
            format!(
                "In your last session, you logged {:.0}m active focus ({}) and reclaimed {:.1} MB RAM.",
                y.total_focus_mins, y.primary_archetype.as_str(), y.reclaimed_memory_mb
            )
        } else {
            "First session of the day. MemoryBlast is continuously modeling your working rhythm.".to_string()
        };

        let call_to_action = if stashed_tabs_waiting > 0 {
            "Ready to restore your parked session or start with a clean slate?".to_string()
        } else {
            "Clean working set. MemoryBlast Focus Copilot is active in the background.".to_string()
        };

        Self {
            greeting,
            headline,
            yesterday_summary,
            peak_focus_window: adaptive_profile.peak_focus_window,
            dynamic_working_memory_cap: adaptive_profile.dynamic_working_memory_cap,
            primary_archetype: archetype_str,
            total_reclaimed_yesterday_mb: stashed_mb,
            stashed_tabs_waiting,
            call_to_action,
        }
    }

    /// Formats a clean ASCII executive card for terminal display
    pub fn render_terminal_card(&self) -> String {
        let border = "═".repeat(68);
        let mut out = String::new();
        out.push_str(&format!("╔{}╗\n", border));
        out.push_str("║ ☀️ MEMORYBLAST EXECUTIVE ORIENTATION BRIEFING                     ║\n");
        out.push_str(&format!("╠{}╣\n", border));
        out.push_str(&format!("║ {:<66} ║\n", truncate_str(&self.headline, 66)));
        out.push_str(&format!("║ {:<66} ║\n", truncate_str(&self.yesterday_summary, 66)));
        out.push_str(&format!("║ Peak Focus Window: {:<46} ║\n", &self.peak_focus_window));
        out.push_str(&format!("║ Adaptive Working Memory Cap: {:<36} ║\n", format!("{} tabs (personalized)", self.dynamic_working_memory_cap)));
        out.push_str(&format!("║ {:<66} ║\n", truncate_str(&self.call_to_action, 66)));
        out.push_str(&format!("╚{}╝\n", border));
        out
    }
}

fn greeting_by_time_of_day() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let hour = (secs / 3600) % 24;

    if hour < 12 {
        "Good morning!".to_string()
    } else if hour < 18 {
        "Good afternoon!".to_string()
    } else {
        "Good evening!".to_string()
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() > max_len {
        let truncated: String = s.chars().take(max_len.saturating_sub(3)).collect();
        format!("{}...", truncated)
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executive_briefing_generation() {
        let brief = ExecutiveBriefing::generate();
        assert!(!brief.greeting.is_empty());
        assert!(brief.dynamic_working_memory_cap >= 8);

        let card = brief.render_terminal_card();
        assert!(card.contains("EXECUTIVE ORIENTATION BRIEFING"));
    }
}
