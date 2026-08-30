//! SQLite Database Schema & Migrations for Longitudinal Knowledge Engine
//!
//! Provides structured schemas and migrations for:
//! - behavioral_events: Window/tab dwell times, context switch spikes, notification responses
//! - daily_archetype_profiles: Consolidated day-by-day developer productivity metrics
//! - process_memory_curves: Learned memory footprint and growth/leak curves for recurring tools
//! - learned_user_preferences: Adaptive thresholds (dynamic working memory cap, peak focus windows)

use rusqlite::Connection;
use std::path::Path;

pub fn initialize_knowledge_schema(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;

        -- 1. Longitudinal Granular Behavioral Events (Compacted after 7 days)
        CREATE TABLE IF NOT EXISTS behavioral_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp_ms INTEGER NOT NULL,
            event_type TEXT NOT NULL, -- 'Dwell', 'SwitchSpike', 'Thrashing', 'ToastResponse'
            target_name TEXT NOT NULL,
            dwell_secs REAL NOT NULL DEFAULT 0.0,
            memory_bytes INTEGER NOT NULL DEFAULT 0,
            meta_json TEXT NOT NULL DEFAULT '{}'
        );

        CREATE INDEX IF NOT EXISTS idx_events_ts ON behavioral_events(timestamp_ms DESC);
        CREATE INDEX IF NOT EXISTS idx_events_type ON behavioral_events(event_type);

        -- 2. Daily Consolidated Archetype Profiles (Permanent Historical Rollups)
        CREATE TABLE IF NOT EXISTS daily_archetype_profiles (
            date_str TEXT PRIMARY KEY, -- 'YYYY-MM-DD'
            total_focus_mins REAL NOT NULL DEFAULT 0.0,
            avg_active_tabs REAL NOT NULL DEFAULT 0.0,
            peak_active_tabs INTEGER NOT NULL DEFAULT 0,
            avg_switches_per_min REAL NOT NULL DEFAULT 0.0,
            reclaimed_memory_mb REAL NOT NULL DEFAULT 0.0,
            primary_archetype TEXT NOT NULL DEFAULT 'BalancedWork',
            notes TEXT NOT NULL DEFAULT ''
        );

        -- 3. Learned Process Memory Decay & Growth Signatures
        CREATE TABLE IF NOT EXISTS process_memory_curves (
            process_name TEXT PRIMARY KEY,
            observation_count INTEGER NOT NULL DEFAULT 1,
            baseline_bytes INTEGER NOT NULL DEFAULT 0,
            growth_rate_mb_hr REAL NOT NULL DEFAULT 0.0,
            leak_probability REAL NOT NULL DEFAULT 0.0,
            last_updated_epoch INTEGER NOT NULL
        );

        -- 4. Personalized Dynamic Heuristics & Preferences
        CREATE TABLE IF NOT EXISTS learned_user_preferences (
            pref_key TEXT PRIMARY KEY,
            value_json TEXT NOT NULL,
            confidence_score REAL NOT NULL DEFAULT 1.0,
            last_updated_epoch INTEGER NOT NULL
        );

        -- 5. Windows File Explorer Folder Lifecycles & Workspaces
        CREATE TABLE IF NOT EXISTS explorer_lifecycles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tab_id TEXT NOT NULL,
            folder_path TEXT NOT NULL,
            project_name TEXT NOT NULL,
            opened_at_epoch INTEGER NOT NULL,
            closed_at_epoch INTEGER,
            shelf_life_secs REAL NOT NULL DEFAULT 0.0,
            active_dwell_secs REAL NOT NULL DEFAULT 0.0,
            is_forgotten_loop INTEGER NOT NULL DEFAULT 0
        );

        CREATE INDEX IF NOT EXISTS idx_explorer_path ON explorer_lifecycles(folder_path);
        CREATE INDEX IF NOT EXISTS idx_explorer_opened ON explorer_lifecycles(opened_at_epoch DESC);
        ",
    )?;

    Ok(())
}

/// Helper to connect and initialize the knowledge database
pub fn open_knowledge_db(storage_dir: &Path) -> Result<Connection, Box<dyn std::error::Error + Send + Sync>> {
    std::fs::create_dir_all(storage_dir)?;
    let db_path = storage_dir.join("knowledge.db");
    let conn = Connection::open(&db_path)?;
    initialize_knowledge_schema(&conn)?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_initialization() {
        let conn = Connection::open_in_memory().unwrap();
        initialize_knowledge_schema(&conn).unwrap();

        // Verify tables exist
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('behavioral_events', 'daily_archetype_profiles', 'process_memory_curves', 'learned_user_preferences')",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 4);
    }
}
