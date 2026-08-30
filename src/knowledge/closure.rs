//! Daily Session Closure & Evening Wrap-Up Generator
//!
//! Triggered at end-of-day (or via CLI `memoryblast.exe closure`) to generate a
//! structured daily session wrap-up summarizing freed memory, resolved forgotten loops,
//! completed flow sessions, and the day's working archetype.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::copilot::flow_mode::FlowBreakSummary;
use crate::knowledge::consolidator::global_consolidator;
use crate::notifications::badge::global_badge;

/// Structured daily closure summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyClosureSummary {
    pub date_str: String,
    pub epoch: u64,
    pub total_memory_reclaimed_mb: f64,
    pub tabs_stashed_today: usize,
    pub forgotten_loops_resolved: usize,
    pub forgotten_loops_remaining: usize,
    pub flow_sessions_count: usize,
    pub total_flow_minutes: f64,
    pub primary_archetype: String,
    pub celebration_badge: String,
    pub headline: String,
}

impl DailyClosureSummary {
    /// Generates a daily closure summary from current accumulated state
    pub fn generate(flow_summaries: &[FlowBreakSummary]) -> Self {
        let now_epoch = now_epoch_secs();
        let date_str = format_date(now_epoch);

        // Gather stash metrics from badge
        let badge_status = global_badge().get_status();
        let total_memory_reclaimed_mb = badge_status.total_memory_reclaimed_mb;
        let tabs_stashed_today = badge_status.total_tabs_stashed;

        // Gather archetype from consolidator
        let primary_archetype = {
            let cons = global_consolidator().lock().unwrap();
            let recent = cons.get_recent_profiles(1);
            recent
                .first()
                .map(|p| p.primary_archetype.as_str().to_string())
                .unwrap_or_else(|| "Balanced Flow".to_string())
        };

        // Compute flow metrics
        let flow_sessions_count = flow_summaries.len();
        let total_flow_minutes: f64 = flow_summaries.iter().map(|s| s.session_duration_mins).sum();

        // Generate celebration badge based on productivity
        let celebration_badge = if total_flow_minutes >= 120.0 {
            "🏆 Deep Work Champion".to_string()
        } else if tabs_stashed_today >= 10 {
            "🧹 Context Cleanup Hero".to_string()
        } else if total_memory_reclaimed_mb >= 1024.0 {
            "💪 Memory Liberation Expert".to_string()
        } else if total_flow_minutes >= 45.0 {
            "🎯 Focus Achiever".to_string()
        } else {
            "✅ Day Complete".to_string()
        };

        // Generate headline
        let headline = format!(
            "You stashed {} tabs, freed {:.1} GB, and achieved {:.0} minutes of deep focus today.",
            tabs_stashed_today,
            total_memory_reclaimed_mb / 1024.0,
            total_flow_minutes,
        );

        Self {
            date_str,
            epoch: now_epoch,
            total_memory_reclaimed_mb,
            tabs_stashed_today,
            forgotten_loops_resolved: 0,
            forgotten_loops_remaining: badge_status.pending_alerts_count,
            flow_sessions_count,
            total_flow_minutes,
            primary_archetype,
            celebration_badge,
            headline,
        }
    }

    /// Persists the closure summary to SQLite
    pub fn persist(&self, storage_dir: &Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        std::fs::create_dir_all(storage_dir)?;
        let db_path = storage_dir.join("knowledge.db");
        let conn = Connection::open(&db_path)?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS daily_closures (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                date_str TEXT NOT NULL,
                epoch INTEGER NOT NULL,
                total_memory_reclaimed_mb REAL NOT NULL DEFAULT 0.0,
                tabs_stashed INTEGER NOT NULL DEFAULT 0,
                forgotten_resolved INTEGER NOT NULL DEFAULT 0,
                forgotten_remaining INTEGER NOT NULL DEFAULT 0,
                flow_sessions INTEGER NOT NULL DEFAULT 0,
                total_flow_minutes REAL NOT NULL DEFAULT 0.0,
                primary_archetype TEXT NOT NULL DEFAULT 'BalancedWork',
                celebration_badge TEXT NOT NULL DEFAULT '',
                headline TEXT NOT NULL DEFAULT ''
            );"
        )?;

        conn.execute(
            "INSERT INTO daily_closures
             (date_str, epoch, total_memory_reclaimed_mb, tabs_stashed, forgotten_resolved,
              forgotten_remaining, flow_sessions, total_flow_minutes, primary_archetype,
              celebration_badge, headline)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                self.date_str,
                self.epoch as i64,
                self.total_memory_reclaimed_mb,
                self.tabs_stashed_today as i64,
                self.forgotten_loops_resolved as i64,
                self.forgotten_loops_remaining as i64,
                self.flow_sessions_count as i64,
                self.total_flow_minutes,
                self.primary_archetype,
                self.celebration_badge,
                self.headline,
            ],
        )?;

        Ok(())
    }

    /// Renders a TUI-friendly ASCII card for the closure summary
    pub fn render_ascii(&self) -> String {
        let mut out = String::new();
        out.push_str("╔══════════════════════════════════════════════════════════════╗\n");
        out.push_str(&format!("║  {} Session Closure: {}  ║\n", self.celebration_badge, self.date_str));
        out.push_str("╠══════════════════════════════════════════════════════════════╣\n");
        out.push_str(&format!("║  {}  ║\n", self.headline));
        out.push_str("╠══════════════════════════════════════════════════════════════╣\n");
        out.push_str(&format!("║  📦 Tabs Stashed: {:>4}    💾 RAM Freed: {:>7.1} MB    ║\n",
            self.tabs_stashed_today, self.total_memory_reclaimed_mb));
        out.push_str(&format!("║  🧘 Flow Sessions: {:>3}    ⏱️ Focus Time: {:>5.0} min    ║\n",
            self.flow_sessions_count, self.total_flow_minutes));
        out.push_str(&format!("║  🧠 Archetype: {:>20}                     ║\n", self.primary_archetype));
        out.push_str("╚══════════════════════════════════════════════════════════════╝\n");
        out
    }

    /// Retrieves the most recent daily closure from SQLite
    pub fn load_latest(storage_dir: &Path) -> Option<Self> {
        let db_path = storage_dir.join("knowledge.db");
        let conn = Connection::open(&db_path).ok()?;
        let mut stmt = conn
            .prepare(
                "SELECT date_str, epoch, total_memory_reclaimed_mb, tabs_stashed, forgotten_resolved,
                        forgotten_remaining, flow_sessions, total_flow_minutes, primary_archetype,
                        celebration_badge, headline
                 FROM daily_closures
                 ORDER BY epoch DESC LIMIT 1",
            )
            .ok()?;

        stmt.query_row([], |row| {
            Ok(Self {
                date_str: row.get(0)?,
                epoch: row.get::<_, i64>(1)? as u64,
                total_memory_reclaimed_mb: row.get(2)?,
                tabs_stashed_today: row.get::<_, i64>(3)? as usize,
                forgotten_loops_resolved: row.get::<_, i64>(4)? as usize,
                forgotten_loops_remaining: row.get::<_, i64>(5)? as usize,
                flow_sessions_count: row.get::<_, i64>(6)? as usize,
                total_flow_minutes: row.get(7)?,
                primary_archetype: row.get(8)?,
                celebration_badge: row.get(9)?,
                headline: row.get(10)?,
            })
        })
        .ok()
    }
}

fn now_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn format_date(epoch: u64) -> String {
    // Simple date formatting (YYYY-MM-DD) without chrono dependency
    let secs_per_day: u64 = 86400;
    let days = epoch / secs_per_day;
    // Approximate calendar date from Unix epoch
    let years_approx = days / 365;
    let year = 1970 + years_approx;
    let remaining_days = days - (years_approx * 365 + years_approx / 4);
    let month = (remaining_days / 30).min(11) + 1;
    let day = (remaining_days % 30) + 1;
    format!("{:04}-{:02}-{:02}", year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_closure_generation() {
        let flow_summaries = vec![
            FlowBreakSummary {
                session_duration_mins: 52.0,
                suppressed_alerts: 3,
                message: "🎯 Flow session complete: 52 minutes".to_string(),
            },
            FlowBreakSummary {
                session_duration_mins: 78.0,
                suppressed_alerts: 5,
                message: "🎯 Flow session complete: 78 minutes".to_string(),
            },
        ];

        let closure = DailyClosureSummary::generate(&flow_summaries);
        assert_eq!(closure.flow_sessions_count, 2);
        assert!((closure.total_flow_minutes - 130.0).abs() < 0.1);
        assert!(closure.celebration_badge.contains("Champion"));
        assert!(!closure.headline.is_empty());
    }

    #[test]
    fn test_closure_ascii_render() {
        let closure = DailyClosureSummary {
            date_str: "2026-08-30".to_string(),
            epoch: 1788192000,
            total_memory_reclaimed_mb: 2100.0,
            tabs_stashed_today: 14,
            forgotten_loops_resolved: 3,
            forgotten_loops_remaining: 1,
            flow_sessions_count: 2,
            total_flow_minutes: 95.0,
            primary_archetype: "DeepCodingFlow".to_string(),
            celebration_badge: "🏆 Deep Work Champion".to_string(),
            headline: "You stashed 14 tabs, freed 2.1 GB, and achieved 95 minutes of deep focus today.".to_string(),
        };

        let ascii = closure.render_ascii();
        assert!(ascii.contains("Deep Work Champion"));
        assert!(ascii.contains("DeepCodingFlow"));
    }
}
