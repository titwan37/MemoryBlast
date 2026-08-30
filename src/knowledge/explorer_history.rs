//! Explorer Folder History & Workspace Intelligence Persistence
//!
//! Stores exact opening/closing timestamps, shelf-life duration,
//! and dwell times for open File Explorer directories in SQLite (`explorer_lifecycles`).

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// Persistent record of a single Explorer folder tab/window's lifecycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorerLifecycleRecord {
    pub id: i64,
    pub tab_id: String,
    pub folder_path: String,
    pub project_name: String,
    pub opened_at_epoch: u64,
    pub closed_at_epoch: Option<u64>,
    pub shelf_life_secs: f64,
    pub active_dwell_secs: f64,
    pub is_forgotten_loop: bool,
}

/// Consolidated summary of a frequently used workspace root directory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceRootSummary {
    pub root_path: String,
    pub total_dwell_mins: f64,
    pub observation_count: usize,
    pub last_active_epoch: u64,
}

/// Store managing SQLite persistence for File Explorer lifecycles
pub struct ExplorerHistoryStore {
    db_path: PathBuf,
}

impl ExplorerHistoryStore {
    pub fn new(storage_dir: &Path) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        std::fs::create_dir_all(storage_dir)?;
        let db_path = storage_dir.join("knowledge.db");

        let conn = Connection::open(&db_path)?;
        conn.execute_batch(
            "
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

        Ok(Self { db_path })
    }

    fn connect(&self) -> Result<Connection, rusqlite::Error> {
        Connection::open(&self.db_path)
    }

    /// Records when a new folder window is opened
    pub fn record_folder_opened(&self, rec: &ExplorerLifecycleRecord) -> Result<i64, rusqlite::Error> {
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO explorer_lifecycles
             (tab_id, folder_path, project_name, opened_at_epoch, closed_at_epoch, shelf_life_secs, active_dwell_secs, is_forgotten_loop)
             VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6, ?7)",
            params![
                rec.tab_id,
                rec.folder_path,
                rec.project_name,
                rec.opened_at_epoch as i64,
                rec.shelf_life_secs,
                rec.active_dwell_secs,
                rec.is_forgotten_loop as i32,
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Updates the record when the folder window is closed
    pub fn record_folder_closed(&self, rec: &ExplorerLifecycleRecord) -> Result<(), rusqlite::Error> {
        let conn = self.connect()?;
        let closed_epoch = rec.closed_at_epoch.unwrap_or(rec.opened_at_epoch);

        let updated = conn.execute(
            "UPDATE explorer_lifecycles SET
                closed_at_epoch = ?1,
                shelf_life_secs = ?2,
                active_dwell_secs = ?3,
                is_forgotten_loop = ?4
             WHERE tab_id = ?5 AND closed_at_epoch IS NULL",
            params![
                closed_epoch as i64,
                rec.shelf_life_secs,
                rec.active_dwell_secs,
                rec.is_forgotten_loop as i32,
                rec.tab_id,
            ],
        )?;

        if updated == 0 {
            conn.execute(
                "INSERT INTO explorer_lifecycles
                 (tab_id, folder_path, project_name, opened_at_epoch, closed_at_epoch, shelf_life_secs, active_dwell_secs, is_forgotten_loop)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    rec.tab_id,
                    rec.folder_path,
                    rec.project_name,
                    rec.opened_at_epoch as i64,
                    closed_epoch as i64,
                    rec.shelf_life_secs,
                    rec.active_dwell_secs,
                    rec.is_forgotten_loop as i32,
                ],
            )?;
        }

        Ok(())
    }

    /// Aggregates top workspace root directories based on cumulative active dwell time
    pub fn get_top_workspace_roots(&self, limit: usize) -> Vec<WorkspaceRootSummary> {
        let mut list = Vec::new();
        if let Ok(conn) = self.connect() {
            let mut stmt = conn
                .prepare(
                    "SELECT folder_path, SUM(active_dwell_secs), COUNT(*), MAX(opened_at_epoch)
                     FROM explorer_lifecycles
                     GROUP BY folder_path
                     ORDER BY SUM(active_dwell_secs) DESC
                     LIMIT ?1",
                )
                .unwrap();

            let rows = stmt.query_map(params![limit as i64], |row| {
                let path: String = row.get(0)?;
                let total_dwell_secs: f64 = row.get(1).unwrap_or(0.0);
                let count: i64 = row.get(2)?;
                let max_epoch: i64 = row.get(3)?;

                Ok(WorkspaceRootSummary {
                    root_path: path,
                    total_dwell_mins: (total_dwell_secs / 60.0 * 10.0).round() / 10.0,
                    observation_count: count as usize,
                    last_active_epoch: max_epoch as u64,
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

    /// Retrieves list of forgotten Explorer tabs (open >90 min with low interaction)
    pub fn get_forgotten_explorer_tabs(&self) -> Vec<ExplorerLifecycleRecord> {
        let mut list = Vec::new();
        if let Ok(conn) = self.connect() {
            let mut stmt = conn
                .prepare(
                    "SELECT id, tab_id, folder_path, project_name, opened_at_epoch, closed_at_epoch,
                            shelf_life_secs, active_dwell_secs, is_forgotten_loop
                     FROM explorer_lifecycles
                     WHERE is_forgotten_loop = 1
                     ORDER BY opened_at_epoch DESC
                     LIMIT 20",
                )
                .unwrap();

            let rows = stmt.query_map([], |row| {
                Ok(ExplorerLifecycleRecord {
                    id: row.get(0)?,
                    tab_id: row.get(1)?,
                    folder_path: row.get(2)?,
                    project_name: row.get(3)?,
                    opened_at_epoch: row.get::<_, i64>(4)? as u64,
                    closed_at_epoch: row.get::<_, Option<i64>>(5)?.map(|e| e as u64),
                    shelf_life_secs: row.get(6)?,
                    active_dwell_secs: row.get(7)?,
                    is_forgotten_loop: row.get::<_, i32>(8)? == 1,
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

/// Global singleton for the Explorer history store
pub fn global_explorer_history() -> &'static Mutex<ExplorerHistoryStore> {
    static STORE: OnceLock<Mutex<ExplorerHistoryStore>> = OnceLock::new();
    STORE.get_or_init(|| {
        let dir = Path::new("memoryblast_data");
        let store = ExplorerHistoryStore::new(dir).unwrap_or_else(|_| {
            let fallback = std::env::temp_dir().join("memoryblast_knowledge");
            ExplorerHistoryStore::new(&fallback).expect("Failed to init ExplorerHistoryStore")
        });
        Mutex::new(store)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explorer_lifecycle_storage() {
        let temp_dir = std::env::temp_dir().join(format!(
            "test_exp_hist_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let store = ExplorerHistoryStore::new(&temp_dir).unwrap();

        let mut rec = ExplorerLifecycleRecord {
            id: 0,
            tab_id: "hwnd_999".to_string(),
            folder_path: r"C:\Dev\MemoryBlast".to_string(),
            project_name: "MemoryBlast".to_string(),
            opened_at_epoch: 1000,
            closed_at_epoch: None,
            shelf_life_secs: 0.0,
            active_dwell_secs: 0.0,
            is_forgotten_loop: false,
        };

        store.record_folder_opened(&rec).unwrap();

        rec.closed_at_epoch = Some(2200);
        rec.shelf_life_secs = 1200.0;
        rec.active_dwell_secs = 450.0;

        store.record_folder_closed(&rec).unwrap();

        let top_roots = store.get_top_workspace_roots(5);
        assert_eq!(top_roots.len(), 1);
        assert_eq!(top_roots[0].root_path, r"C:\Dev\MemoryBlast");
        assert!(top_roots[0].total_dwell_mins >= 7.0);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
