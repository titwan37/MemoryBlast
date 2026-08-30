//! Session Park & Reclaim Engine
//!
//! Provides one-click task freezing, stashing open tabs into SQLite / Markdown,
//! and terminating or trimming browser tabs to instantly restore focus and RAM.
//! Listens for the global hotkey Ctrl+Shift+F.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use windows::Win32::Foundation::{BOOL, HWND};
use windows::Win32::UI::WindowsAndMessaging::MSG;

use crate::chromium::query_cdp_tabs;
use crate::notifications::badge::global_badge;
use crate::notifications::toast::send_celebration_toast;
use crate::proc::kill_process;

#[link(name = "user32")]
extern "system" {
    fn RegisterHotKey(hwnd: HWND, id: i32, fsmodifiers: u32, vk: u32) -> BOOL;
    fn UnregisterHotKey(hwnd: HWND, id: i32) -> BOOL;
    fn GetMessageW(lpmsg: *mut MSG, hwnd: HWND, wmsgfiltermin: u32, wmsgfiltermax: u32) -> BOOL;
}

const MOD_CONTROL: u32 = 0x0002;
const MOD_SHIFT: u32 = 0x0004;
const VK_F: u32 = 0x46;
const HOTKEY_ID: i32 = 0x9001;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StashedTab {
    pub title: String,
    pub url: String,
    pub pid: Option<u32>,
    pub memory_bytes: u64,
    pub cdp_id: Option<String>,
    #[serde(default)]
    pub is_explorer_folder: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StashedSession {
    pub id: i64,
    pub timestamp_ms: u64,
    pub session_name: String,
    pub tab_count: usize,
    pub total_memory_bytes: u64,
    pub tabs: Vec<StashedTab>,
    pub restored: bool,
}

pub type StashResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub struct StashManager {
    db_path: PathBuf,
    stashes_dir: PathBuf,
}

impl StashManager {
    pub fn new(data_dir: &Path) -> StashResult<Self> {
        let stashes_dir = data_dir.join("stashes");
        fs::create_dir_all(&stashes_dir)?;
        let db_path = data_dir.join("sessions.db");

        let conn = Connection::open(&db_path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS stashed_sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp_ms INTEGER NOT NULL,
                session_name TEXT NOT NULL,
                tab_count INTEGER NOT NULL,
                total_memory_bytes INTEGER NOT NULL,
                tabs_json TEXT NOT NULL,
                restored INTEGER DEFAULT 0
            );",
        )?;

        Ok(Self {
            db_path,
            stashes_dir,
        })
    }

    pub fn save_session(
        &self,
        name: &str,
        tabs: Vec<StashedTab>,
        total_mem: u64,
    ) -> StashResult<i64> {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let tabs_json = serde_json::to_string(&tabs)?;
        let conn = Connection::open(&self.db_path)?;

        conn.execute(
            "INSERT INTO stashed_sessions (timestamp_ms, session_name, tab_count, total_memory_bytes, tabs_json, restored)
             VALUES (?1, ?2, ?3, ?4, ?5, 0)",
            params![now_ms, name, tabs.len(), total_mem, tabs_json],
        )?;

        let id = conn.last_insert_rowid();

        // Also write a human-readable markdown checklist session file
        self.write_markdown_checklist(id, name, now_ms, &tabs, total_mem)?;

        Ok(id)
    }

    pub fn list_sessions(&self) -> StashResult<Vec<StashedSession>> {
        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare(
            "SELECT id, timestamp_ms, session_name, tab_count, total_memory_bytes, tabs_json, restored
             FROM stashed_sessions ORDER BY id DESC LIMIT 50",
        )?;

        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let timestamp_ms: u64 = row.get(1)?;
            let session_name: String = row.get(2)?;
            let tab_count: usize = row.get(3)?;
            let total_memory_bytes: u64 = row.get(4)?;
            let tabs_json: String = row.get(5)?;
            let restored_int: i32 = row.get(6)?;

            let tabs: Vec<StashedTab> = serde_json::from_str(&tabs_json).unwrap_or_default();
            Ok(StashedSession {
                id,
                timestamp_ms,
                session_name,
                tab_count,
                total_memory_bytes,
                tabs,
                restored: restored_int != 0,
            })
        })?;

        let mut results = Vec::new();
        for r in rows {
            if let Ok(sess) = r {
                results.push(sess);
            }
        }
        Ok(results)
    }

    pub fn mark_restored(&self, id: i64) -> StashResult<()> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "UPDATE stashed_sessions SET restored = 1 WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    fn write_markdown_checklist(
        &self,
        id: i64,
        name: &str,
        ts: u64,
        tabs: &[StashedTab],
        total_mem: u64,
    ) -> StashResult<()> {
        let filename = format!("session_{ts}_id{id}.md");
        let path = self.stashes_dir.join(filename);

        let mem_mb = total_mem as f64 / (1024.0 * 1024.0);
        let mut md = String::new();
        md.push_str(&format!("# 📦 Stashed Session: {name}\n\n"));
        md.push_str(&format!("- **Session ID**: `{id}`\n"));
        md.push_str(&format!("- **Tab Count**: {cnt}\n", cnt = tabs.len()));
        md.push_str(&format!("- **Reclaimed RAM**: {mem_mb:.1} MB\n"));
        md.push_str(&format!("- **Stashed Time**: {}\n\n", ts));

        // Capture developer working state
        let anchor = crate::copilot::anchor::ResumeAnchor::capture(
            Path::new("."),
            &tabs.iter().map(|t| (t.title.clone(), t.url.clone())).collect::<Vec<_>>(),
        );

        md.push_str("## ⚓ Developer Working State\n\n");
        md.push_str(&format!("- **Git Branch**: `{}`\n", anchor.git_branch));
        md.push_str(&format!("- **Last Commit**: `{}`\n", anchor.git_last_commit));
        if let Some(ref file) = anchor.active_file_path {
            md.push_str(&format!("- **Active File**: `{}` (line {})\n", file, anchor.active_line_number.unwrap_or(1)));
        }
        if let Some(ref url) = anchor.ide_launch_url {
            md.push_str(&format!("- **1-Click IDE Launch**: [{}]({})\n", url, url));
        }
        if let Some(ref clip) = anchor.last_clipboard_snippet {
            md.push_str(&format!("- **Last Clipboard Reference**: `{}`\n", clip.replace('\n', " ")));
        }
        md.push_str("\n");

        md.push_str("## 🎯 Saved Task & Tab Checklist\n\n");

        for (i, tab) in tabs.iter().filter(|t| !t.is_explorer_folder).enumerate() {
            let link = if !tab.url.is_empty() && tab.url.starts_with("http") {
                format!("[{}]({})", tab.title, tab.url)
            } else {
                tab.title.clone()
            };
            let tab_ram = tab.memory_bytes as f64 / (1024.0 * 1024.0);
            md.push_str(&format!("{}. [ ] {} ({:.1} MB)\n", i + 1, link, tab_ram));
        }

        let folders: Vec<_> = tabs.iter().filter(|t| t.is_explorer_folder).collect();
        if !folders.is_empty() {
            md.push_str("\n## 📁 Stashed File Explorer Workspaces\n\n");
            for (i, folder) in folders.iter().enumerate() {
                md.push_str(&format!("{}. [ ] `{}` ({})\n", i + 1, folder.url, folder.title));
            }
        }

        md.push_str("\n---\n*Created automatically by MemoryBlast Executive Function & Focus Copilot.*\n");

        fs::write(path, md)?;
        Ok(())
    }
}

/// Discovers open tabs via CDP and process inspection, parks them into a session snapshot,
/// and closes the tabs/processes to liberate working memory.
pub async fn park_current_context(
    known_cdp_ports: &[u16],
    candidate_renderer_pids: &[(u32, String, u64)], // (pid, title, working_set)
) -> Result<StashedSession, Box<dyn std::error::Error + Send + Sync>> {
    let mut stashed_tabs = Vec::new();
    let mut total_memory_bytes = 0u64;

    // 1. Ingest tabs from active CDP debug ports if available
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(400))
        .build()?;

    let mut closed_cdp_ids = Vec::new();
    for &port in known_cdp_ports {
        let cdp_tabs = query_cdp_tabs(port).await;
        for tab in cdp_tabs {
            if let Some(tab_type) = &tab.target_type {
                if tab_type != "page" {
                    continue;
                }
            }

            let title = tab.title.unwrap_or_else(|| "Untitled Tab".to_string());
            let url = tab.url.unwrap_or_default();
            let cdp_id = tab.id.clone();

            stashed_tabs.push(StashedTab {
                title: title.clone(),
                url: url.clone(),
                pid: None,
                memory_bytes: 150 * 1024 * 1024, // baseline estimation for active page
                cdp_id: cdp_id.clone(),
                is_explorer_folder: false,
            });
            total_memory_bytes += 150 * 1024 * 1024;

            // Close via CDP /json/close/{id}
            if let Some(ref id) = cdp_id {
                let close_url = format!("http://127.0.0.1:{port}/json/close/{id}");
                let _ = client.get(&close_url).send().await;
                closed_cdp_ids.push(id.clone());
            }
        }
    }

    // 2. Include tabs identified from renderer processes
    for (pid, title, ws) in candidate_renderer_pids {
        if !stashed_tabs.iter().any(|t| t.title == *title) {
            stashed_tabs.push(StashedTab {
                title: title.clone(),
                url: String::new(),
                pid: Some(*pid),
                memory_bytes: *ws,
                cdp_id: None,
                is_explorer_folder: false,
            });
            total_memory_bytes += *ws;
        }

        // If CDP was not used for this tab, terminate renderer process to reclaim RAM
        let _ = kill_process(*pid);
    }

    // 3. Include active Windows File Explorer workspace folders
    let open_folders = crate::explorer::resolve_open_explorer_folders();
    for folder in open_folders {
        if !stashed_tabs.iter().any(|t| t.url == folder.folder_path) {
            stashed_tabs.push(StashedTab {
                title: format!("📁 {}", folder.project_name),
                url: folder.folder_path.clone(),
                pid: None,
                memory_bytes: 45 * 1024 * 1024,
                cdp_id: None,
                is_explorer_folder: true,
            });
            total_memory_bytes += 45 * 1024 * 1024;
        }
    }

    let tab_count = stashed_tabs.len().max(1);
    let reclaimed_mb = total_memory_bytes as f64 / (1024.0 * 1024.0);

    // Save to SQLite
    let manager = StashManager::new(Path::new("memoryblast_data"))?;
    let session_name = format!(
        "Parked Session ({cnt} items, {mb:.0} MB)",
        cnt = tab_count,
        mb = reclaimed_mb
    );

    let id = manager.save_session(&session_name, stashed_tabs.clone(), total_memory_bytes)?;

    // Update global badge & celebration metrics
    let badge = global_badge();
    badge.record_stash(tab_count, reclaimed_mb);

    // Trigger celebration toast
    let _ = send_celebration_toast(tab_count, reclaimed_mb);

    Ok(StashedSession {
        id,
        timestamp_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
        session_name,
        tab_count,
        total_memory_bytes,
        tabs: stashed_tabs,
        restored: false,
    })
}

/// Restores a previously stashed session by reopening tab URLs and Explorer folders
pub fn restore_session(stash_id: i64) -> StashResult<usize> {
    let manager = StashManager::new(Path::new("memoryblast_data"))?;
    let sessions = manager.list_sessions()?;
    let session = sessions
        .into_iter()
        .find(|s| s.id == stash_id)
        .ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> {
            "Stashed session not found".into()
        })?;

    let mut restored_count = 0;
    for tab in &session.tabs {
        if tab.is_explorer_folder
            || tab.url.starts_with("C:\\")
            || tab.url.starts_with("D:\\")
            || tab.url.starts_with(r"\\")
        {
            let _ = std::process::Command::new("explorer.exe")
                .arg(&tab.url)
                .spawn();
            restored_count += 1;
        } else if !tab.url.is_empty() && tab.url.starts_with("http") {
            let _ = std::process::Command::new("cmd")
                .args(["/c", "start", "", &tab.url])
                .spawn();
            restored_count += 1;
        }
    }

    let _ = manager.mark_restored(stash_id);
    let badge = global_badge();
    badge.update(|s| {
        s.last_action_message = Some(format!(
            "Restored session #{stash_id} ({restored_count} tabs & folders reopened)"
        ));
    });

    Ok(restored_count)
}

/// Spawns the dedicated Win32 Global Hotkey thread listening for Ctrl+Shift+F
pub fn spawn_global_hotkey_listener(
    trigger_callback: Arc<dyn Fn() + Send + Sync + 'static>,
) -> StashResult<()> {
    std::thread::Builder::new()
        .name("copilot-hotkey".to_string())
        .spawn(move || unsafe {
            let null_hwnd = HWND(std::ptr::null_mut());
            let registered = RegisterHotKey(null_hwnd, HOTKEY_ID, MOD_CONTROL | MOD_SHIFT, VK_F);
            if !registered.as_bool() {
                eprintln!("⚠️ Notice: Could not register Ctrl+Shift+F global hotkey (might be in use by another app)");
                return;
            }

            println!("⚡ Focus Copilot: Global hotkey [Ctrl+Shift+F] active — Press anytime to Park Current Context.");

            let mut msg = MSG::default();
            while GetMessageW(&mut msg, null_hwnd, 0, 0).as_bool() {
                if msg.message == 0x0312 && msg.wParam.0 == HOTKEY_ID as usize {
                    // WM_HOTKEY
                    println!("🎯 [Ctrl+Shift+F] Global Hotkey detected! Parking current context...");
                    trigger_callback();
                }
            }

            let _ = UnregisterHotKey(null_hwnd, HOTKEY_ID);
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stash_manager_save_and_list() {
        let temp_dir = std::env::temp_dir().join(format!("memoryblast_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let manager = StashManager::new(&temp_dir).expect("Failed to create test StashManager");

        let test_tabs = vec![
            StashedTab {
                title: "Deep Rust Async Research".to_string(),
                url: "https://docs.rs/tokio".to_string(),
                pid: Some(1234),
                memory_bytes: 350 * 1024 * 1024,
                cdp_id: Some("target-1".to_string()),
                is_explorer_folder: false,
            },
            StashedTab {
                title: "ADHD Cognitive Ergonomics Paper".to_string(),
                url: "https://arxiv.org/abs/2401.0000".to_string(),
                pid: Some(5678),
                memory_bytes: 220 * 1024 * 1024,
                cdp_id: Some("target-2".to_string()),
                is_explorer_folder: false,
            },
        ];

        let id = manager.save_session("Test Research Session", test_tabs, 570 * 1024 * 1024).expect("Failed to save session");
        assert!(id > 0);

        let sessions = manager.list_sessions().expect("Failed to list sessions");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].tab_count, 2);
        assert_eq!(sessions[0].tabs[0].title, "Deep Rust Async Research");
        assert_eq!(sessions[0].restored, false);

        manager.mark_restored(id).expect("Failed to mark restored");
        let updated = manager.list_sessions().expect("Failed to list sessions");
        assert_eq!(updated[0].restored, true);

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
