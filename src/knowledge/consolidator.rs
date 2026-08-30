//! Session Consolidation & Incremental Behavioral Rollup Engine
//!
//! Aggregates raw dwell, context-switching, and memory events into permanent
//! daily archetype profiles. Learns recurring process memory growth curves,
//! and runs 7-day rolling compaction to maintain zero-bloat local sovereignty.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::knowledge::schema::open_knowledge_db;

/// Developer Daily Work Archetype
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DailyWorkArchetype {
    DeepCodingFlow,
    HighSprawlResearch,
    DivergentTabHoarding,
    DebuggingThrashing,
    BalancedWork,
}

impl DailyWorkArchetype {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DeepCodingFlow => "Deep Coding Flow",
            Self::HighSprawlResearch => "High-Sprawl Research",
            Self::DivergentTabHoarding => "Divergent Tab Hoarding",
            Self::DebuggingThrashing => "Debugging & Thrashing",
            Self::BalancedWork => "Balanced Flow",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Deep Coding Flow" => Self::DeepCodingFlow,
            "High-Sprawl Research" => Self::HighSprawlResearch,
            "Divergent Tab Hoarding" => Self::DivergentTabHoarding,
            "Debugging & Thrashing" => Self::DebuggingThrashing,
            _ => Self::BalancedWork,
        }
    }
}

/// Consolidated Daily Summary Profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyProfile {
    pub date_str: String,
    pub total_focus_mins: f64,
    pub avg_active_tabs: f64,
    pub peak_active_tabs: usize,
    pub avg_switches_per_min: f64,
    pub reclaimed_memory_mb: f64,
    pub primary_archetype: DailyWorkArchetype,
    pub notes: String,
}

/// Engine managing incremental knowledge persistence and rollups
pub struct KnowledgeConsolidator {
    storage_dir: PathBuf,
}

#[allow(dead_code)]
impl KnowledgeConsolidator {
    pub fn new(storage_dir: &Path) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let _ = open_knowledge_db(storage_dir)?;
        Ok(Self {
            storage_dir: storage_dir.to_path_buf(),
        })
    }

    fn connect(&self) -> Result<Connection, rusqlite::Error> {
        let db_path = self.storage_dir.join("knowledge.db");
        Connection::open(&db_path)
    }

    /// Logs an application or tab dwell event
    pub fn log_dwell(&self, target_name: &str, dwell_secs: f64, memory_bytes: u64, is_tab: bool) {
        if dwell_secs < 1.0 {
            return;
        }

        let now_ms = now_epoch_ms();
        let meta = serde_json::json!({ "is_tab": is_tab }).to_string();

        if let Ok(conn) = self.connect() {
            let _ = conn.execute(
                "INSERT INTO behavioral_events (timestamp_ms, event_type, target_name, dwell_secs, memory_bytes, meta_json)
                 VALUES (?1, 'Dwell', ?2, ?3, ?4, ?5)",
                params![now_ms as i64, target_name, dwell_secs, memory_bytes as i64, meta],
            );
        }
    }

    /// Logs a context switch velocity spike or telemetry tick
    pub fn log_context_switch(&self, switches_per_min: f64, active_tabs: usize) {
        let now_ms = now_epoch_ms();
        let meta = serde_json::json!({
            "switches_per_min": switches_per_min,
            "active_tabs": active_tabs
        })
        .to_string();

        if let Ok(conn) = self.connect() {
            let _ = conn.execute(
                "INSERT INTO behavioral_events (timestamp_ms, event_type, target_name, dwell_secs, memory_bytes, meta_json)
                 VALUES (?1, 'SwitchSpike', 'ContextSwitch', 0.0, 0, ?2)",
                params![now_ms as i64, meta],
            );
        }
    }

    /// Logs user response to an interactive notification (e.g. 'focused', 'stashed', 'snoozed')
    pub fn log_toast_response(&self, action: &str, target_name: &str) {
        let now_ms = now_epoch_ms();
        let meta = serde_json::json!({ "action": action }).to_string();

        if let Ok(conn) = self.connect() {
            let _ = conn.execute(
                "INSERT INTO behavioral_events (timestamp_ms, event_type, target_name, dwell_secs, memory_bytes, meta_json)
                 VALUES (?1, 'ToastResponse', ?2, 0.0, 0, ?3)",
                params![now_ms as i64, target_name, meta],
            );
        }
    }

    /// Incremental daily consolidation rollup routine
    pub fn consolidate_today(&self) -> Result<DailyProfile, Box<dyn std::error::Error + Send + Sync>> {
        let date_str = today_date_str();
        self.consolidate_day(&date_str)
    }

    /// Consolidates events for a specific calendar date (YYYY-MM-DD)
    pub fn consolidate_day(&self, date_str: &str) -> Result<DailyProfile, Box<dyn std::error::Error + Send + Sync>> {
        let conn = self.connect()?;

        // Calculate focus minutes and average metrics from today's events
        let mut total_dwell_secs = 0.0f64;
        let mut switch_velocities = Vec::new();
        let mut tab_counts = Vec::new();

        {
            let mut stmt = conn.prepare(
                "SELECT event_type, dwell_secs, meta_json FROM behavioral_events WHERE timestamp_ms >= ?1",
            )?;

            // Approximate 24-hour window from midnight UTC of given date
            let start_of_day_ms = (now_epoch_ms() / 86_400_000) * 86_400_000;
            let rows = stmt.query_map(params![start_of_day_ms as i64], |row| {
                let ev_type: String = row.get(0)?;
                let dwell: f64 = row.get(1)?;
                let meta: String = row.get(2)?;
                Ok((ev_type, dwell, meta))
            })?;

            for r in rows.flatten() {
                let (ev_type, dwell, meta) = r;
                if ev_type == "Dwell" {
                    total_dwell_secs += dwell;
                } else if ev_type == "SwitchSpike" {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&meta) {
                        if let Some(s) = v.get("switches_per_min").and_then(|val| val.as_f64()) {
                            switch_velocities.push(s);
                        }
                        if let Some(t) = v.get("active_tabs").and_then(|val| val.as_u64()) {
                            tab_counts.push(t as usize);
                        }
                    }
                }
            }
        }

        let total_focus_mins = (total_dwell_secs / 60.0).max(1.0);
        let avg_active_tabs = if !tab_counts.is_empty() {
            tab_counts.iter().sum::<usize>() as f64 / tab_counts.len() as f64
        } else {
            8.0
        };
        let peak_active_tabs = tab_counts.iter().copied().max().unwrap_or(10);
        let avg_switches_per_min = if !switch_velocities.is_empty() {
            switch_velocities.iter().sum::<f64>() / switch_velocities.len() as f64
        } else {
            1.5
        };

        // Query total reclaimed RAM from badge / stashed_sessions
        let reclaimed_mb: f64 = {
            let status = crate::notifications::badge::global_badge().get_status();
            status.total_memory_reclaimed_mb
        };

        // Determine Behavioral Archetype
        let archetype = if avg_switches_per_min < 2.0 && total_focus_mins >= 5.0 {
            DailyWorkArchetype::DeepCodingFlow
        } else if avg_active_tabs > 16.0 && avg_switches_per_min > 4.5 {
            DailyWorkArchetype::DivergentTabHoarding
        } else if avg_active_tabs > 12.0 {
            DailyWorkArchetype::HighSprawlResearch
        } else {
            DailyWorkArchetype::BalancedWork
        };

        let profile = DailyProfile {
            date_str: date_str.to_string(),
            total_focus_mins,
            avg_active_tabs,
            peak_active_tabs,
            avg_switches_per_min,
            reclaimed_memory_mb: reclaimed_mb,
            primary_archetype: archetype.clone(),
            notes: format!("Consolidated at {}", now_epoch_ms()),
        };

        // Upsert into daily_archetype_profiles
        conn.execute(
            "INSERT INTO daily_archetype_profiles
             (date_str, total_focus_mins, avg_active_tabs, peak_active_tabs, avg_switches_per_min, reclaimed_memory_mb, primary_archetype, notes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(date_str) DO UPDATE SET
                total_focus_mins = excluded.total_focus_mins,
                avg_active_tabs = excluded.avg_active_tabs,
                peak_active_tabs = excluded.peak_active_tabs,
                avg_switches_per_min = excluded.avg_switches_per_min,
                reclaimed_memory_mb = excluded.reclaimed_memory_mb,
                primary_archetype = excluded.primary_archetype,
                notes = excluded.notes",
            params![
                profile.date_str,
                profile.total_focus_mins,
                profile.avg_active_tabs,
                profile.peak_active_tabs as i64,
                profile.avg_switches_per_min,
                profile.reclaimed_memory_mb,
                profile.primary_archetype.as_str(),
                profile.notes,
            ],
        )?;

        Ok(profile)
    }

    /// Learns or updates the memory consumption growth curve for a recurrent process
    pub fn update_process_memory_curve(
        &self,
        process_name: &str,
        current_bytes: u64,
        growth_rate_mb_hr: f64,
    ) {
        if let Ok(conn) = self.connect() {
            let now_epoch = now_epoch_ms() / 1000;
            let leak_prob = if growth_rate_mb_hr > 50.0 { 0.85 } else if growth_rate_mb_hr > 20.0 { 0.4 } else { 0.05 };

            let _ = conn.execute(
                "INSERT INTO process_memory_curves
                 (process_name, observation_count, baseline_bytes, growth_rate_mb_hr, leak_probability, last_updated_epoch)
                 VALUES (?1, 1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(process_name) DO UPDATE SET
                    observation_count = observation_count + 1,
                    baseline_bytes = (baseline_bytes + excluded.baseline_bytes) / 2,
                    growth_rate_mb_hr = (growth_rate_mb_hr * 0.7) + (excluded.growth_rate_mb_hr * 0.3),
                    leak_probability = (leak_probability * 0.7) + (excluded.leak_probability * 0.3),
                    last_updated_epoch = excluded.last_updated_epoch",
                params![process_name, current_bytes as i64, growth_rate_mb_hr, leak_prob, now_epoch as i64],
            );
        }
    }

    /// Performs rolling 7-day compaction, deleting granular events while retaining daily rollups
    pub fn compact_old_events(&self, retention_days: u64) -> Result<usize, rusqlite::Error> {
        let conn = self.connect()?;
        let cutoff_ms = now_epoch_ms().saturating_sub(retention_days * 86_400_000);

        let deleted = conn.execute(
            "DELETE FROM behavioral_events WHERE timestamp_ms < ?1",
            params![cutoff_ms as i64],
        )?;

        Ok(deleted)
    }

    /// Retrieves historical daily profiles
    pub fn get_recent_profiles(&self, limit: usize) -> Vec<DailyProfile> {
        let mut list = Vec::new();
        if let Ok(conn) = self.connect() {
            let mut stmt = conn
                .prepare(
                    "SELECT date_str, total_focus_mins, avg_active_tabs, peak_active_tabs,
                            avg_switches_per_min, reclaimed_memory_mb, primary_archetype, notes
                     FROM daily_archetype_profiles
                     ORDER BY date_str DESC
                     LIMIT ?1",
                )
                .unwrap();

            let rows = stmt.query_map(params![limit as i64], |row| {
                let arch_str: String = row.get(6)?;
                Ok(DailyProfile {
                    date_str: row.get(0)?,
                    total_focus_mins: row.get(1)?,
                    avg_active_tabs: row.get(2)?,
                    peak_active_tabs: row.get::<_, i64>(3)? as usize,
                    avg_switches_per_min: row.get(4)?,
                    reclaimed_memory_mb: row.get(5)?,
                    primary_archetype: DailyWorkArchetype::from_str(&arch_str),
                    notes: row.get(7)?,
                })
            });

            if let Ok(iter) = rows {
                for item in iter.flatten() {
                    list.push(item);
                }
            }
        }
        list
    }
}

/// Global singleton for knowledge consolidation
pub fn global_consolidator() -> &'static Mutex<KnowledgeConsolidator> {
    static CONSOLIDATOR: OnceLock<Mutex<KnowledgeConsolidator>> = OnceLock::new();
    CONSOLIDATOR.get_or_init(|| {
        let dir = Path::new("memoryblast_data");
        let consolidator = KnowledgeConsolidator::new(dir).unwrap_or_else(|_| {
            let fallback = std::env::temp_dir().join("memoryblast_knowledge");
            KnowledgeConsolidator::new(&fallback).expect("Failed to init KnowledgeConsolidator")
        });
        Mutex::new(consolidator)
    })
}

fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn today_date_str() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Simple UTC day calculation
    let days = secs / 86400;
    // Approximated ISO date string for local persistence
    let year = 1970 + days / 365;
    let day_of_year = days % 365;
    let month = (day_of_year / 30).min(11) + 1;
    let day = (day_of_year % 30) + 1;
    format!("{year:04}-{month:02}-{day:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consolidation_and_archetyping() {
        let temp_dir = std::env::temp_dir().join(format!("test_cons_{}", now_epoch_ms()));
        let consolidator = KnowledgeConsolidator::new(&temp_dir).unwrap();

        consolidator.log_dwell("Visual Studio Code", 300.0, 500 * 1024 * 1024, false);
        consolidator.log_dwell("Rust Docs", 120.0, 180 * 1024 * 1024, true);
        consolidator.log_context_switch(1.2, 8);

        let profile = consolidator.consolidate_today().unwrap();
        assert!(profile.total_focus_mins >= 5.0);
        assert_eq!(profile.primary_archetype, DailyWorkArchetype::DeepCodingFlow);

        let profiles = consolidator.get_recent_profiles(5);
        assert_eq!(profiles.len(), 1);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
