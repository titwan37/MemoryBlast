use std::collections::HashMap;
use std::time::Duration;
use serde::Deserialize;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
};

#[derive(Debug, Clone, Deserialize)]
pub struct CdpTabInfo {
    #[allow(dead_code)]
    pub id: Option<String>,
    pub title: Option<String>,
    pub url: Option<String>,
    #[serde(rename = "type")]
    pub target_type: Option<String>,
}

/// Known Chromium browser binary names (case-insensitive)
pub const CHROMIUM_BINARIES: &[(&str, &str)] = &[
    ("chrome.exe", "Google Chrome"),
    ("msedge.exe", "Microsoft Edge"),
    ("brave.exe", "Brave Browser"),
    ("arc.exe", "Arc Browser"),
    ("opera.exe", "Opera"),
    ("vivaldi.exe", "Vivaldi"),
];

/// Checks if a process executable name belongs to a supported Chromium browser.
pub fn get_chromium_browser_name(exe_name: &str) -> Option<&'static str> {
    let lower = exe_name.to_lowercase();
    for (bin, friendly) in CHROMIUM_BINARIES {
        if lower == *bin {
            return Some(*friendly);
        }
    }
    None
}

/// Context used during EnumWindows callback.
struct EnumWindowsContext {
    titles_by_pid: HashMap<u32, Vec<String>>,
}

unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctx = &mut *(lparam.0 as *mut EnumWindowsContext);

    if !IsWindowVisible(hwnd).as_bool() {
        return BOOL(1);
    }

    let length = GetWindowTextLengthW(hwnd);
    if length <= 0 {
        return BOOL(1);
    }

    let mut buffer: Vec<u16> = vec![0u16; (length + 1) as usize];
    let read_len = GetWindowTextW(hwnd, &mut buffer);
    if read_len > 0 {
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid != 0 {
            let raw_title = String::from_utf16_lossy(&buffer[..read_len as usize]);
            let cleaned = clean_browser_title(&raw_title);
            if !cleaned.is_empty() {
                ctx.titles_by_pid.entry(pid).or_default().push(cleaned);
            }
        }
    }

    BOOL(1)
}

/// Enumerates visible top-level windows and returns a map of PID to window titles.
pub fn get_window_titles_by_pid() -> HashMap<u32, Vec<String>> {
    let mut ctx = EnumWindowsContext {
        titles_by_pid: HashMap::new(),
    };

    unsafe {
        let _ = EnumWindows(
            Some(enum_windows_proc),
            LPARAM(&mut ctx as *mut _ as isize),
        );
    }

    ctx.titles_by_pid
}

/// Strips common browser suffixes from window titles for clean tab display.
fn clean_browser_title(title: &str) -> String {
    let suffixes = [
        " - Google Chrome",
        " - Personal - Microsoft​ Edge",
        " - Work - Microsoft​ Edge",
        " - Microsoft​ Edge",
        " - Microsoft Edge",
        " - Brave",
        " - Arc",
        " - Opera",
        " - Vivaldi",
    ];

    let mut cleaned = title.trim();
    for suffix in suffixes {
        if let Some(stripped) = cleaned.strip_suffix(suffix) {
            cleaned = stripped.trim();
            break;
        }
    }

    cleaned.to_string()
}

/// Queries Chrome DevTools Protocol (CDP) `/json/list` asynchronously if a debug port is open.
/// This is non-destructive: it only queries the existing port and never restarts or alters the browser.
pub async fn query_cdp_tabs(port: u16) -> Vec<CdpTabInfo> {
    let url = format!("http://127.0.0.1:{port}/json/list");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(250))
        .build();

    let Ok(client) = client else {
        return Vec::new();
    };

    let response = client.get(&url).send().await;
    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                resp.json::<Vec<CdpTabInfo>>().await.unwrap_or_default()
            } else {
                Vec::new()
            }
        }
        Err(_) => Vec::new(),
    }
}
