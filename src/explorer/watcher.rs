//! Windows File Explorer Window & Tab Lifecycle Watcher
//!
//! Inspects active `explorer.exe` windows (`CabinetWClass`), extracts active folder
//! directory paths via UI Automation and Win32 window properties, tracks exact opening
//! and closing timestamps, calculates dwell duration, and identifies forgotten folder tabs.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use windows::core::VARIANT;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, TreeScope_Descendants, UIA_ControlTypePropertyId,
    UIA_EditControlTypeId,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW,
    IsWindowVisible,
};

use crate::knowledge::explorer_history::{global_explorer_history, ExplorerLifecycleRecord};

/// Information about an active Explorer folder window or tab
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorerFolderInfo {
    pub hwnd: isize,
    pub folder_path: String,
    pub title: String,
    pub project_name: String,
    pub opened_at_epoch: u64,
    pub active_dwell_secs: f64,
    pub shelf_life_secs: f64,
    pub is_forgotten_loop: bool,
}

struct WindowSearchContext {
    hwnds: Vec<HWND>,
}

unsafe extern "system" fn enum_explorer_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctx = &mut *(lparam.0 as *mut WindowSearchContext);

    if !IsWindowVisible(hwnd).as_bool() {
        return BOOL(1);
    }

    let mut class_buf = [0u16; 256];
    let len = GetClassNameW(hwnd, &mut class_buf);
    let class_name = String::from_utf16_lossy(&class_buf[..len as usize]);

    // Standard Windows File Explorer window class
    if class_name == "CabinetWClass" {
        ctx.hwnds.push(hwnd);
    }

    BOOL(1)
}

/// Resolves all currently open Explorer folders and Windows 11 tabs
pub fn resolve_open_explorer_folders() -> Vec<ExplorerFolderInfo> {
    let mut ctx = WindowSearchContext { hwnds: Vec::new() };

    unsafe {
        let _ = EnumWindows(
            Some(enum_explorer_windows_proc),
            LPARAM(&mut ctx as *mut _ as isize),
        );
    }

    let mut results = Vec::new();
    let now_epoch = now_epoch_secs();

    for hwnd in ctx.hwnds {
        let hwnd_val = hwnd.0 as isize;

        // 1. Read window text (often the folder name or full path)
        let title = unsafe {
            let length = GetWindowTextLengthW(hwnd);
            if length > 0 {
                let mut buf = vec![0u16; (length + 1) as usize];
                let read_len = GetWindowTextW(hwnd, &mut buf);
                if read_len > 0 {
                    String::from_utf16_lossy(&buf[..read_len as usize])
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        };

        // 2. Try UIA extraction for tabs and address bar path
        let uia_path = extract_path_via_uia(hwnd);

        let folder_path = if let Some(p) = uia_path {
            p
        } else if is_potential_filesystem_path(&title) {
            title.clone()
        } else {
            // Fallback: search common drive prefixes or title
            title.clone()
        };

        let project_name = extract_project_name(&folder_path);

        results.push(ExplorerFolderInfo {
            hwnd: hwnd_val,
            folder_path,
            title,
            project_name,
            opened_at_epoch: now_epoch,
            active_dwell_secs: 0.0,
            shelf_life_secs: 0.0,
            is_forgotten_loop: false,
        });
    }

    results
}

/// Uses Windows UI Automation to extract the real filesystem path from the address bar edit control
fn extract_path_via_uia(hwnd: HWND) -> Option<String> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let uia: Result<IUIAutomation, _> = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER);

        let Ok(uia) = uia else {
            CoUninitialize();
            return None;
        };

        let root_elem = uia.ElementFromHandle(hwnd);
        let Ok(root) = root_elem else {
            CoUninitialize();
            return None;
        };

        // Look for Edit control (the Explorer address bar edit box)
        let edit_prop = VARIANT::from(UIA_EditControlTypeId.0);
        let edit_cond = uia.CreatePropertyCondition(UIA_ControlTypePropertyId, &edit_prop);

        if let Ok(cond) = edit_cond {
            if let Ok(elem) = root.FindFirst(TreeScope_Descendants, &cond) {
                if let Ok(name_bstr) = elem.CurrentName() {
                    let text = name_bstr.to_string();
                    if let Some(clean) = clean_address_bar_text(&text) {
                        CoUninitialize();
                        return Some(clean);
                    }
                }
            }
        }

        CoUninitialize();
        None
    }
}

/// Cleans address bar text (which often looks like "Address: C:\Dev\MemoryBlast")
pub fn clean_address_bar_text(raw: &str) -> Option<String> {
    let stripped = raw.trim();
    let lower = stripped.to_lowercase();

    let path_candidate = if lower.starts_with("address: ") || lower.starts_with("adresse : ") {
        &stripped[9..].trim()
    } else if lower.starts_with("address:") || lower.starts_with("adresse:") {
        &stripped[8..].trim()
    } else {
        stripped
    };

    if is_potential_filesystem_path(path_candidate) {
        Some(path_candidate.to_string())
    } else {
        None
    }
}

/// Decodes file URI formats (`file:///C:/Users/...` or URL-encoded paths)
pub fn decode_file_uri(uri: &str) -> Option<String> {
    let trimmed = uri.trim();
    if let Some(rest) = trimmed.strip_prefix("file:///") {
        let decoded = percent_decode_str(rest).replace('/', "\\");
        Some(decoded)
    } else if let Some(rest) = trimmed.strip_prefix("file://") {
        let decoded = percent_decode_str(rest).replace('/', "\\");
        Some(decoded)
    } else if is_potential_filesystem_path(trimmed) {
        Some(trimmed.to_string())
    } else {
        None
    }
}

fn percent_decode_str(s: &str) -> String {
    let mut out = String::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex_val) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(hex_val as char);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn is_potential_filesystem_path(s: &str) -> bool {
    let s = s.trim();
    if s.len() >= 3 {
        let bytes = s.as_bytes();
        // Check Windows drive syntax "C:\" or "C:/"
        if bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && (bytes[2] == b'\\' || bytes[2] == b'/') {
            return true;
        }
        // UNC path "\\server\share"
        if s.starts_with(r"\\") {
            return true;
        }
    }
    false
}

/// Extracts a friendly project / directory name from path
pub fn extract_project_name(path_str: &str) -> String {
    let p = Path::new(path_str);
    if let Some(name) = p.file_name() {
        return name.to_string_lossy().to_string();
    }
    path_str.to_string()
}

/// Active in-memory tracking state for an Explorer window or tab
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ActiveExplorerState {
    hwnd: isize,
    folder_path: String,
    title: String,
    project_name: String,
    opened_at_epoch: u64,
    opened_instant: Instant,
    last_seen_instant: Instant,
    active_dwell_secs: f64,
    was_foreground_recently: bool,
}

/// State machine tracking Explorer lifecycles across sampling ticks
pub struct ExplorerLifecycleTracker {
    active_folders: HashMap<isize, ActiveExplorerState>,
    last_eval_instant: Instant,
}

impl ExplorerLifecycleTracker {
    pub fn new() -> Self {
        Self {
            active_folders: HashMap::new(),
            last_eval_instant: Instant::now(),
        }
    }

    /// Evaluates current open Explorer windows and detects open, dwell, and close transitions
    pub fn evaluate_snapshot(&mut self) -> Vec<ExplorerFolderInfo> {
        let now = Instant::now();
        let dt = now.duration_since(self.last_eval_instant).as_secs_f64().max(0.5);
        self.last_eval_instant = now;
        let now_epoch = now_epoch_secs();

        let fg_hwnd = unsafe { GetForegroundWindow().0 as isize };
        let current_open = resolve_open_explorer_folders();
        let mut current_hwnds = HashSet::new();

        // 1. Process active / newly opened folders
        for folder in &current_open {
            current_hwnds.insert(folder.hwnd);
            let is_fg = folder.hwnd == fg_hwnd;

            let entry = self.active_folders.entry(folder.hwnd).or_insert_with(|| {
                let rec = ExplorerLifecycleRecord {
                    id: 0,
                    tab_id: format!("hwnd_{}", folder.hwnd),
                    folder_path: folder.folder_path.clone(),
                    project_name: folder.project_name.clone(),
                    opened_at_epoch: now_epoch,
                    closed_at_epoch: None,
                    shelf_life_secs: 0.0,
                    active_dwell_secs: 0.0,
                    is_forgotten_loop: false,
                };

                // Persist initial open record
                if let Ok(store) = global_explorer_history().lock() {
                    let _ = store.record_folder_opened(&rec);
                }

                ActiveExplorerState {
                    hwnd: folder.hwnd,
                    folder_path: folder.folder_path.clone(),
                    title: folder.title.clone(),
                    project_name: folder.project_name.clone(),
                    opened_at_epoch: now_epoch,
                    opened_instant: now,
                    last_seen_instant: now,
                    active_dwell_secs: 0.0,
                    was_foreground_recently: is_fg,
                }
            });

            // Update metrics
            entry.last_seen_instant = now;
            if is_fg {
                entry.active_dwell_secs += dt;
                entry.was_foreground_recently = true;
            }
        }

        // 2. Identify closed windows
        let mut closed_hwnds = Vec::new();
        for (hwnd, state) in &self.active_folders {
            if !current_hwnds.contains(hwnd) {
                let shelf_life = state.opened_instant.elapsed().as_secs_f64();
                let is_forgotten = shelf_life > 5400.0 && state.active_dwell_secs < 60.0;

                let rec = ExplorerLifecycleRecord {
                    id: 0,
                    tab_id: format!("hwnd_{}", state.hwnd),
                    folder_path: state.folder_path.clone(),
                    project_name: state.project_name.clone(),
                    opened_at_epoch: state.opened_at_epoch,
                    closed_at_epoch: Some(now_epoch),
                    shelf_life_secs: shelf_life,
                    active_dwell_secs: state.active_dwell_secs,
                    is_forgotten_loop: is_forgotten,
                };

                // Persist closure to SQLite
                if let Ok(store) = global_explorer_history().lock() {
                    let _ = store.record_folder_closed(&rec);
                }

                closed_hwnds.push(*hwnd);
            }
        }

        for h in closed_hwnds {
            self.active_folders.remove(&h);
        }

        // Return current open folders enriched with dwell & shelf-life
        current_open
            .into_iter()
            .map(|mut f| {
                if let Some(state) = self.active_folders.get(&f.hwnd) {
                    f.active_dwell_secs = state.active_dwell_secs;
                    f.shelf_life_secs = state.opened_instant.elapsed().as_secs_f64();
                    f.is_forgotten_loop = f.shelf_life_secs > 5400.0 && f.active_dwell_secs < 60.0;
                }
                f
            })
            .collect()
    }
}

/// Global singleton for the Explorer lifecycle tracker
pub fn global_explorer_tracker() -> &'static Mutex<ExplorerLifecycleTracker> {
    static TRACKER: OnceLock<Mutex<ExplorerLifecycleTracker>> = OnceLock::new();
    TRACKER.get_or_init(|| Mutex::new(ExplorerLifecycleTracker::new()))
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
    fn test_decode_file_uri() {
        assert_eq!(
            decode_file_uri("file:///C:/Dev/MemoryBlast").unwrap(),
            r"C:\Dev\MemoryBlast"
        );
        assert_eq!(
            decode_file_uri("file:///C:/Users/titwa/Documents%20Folder").unwrap(),
            r"C:\Users\titwa\Documents Folder"
        );
        assert_eq!(
            decode_file_uri(r"C:\Dev\RustProject").unwrap(),
            r"C:\Dev\RustProject"
        );
    }

    #[test]
    fn test_clean_address_bar() {
        assert_eq!(
            clean_address_bar_text("Address: C:\\Dev\\MemoryBlast").unwrap(),
            r"C:\Dev\MemoryBlast"
        );
        assert_eq!(
            clean_address_bar_text("Adresse : D:\\Work").unwrap(),
            r"D:\Work"
        );
    }

    #[test]
    fn test_extract_project_name() {
        assert_eq!(extract_project_name(r"C:\Dev\MemoryBlast"), "MemoryBlast");
        assert_eq!(extract_project_name(r"D:\Projects\GameEngine\"), "GameEngine");
    }
}
