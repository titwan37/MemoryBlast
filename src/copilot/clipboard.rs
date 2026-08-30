//! Clipboard Telemetry & Thrashing Engine
//!
//! Registers a non-polling Win32 clipboard format listener (AddClipboardFormatListener),
//! stores historical clipboard events in SQLite with automatic secret redaction,
//! classifies snippets (StackTrace, CodeSnippet, ShellCommand, URL, GenericText),
//! and detects frustration / doom loops (repeated copying of identical errors).

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use windows::Win32::Foundation::{BOOL, HANDLE, HWND};
use windows::Win32::UI::WindowsAndMessaging::MSG;

#[link(name = "user32")]
extern "system" {
    fn AddClipboardFormatListener(hwnd: HWND) -> BOOL;
    fn RemoveClipboardFormatListener(hwnd: HWND) -> BOOL;
    fn OpenClipboard(hwnd: HWND) -> BOOL;
    fn CloseClipboard() -> BOOL;
    fn GetClipboardData(uformat: u32) -> HANDLE;
    fn IsClipboardFormatAvailable(format: u32) -> BOOL;
    fn CreateWindowExW(
        dwexstyle: u32,
        lpclassname: *const u16,
        lpwindowname: *const u16,
        dwstyle: u32,
        x: i32,
        y: i32,
        nwidth: i32,
        nheight: i32,
        hwndparent: HWND,
        hmenu: usize,
        hinstance: usize,
        lpparam: *mut core::ffi::c_void,
    ) -> HWND;
    fn DestroyWindow(hwnd: HWND) -> BOOL;
    fn GetMessageW(lpmsg: *mut MSG, hwnd: HWND, wmsgfiltermin: u32, wmsgfiltermax: u32) -> BOOL;
    fn TranslateMessage(lpmsg: *const MSG) -> BOOL;
    fn DispatchMessageW(lpmsg: *const MSG) -> isize;
}

#[link(name = "kernel32")]
extern "system" {
    fn GlobalLock(hmem: HANDLE) -> *mut u16;
    fn GlobalUnlock(hmem: HANDLE) -> BOOL;
    #[allow(dead_code)]
    fn GlobalSize(hmem: HANDLE) -> usize;
}

const CF_UNICODETEXT: u32 = 13;
const WM_CLIPBOARDUPDATE: u32 = 0x031D;

/// Category of copied snippet
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClipboardCategory {
    StackTrace,
    CodeSnippet,
    ShellCommand,
    URL,
    GenericText,
}

impl ClipboardCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::StackTrace => "StackTrace",
            Self::CodeSnippet => "CodeSnippet",
            Self::ShellCommand => "ShellCommand",
            Self::URL => "URL",
            Self::GenericText => "GenericText",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "StackTrace" => Self::StackTrace,
            "CodeSnippet" => Self::CodeSnippet,
            "ShellCommand" => Self::ShellCommand,
            "URL" => Self::URL,
            _ => Self::GenericText,
        }
    }
}

/// Recorded clipboard event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardEvent {
    pub id: i64,
    pub timestamp_ms: u64,
    pub content: String,
    pub char_count: usize,
    pub category: ClipboardCategory,
    pub was_redacted: bool,
}

/// Frustration Loop alert state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrustrationAlert {
    pub repeated_text_snippet: String,
    pub repeat_count: usize,
    pub duration_secs: u64,
    pub suggested_prompt: String,
}

/// Manager handling SQLite persistence and in-memory frustration tracking
pub struct ClipboardManager {
    db_path: PathBuf,
    recent_copies: VecDeque<(String, Instant)>,
    last_frustration_alert: Option<Instant>,
    last_snippet: Option<ClipboardEvent>,
}

impl ClipboardManager {
    pub fn new(storage_dir: &Path) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        std::fs::create_dir_all(storage_dir)?;
        let db_path = storage_dir.join("copilot_clipboard.db");

        let conn = Connection::open(&db_path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS clipboard_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp_ms INTEGER NOT NULL,
                content TEXT NOT NULL,
                char_count INTEGER NOT NULL,
                category TEXT NOT NULL,
                was_redacted INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_clipboard_ts ON clipboard_events(timestamp_ms DESC)",
            [],
        )?;

        Ok(Self {
            db_path,
            recent_copies: VecDeque::new(),
            last_frustration_alert: None,
            last_snippet: None,
        })
    }

    /// Records a new clipboard copy, applies secret redaction, classifies, and checks for frustration loops
    pub fn record_copy(&mut self, raw_text: &str) -> (ClipboardEvent, Option<FrustrationAlert>) {
        let trimmed = raw_text.trim();
        let (sanitized, was_redacted) = redact_secrets(trimmed);
        let category = classify_content(&sanitized);
        let char_count = sanitized.chars().count();

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let mut event_id = 0;
        if let Ok(conn) = Connection::open(&self.db_path) {
            let res = conn.execute(
                "INSERT INTO clipboard_events (timestamp_ms, content, char_count, category, was_redacted)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    now_ms as i64,
                    sanitized,
                    char_count as i64,
                    category.as_str(),
                    if was_redacted { 1 } else { 0 }
                ],
            );
            if res.is_ok() {
                event_id = conn.last_insert_rowid();
            }
        }

        let event = ClipboardEvent {
            id: event_id,
            timestamp_ms: now_ms,
            content: sanitized.clone(),
            char_count,
            category: category.clone(),
            was_redacted,
        };

        self.last_snippet = Some(event.clone());

        // Check for Frustration Loop (repeated identical/similar error copies in 10 minutes)
        let now = Instant::now();
        self.recent_copies.retain(|(_, t)| now.duration_since(*t) < Duration::from_secs(600));

        // Use a 120-char fingerprint for matching repeated error traces
        let fingerprint = if sanitized.len() > 120 {
            sanitized[..120].to_string()
        } else {
            sanitized.clone()
        };

        self.recent_copies.push_back((fingerprint.clone(), now));

        let mut frustration_alert = None;
        if category == ClipboardCategory::StackTrace || category == ClipboardCategory::CodeSnippet {
            let matching_count = self
                .recent_copies
                .iter()
                .filter(|(fp, _)| fp == &fingerprint)
                .count();

            if matching_count >= 3 {
                // Throttle alerts to once per 5 minutes
                let can_alert = self
                    .last_frustration_alert
                    .map(|t| now.duration_since(t) >= Duration::from_secs(300))
                    .unwrap_or(true);

                if can_alert {
                    self.last_frustration_alert = Some(now);
                    let snippet_preview = if sanitized.len() > 80 {
                        format!("{}...", &sanitized[..80])
                    } else {
                        sanitized.clone()
                    };

                    frustration_alert = Some(FrustrationAlert {
                        repeated_text_snippet: snippet_preview,
                        repeat_count: matching_count,
                        duration_secs: 600,
                        suggested_prompt: "Stuck on this compiler/runtime error? [ 💡 Rubber Duck Prompt ] [ ⏸ Take a 5m breather ]".to_string(),
                    });
                }
            }
        }

        (event, frustration_alert)
    }

    /// Returns recent clipboard events
    pub fn get_recent(&self, limit: usize) -> Vec<ClipboardEvent> {
        let mut events = Vec::new();
        if let Ok(conn) = Connection::open(&self.db_path) {
            let mut stmt = conn
                .prepare(
                    "SELECT id, timestamp_ms, content, char_count, category, was_redacted
                     FROM clipboard_events
                     ORDER BY timestamp_ms DESC
                     LIMIT ?1",
                )
                .unwrap();

            let rows = stmt.query_map(params![limit as i64], |row| {
                let cat_str: String = row.get(4)?;
                Ok(ClipboardEvent {
                    id: row.get(0)?,
                    timestamp_ms: row.get::<_, i64>(1)? as u64,
                    content: row.get(2)?,
                    char_count: row.get::<_, i64>(3)? as usize,
                    category: ClipboardCategory::from_str(&cat_str),
                    was_redacted: row.get::<_, i32>(5)? != 0,
                })
            });

            if let Ok(iter) = rows {
                for item in iter.flatten() {
                    events.push(item);
                }
            }
        }
        events
    }

    /// Gets the most recently copied snippet
    pub fn get_last_snippet(&self) -> Option<ClipboardEvent> {
        self.last_snippet.clone()
    }

    /// Returns how many frustration-triggering repeated clips exist in the current tracking window
    pub fn recent_frustration_count(&self) -> usize {
        let now = Instant::now();
        let mut fingerprint_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for (fp, t) in &self.recent_copies {
            if now.duration_since(*t) < Duration::from_secs(600) {
                *fingerprint_counts.entry(fp.as_str()).or_default() += 1;
            }
        }
        fingerprint_counts.values().filter(|&&c| c >= 3).count()
    }
}

/// Global singleton for clipboard telemetry
pub fn global_clipboard() -> &'static Mutex<ClipboardManager> {
    static CLIPBOARD: OnceLock<Mutex<ClipboardManager>> = OnceLock::new();
    CLIPBOARD.get_or_init(|| {
        let dir = Path::new("memoryblast_data");
        let mgr = ClipboardManager::new(dir).unwrap_or_else(|_| {
            let fallback_dir = std::env::temp_dir().join("memoryblast_data");
            ClipboardManager::new(&fallback_dir).expect("Failed to initialize ClipboardManager")
        });
        Mutex::new(mgr)
    })
}

/// Redacts secrets (API keys, passwords, tokens, private keys)
pub fn redact_secrets(input: &str) -> (String, bool) {
    let mut redacted = input.to_string();
    let mut modified = false;

    // 1. GitHub personal access tokens
    let gh_patterns = ["ghp_", "gho_", "ghu_", "ghs_", "ghr_"];
    for prefix in &gh_patterns {
        while let Some(start) = redacted.find(prefix) {
            let rest = &redacted[start..];
            let end_offset = rest.find(|c: char| !c.is_alphanumeric() && c != '_').unwrap_or(rest.len());
            if end_offset >= 30 {
                let secret = &redacted[start..start + end_offset];
                redacted = redacted.replace(secret, "[REDACTED_GITHUB_TOKEN]");
                modified = true;
            } else {
                break;
            }
        }
    }

    // 2. Bearer tokens
    if let Some(pos) = redacted.to_lowercase().find("bearer ") {
        let rest = &redacted[pos + 7..];
        let token_len = rest.find(|c: char| c.is_whitespace() || c == '"' || c == '\'').unwrap_or(rest.len());
        if token_len > 15 {
            let secret = &redacted[pos..pos + 7 + token_len];
            redacted = redacted.replace(secret, "Bearer [REDACTED_BEARER_TOKEN]");
            modified = true;
        }
    }

    // 3. AWS Access Keys
    for prefix in &["AKIA", "ASIA"] {
        if let Some(pos) = redacted.find(prefix) {
            if pos + 20 <= redacted.len() {
                let candidate = &redacted[pos..pos + 20];
                if candidate.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) {
                    redacted = redacted.replace(candidate, "[REDACTED_AWS_KEY]");
                    modified = true;
                }
            }
        }
    }

    // 4. Private keys
    if redacted.contains("BEGIN PRIVATE KEY") || redacted.contains("BEGIN RSA PRIVATE KEY") {
        redacted = "[REDACTED_PRIVATE_KEY]".to_string();
        modified = true;
    }

    (redacted, modified)
}

/// Classifies text into semantic categories
pub fn classify_content(text: &str) -> ClipboardCategory {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return ClipboardCategory::GenericText;
    }

    // URL
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        if !trimmed.contains('\n') && !trimmed.contains(' ') {
            return ClipboardCategory::URL;
        }
    }

    // Stack Trace / Errors
    let stack_trace_indicators = [
        "error[E",
        "error: ",
        "panicked at",
        "stack backtrace:",
        "Traceback (most recent call last):",
        "NullPointerException",
        "TypeError:",
        "UnhandledPromiseRejection",
        "Exception in thread",
        "at System.",
        "at async ",
        "warning: ",
    ];
    for ind in &stack_trace_indicators {
        if trimmed.contains(ind) {
            return ClipboardCategory::StackTrace;
        }
    }

    // Shell Commands
    let shell_prefixes = [
        "cargo ", "git ", "npm ", "pnpm ", "yarn ", "docker ", "kubectl ",
        "python ", "pip ", "rustc ", "cd ", "ls ", "dir ", "pwsh ", "powershell ",
    ];
    for prefix in &shell_prefixes {
        if trimmed.starts_with(prefix) && !trimmed.contains('\n') {
            return ClipboardCategory::ShellCommand;
        }
    }

    // Code Snippets
    let code_keywords = [
        "fn ", "pub ", "struct ", "impl ", "let ", "const ", "def ", "class ",
        "import ", "return ", "async ", "await ", "match ", "interface ", "function ",
    ];
    let mut keyword_hits = 0;
    for kw in &code_keywords {
        if trimmed.contains(kw) {
            keyword_hits += 1;
        }
    }
    if keyword_hits >= 2 || (trimmed.contains('{') && trimmed.contains('}')) || (trimmed.contains(';') && trimmed.contains('\n')) {
        return ClipboardCategory::CodeSnippet;
    }

    ClipboardCategory::GenericText
}

/// Reads Unicode text from the current Windows clipboard
pub fn read_clipboard_text() -> Option<String> {
    unsafe {
        let null_hwnd = HWND(std::ptr::null_mut());
        if !IsClipboardFormatAvailable(CF_UNICODETEXT).as_bool() {
            return None;
        }
        if !OpenClipboard(null_hwnd).as_bool() {
            return None;
        }

        let handle = GetClipboardData(CF_UNICODETEXT);
        if handle.0.is_null() {
            let _ = CloseClipboard();
            return None;
        }

        let ptr = GlobalLock(handle);
        if ptr.is_null() {
            let _ = CloseClipboard();
            return None;
        }

        let mut len = 0;
        while *ptr.add(len) != 0 {
            len += 1;
        }

        let slice = std::slice::from_raw_parts(ptr, len);
        let text = String::from_utf16_lossy(slice);

        let _ = GlobalUnlock(handle);
        let _ = CloseClipboard();

        if text.trim().is_empty() {
            None
        } else {
            Some(text)
        }
    }
}

/// Spawns the dedicated Win32 background thread with AddClipboardFormatListener
pub fn spawn_clipboard_listener(
    on_event: Arc<dyn Fn(ClipboardEvent, Option<FrustrationAlert>) + Send + Sync + 'static>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::thread::Builder::new()
        .name("copilot-clipboard".to_string())
        .spawn(move || unsafe {
            let static_class: Vec<u16> = "STATIC\0".encode_utf16().collect();

            let hwnd = CreateWindowExW(
                0,
                static_class.as_ptr(),
                std::ptr::null(),
                0,
                0,
                0,
                0,
                0,
                HWND(std::ptr::null_mut()),
                0,
                0,
                std::ptr::null_mut(),
            );

            if hwnd.0.is_null() {
                eprintln!("⚠️ Notice: Could not create clipboard listener window");
                return;
            }

            let listener_registered = AddClipboardFormatListener(hwnd);
            if !listener_registered.as_bool() {
                eprintln!("⚠️ Notice: Could not register AddClipboardFormatListener");
                let _ = DestroyWindow(hwnd);
                return;
            }

            println!("📋 Focus Copilot: Win32 Clipboard format listener active (secret-safe, zero-polling).");

            let mut msg = MSG::default();
            while GetMessageW(&mut msg, hwnd, 0, 0).as_bool() {
                if msg.message == WM_CLIPBOARDUPDATE {
                    if let Some(text) = read_clipboard_text() {
                        let (event, frustration) = {
                            let mut mgr = global_clipboard().lock().unwrap();
                            mgr.record_copy(&text)
                        };
                        on_event(event, frustration);
                    }
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            let _ = RemoveClipboardFormatListener(hwnd);
            let _ = DestroyWindow(hwnd);
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_redaction() {
        let text_with_github = "Check out this token: ghp_1234567890abcdef1234567890abcdef1234 for repo";
        let (redacted, modified) = redact_secrets(text_with_github);
        assert!(modified);
        assert!(redacted.contains("[REDACTED_GITHUB_TOKEN]"));
        assert!(!redacted.contains("ghp_1234567890"));

        let text_with_bearer = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.xyz123abc456";
        let (redacted_b, modified_b) = redact_secrets(text_with_bearer);
        assert!(modified_b);
        assert!(redacted_b.contains("Bearer [REDACTED_BEARER_TOKEN]"));

        let text_clean = "cargo check --release";
        let (clean, clean_mod) = redact_secrets(text_clean);
        assert!(!clean_mod);
        assert_eq!(clean, "cargo check --release");
    }

    #[test]
    fn test_classification() {
        assert_eq!(classify_content("https://docs.rs/rusqlite"), ClipboardCategory::URL);
        assert_eq!(
            classify_content("error[E0308]: mismatched types\n --> src/main.rs:12:5"),
            ClipboardCategory::StackTrace
        );
        assert_eq!(classify_content("cargo run --bin memoryblast"), ClipboardCategory::ShellCommand);
        assert_eq!(
            classify_content("pub fn test_fn() -> Result<(), String> { Ok(()) }"),
            ClipboardCategory::CodeSnippet
        );
        assert_eq!(classify_content("Meeting at 3pm tomorrow"), ClipboardCategory::GenericText);
    }

    #[test]
    fn test_clipboard_manager_and_frustration() {
        let temp_dir = std::env::temp_dir().join(format!("test_clip_{}", std::time::SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
        let mut mgr = ClipboardManager::new(&temp_dir).unwrap();

        let error = "error[E0425]: cannot find value `foo` in this scope\n --> src/copilot.rs:45:9";
        let (_, f1) = mgr.record_copy(error);
        assert!(f1.is_none());

        let (_, f2) = mgr.record_copy(error);
        assert!(f2.is_none());

        let (_, f3) = mgr.record_copy(error);
        assert!(f3.is_some());
        let alert = f3.unwrap();
        assert_eq!(alert.repeat_count, 3);
        assert!(alert.suggested_prompt.contains("Rubber Duck"));

        let recent = mgr.get_recent(5);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].category, ClipboardCategory::StackTrace);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
