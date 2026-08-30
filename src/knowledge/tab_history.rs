//! Chromium Tab History & SQLite Persistence
//!
//! Stores exact opening/closing timestamps, shelf-life duration,
//! and closure causes for all observed Chromium tabs in SQLite (`tab_lifecycles`).

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use crate::chromium::lifecycle::ClosureCause;

/// Persistent record of a single tab's complete lifecycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabLifecycleRecord {
    pub id: i64,
    pub target_id: String,
    pub domain: String,
    pub url: String,
    pub title: String,
    pub opened_at_epoch: u64,
    pub closed_at_epoch: Option<u64>,
    pub shelf_life_secs: f64,
    pub active_dwell_secs: f64,
    pub closure_cause: ClosureCause,
    pub peak_memory_mb: f64,
}

/// Store managing SQLite persistence for tab lifecycles
pub struct TabHistoryStore {
    db_path: PathBuf,
}

impl TabHistoryStore {
    pub fn new(storage_dir: &Path) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        std::fs::create_dir_all(storage_dir)?;
        let db_path = storage_dir.join("knowledge.db");

        let conn = Connection::open(&db_path)?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS tab_lifecycles (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                target_id TEXT NOT NULL,
                domain TEXT NOT NULL,
                url TEXT NOT NULL,
                title TEXT NOT NULL,
                opened_at_epoch INTEGER NOT NULL,
                closed_at_epoch INTEGER,
                shelf_life_secs REAL NOT NULL DEFAULT 0.0,
                active_dwell_secs REAL NOT NULL DEFAULT 0.0,
                closure_cause TEXT NOT NULL,
                peak_memory_mb REAL NOT NULL DEFAULT 0.0
            );

            CREATE INDEX IF NOT EXISTS idx_tab_domain ON tab_lifecycles(domain);
            CREATE INDEX IF NOT EXISTS idx_tab_opened ON tab_lifecycles(opened_at_epoch DESC);
            CREATE INDEX IF NOT EXISTS idx_tab_cause ON tab_lifecycles(closure_cause);
            ",
        )?;

        Ok(Self { db_path })
    }

    fn connect(&self) -> Result<Connection, rusqlite::Error> {
        Connection::open(&self.db_path)
    }

    /// Records when a new tab is opened
    pub fn record_tab_opened(&self, rec: &TabLifecycleRecord) -> Result<i64, rusqlite::Error> {
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO tab_lifecycles
             (target_id, domain, url, title, opened_at_epoch, closed_at_epoch, shelf_life_secs, active_dwell_secs, closure_cause, peak_memory_mb)
             VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7, ?8, ?9)",
            params![
                rec.target_id,
                rec.domain,
                rec.url,
                rec.title,
                rec.opened_at_epoch as i64,
                rec.shelf_life_secs,
                rec.active_dwell_secs,
                rec.closure_cause.as_str(),
                rec.peak_memory_mb,
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Updates the record when the tab is closed or destroyed
    pub fn record_tab_closed(&self, rec: &TabLifecycleRecord) -> Result<(), rusqlite::Error> {
        let conn = self.connect()?;
        let closed_epoch = rec.closed_at_epoch.unwrap_or(rec.opened_at_epoch);

        let updated = conn.execute(
            "UPDATE tab_lifecycles SET
                closed_at_epoch = ?1,
                shelf_life_secs = ?2,
                active_dwell_secs = ?3,
                closure_cause = ?4,
                peak_memory_mb = ?5
             WHERE target_id = ?6 AND closed_at_epoch IS NULL",
            params![
                closed_epoch as i64,
                rec.shelf_life_secs,
                rec.active_dwell_secs,
                rec.closure_cause.as_str(),
                rec.peak_memory_mb,
                rec.target_id,
            ],
        )?;

        // If no open row was found with NULL closed_at_epoch, insert a completed record
        if updated == 0 {
            conn.execute(
                "INSERT INTO tab_lifecycles
                 (target_id, domain, url, title, opened_at_epoch, closed_at_epoch, shelf_life_secs, active_dwell_secs, closure_cause, peak_memory_mb)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    rec.target_id,
                    rec.domain,
                    rec.url,
                    rec.title,
                    rec.opened_at_epoch as i64,
                    closed_epoch as i64,
                    rec.shelf_life_secs,
                    rec.active_dwell_secs,
                    rec.closure_cause.as_str(),
                    rec.peak_memory_mb,
                ],
            )?;
        }

        Ok(())
    }

    /// Retrieves completed tab lifecycles for analytics
    pub fn get_completed_lifecycles(&self, limit: usize) -> Vec<TabLifecycleRecord> {
        let mut list = Vec::new();
        if let Ok(conn) = self.connect() {
            let mut stmt = conn
                .prepare(
                    "SELECT id, target_id, domain, url, title, opened_at_epoch, closed_at_epoch,
                            shelf_life_secs, active_dwell_secs, closure_cause, peak_memory_mb
                     FROM tab_lifecycles
                     WHERE closed_at_epoch IS NOT NULL
                     ORDER BY closed_at_epoch DESC
                     LIMIT ?1",
                )
                .unwrap();

            let rows = stmt.query_map(params![limit as i64], |row| {
                let cause_str: String = row.get(9)?;
                Ok(TabLifecycleRecord {
                    id: row.get(0)?,
                    target_id: row.get(1)?,
                    domain: row.get(2)?,
                    url: row.get(3)?,
                    title: row.get(4)?,
                    opened_at_epoch: row.get::<_, i64>(5)? as u64,
                    closed_at_epoch: row.get::<_, Option<i64>>(6)?.map(|e| e as u64),
                    shelf_life_secs: row.get(7)?,
                    active_dwell_secs: row.get(8)?,
                    closure_cause: ClosureCause::from_str(&cause_str),
                    peak_memory_mb: row.get(10)?,
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

/// Global singleton for tab history store
pub fn global_tab_history() -> &'static Mutex<TabHistoryStore> {
    static STORE: OnceLock<Mutex<TabHistoryStore>> = OnceLock::new();
    STORE.get_or_init(|| {
        let dir = Path::new("memoryblast_data");
        let store = TabHistoryStore::new(dir).unwrap_or_else(|_| {
            let fallback = std::env::temp_dir().join("memoryblast_knowledge");
            TabHistoryStore::new(&fallback).expect("Failed to init TabHistoryStore")
        });
        Mutex::new(store)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tab_history_lifecycle_storage() {
        let temp_dir = std::env::temp_dir().join(format!("test_tab_hist_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let store = TabHistoryStore::new(&temp_dir).unwrap();

        let mut rec = TabLifecycleRecord {
            id: 0,
            target_id: "tab_123".to_string(),
            domain: "docs.rs".to_string(),
            url: "https://docs.rs/rusqlite".to_string(),
            title: "rusqlite::Connection".to_string(),
            opened_at_epoch: 1000,
            closed_at_epoch: None,
            shelf_life_secs: 0.0,
            active_dwell_secs: 0.0,
            closure_cause: ClosureCause::UserManual,
            peak_memory_mb: 250.0,
        };

        store.record_tab_opened(&rec).unwrap();

        rec.closed_at_epoch = Some(1600);
        rec.shelf_life_secs = 600.0;
        rec.active_dwell_secs = 240.0;
        rec.closure_cause = ClosureCause::CopilotStashed;

        store.record_tab_closed(&rec).unwrap();

        let completed = store.get_completed_lifecycles(10);
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].domain, "docs.rs");
        assert_eq!(completed[0].shelf_life_secs, 600.0);
        assert_eq!(completed[0].closure_cause, ClosureCause::CopilotStashed);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
