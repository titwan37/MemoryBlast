//! Task Resume Anchor ("Where Was I?")
//!
//! Captures developer working state before task transitions or long breaks:
//! - Active Git branch & last commit
//! - Most recently modified files & line numbers
//! - Last copied compiler error / reference from clipboard telemetry
//! - Open Chromium tabs
//! Generates a 2-line "Where you left off" orientation card and 1-click IDE deep links (vscode://).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::copilot::clipboard::global_clipboard;

/// Developer Working State Snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeAnchor {
    pub timestamp_ms: u64,
    pub git_branch: String,
    pub git_last_commit: String,
    pub active_file_path: Option<String>,
    pub active_line_number: Option<usize>,
    pub ide_launch_url: Option<String>,
    pub last_clipboard_snippet: Option<String>,
    pub primary_tab_title: Option<String>,
    pub total_active_tabs: usize,
    pub summary_line1: String,
    pub summary_line2: String,
}

impl ResumeAnchor {
    /// Captures the current developer environment state
    pub fn capture(workspace_path: &Path, active_tabs: &[(String, String)]) -> Self {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        // 1. Query Git Context
        let (git_branch, git_last_commit) = query_git_context(workspace_path);

        // 2. Find most recently modified file in workspace
        let (active_file_path, active_line_number) = find_most_recent_file(workspace_path);

        // 3. Generate IDE deep link URI (e.g. vscode://file/...)
        let ide_launch_url = active_file_path.as_ref().map(|path| {
            let line = active_line_number.unwrap_or(1);
            let normalized = path.replace('\\', "/");
            format!("vscode://file/{}:{}", normalized, line)
        });

        // 4. Query last copied clipboard snippet
        let last_clipboard_snippet = {
            let clip_lock = global_clipboard().lock().ok();
            clip_lock.and_then(|mgr| mgr.get_last_snippet().map(|e| e.content))
        };

        // 5. Tabs context
        let total_active_tabs = active_tabs.len();
        let primary_tab_title = active_tabs.first().map(|(t, _)| t.clone());

        // 6. 2-line "Where you left off" orientation card
        let file_label = if let Some(ref p) = active_file_path {
            let file_name = Path::new(p)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file");
            format!("{} (line {})", file_name, active_line_number.unwrap_or(1))
        } else {
            "no recent edits".to_string()
        };

        let summary_line1 = format!(
            "⚓ You were working on branch '{}' in {}",
            git_branch, file_label
        );

        let summary_line2 = if let Some(ref clip) = last_clipboard_snippet {
            let clip_preview = if clip.len() > 60 {
                format!("{}...", &clip[..60].replace('\n', " "))
            } else {
                clip.replace('\n', " ")
            };
            format!("Last reference: \"{}\"", clip_preview)
        } else if let Some(ref tab) = primary_tab_title {
            let tab_preview = if tab.len() > 60 {
                format!("{}...", &tab[..60])
            } else {
                tab.clone()
            };
            format!("Active reference tab: \"{}\"", tab_preview)
        } else {
            format!("Commit: \"{}\"", git_last_commit)
        };

        let anchor = Self {
            timestamp_ms: now_ms,
            git_branch,
            git_last_commit,
            active_file_path,
            active_line_number,
            ide_launch_url,
            last_clipboard_snippet,
            primary_tab_title,
            total_active_tabs,
            summary_line1,
            summary_line2,
        };

        // Persist latest anchor to disk
        let _ = save_latest_anchor(&anchor, Path::new("memoryblast_data"));

        anchor
    }

    /// Renders a formatted terminal orientation card
    pub fn render_terminal_card(&self) -> String {
        let border = "═".repeat(68);
        let mut out = String::new();
        out.push_str(&format!("╔{}╗\n", border));
        out.push_str("║ ⚓ FOCUS COPILOT — WHERE YOU LEFT OFF                              ║\n");
        out.push_str(&format!("╠{}╣\n", border));
        out.push_str(&format!("║ {:<66} ║\n", truncate_str(&self.summary_line1, 66)));
        out.push_str(&format!("║ {:<66} ║\n", truncate_str(&self.summary_line2, 66)));

        if let Some(ref url) = self.ide_launch_url {
            out.push_str(&format!("║ Launch in IDE: {:<49} ║\n", truncate_str(url, 49)));
        }

        out.push_str(&format!("╚{}╝\n", border));
        out
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

/// Queries Git branch and last commit subject
pub fn query_git_context(dir: &Path) -> (String, String) {
    let branch = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(dir)
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                String::from_utf8(out.stdout).ok().map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "main".to_string());

    let commit = Command::new("git")
        .args(["log", "-1", "--pretty=%s"])
        .current_dir(dir)
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                String::from_utf8(out.stdout).ok().map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "Initial commit".to_string());

    (branch, commit)
}

/// Finds the most recently modified source file in the workspace
pub fn find_most_recent_file(dir: &Path) -> (Option<String>, Option<usize>) {
    // 1. Try git status --porcelain to find dirty files
    if let Ok(out) = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(dir)
        .output()
    {
        if out.status.success() {
            if let Ok(text) = String::from_utf8(out.stdout) {
                for line in text.lines() {
                    let trimmed = line.trim();
                    if trimmed.len() > 3 {
                        let rel_path = &trimmed[3..];
                        let full_path = dir.join(rel_path);
                        if full_path.is_file() {
                            return (Some(full_path.to_string_lossy().to_string()), Some(1));
                        }
                    }
                }
            }
        }
    }

    // 2. Fallback: inspect src/ directory for newest file MTime
    let src_dir = dir.join("src");
    if let Ok(entries) = fs::read_dir(&src_dir) {
        let mut newest_file = None;
        let mut newest_time = SystemTime::UNIX_EPOCH;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Ok(meta) = path.metadata() {
                    if let Ok(mtime) = meta.modified() {
                        if mtime > newest_time {
                            newest_time = mtime;
                            newest_file = Some(path.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }

        if let Some(f) = newest_file {
            return (Some(f), Some(1));
        }
    }

    (None, None)
}

/// Persists latest resume anchor to disk
pub fn save_latest_anchor(anchor: &ResumeAnchor, storage_dir: &Path) -> Result<(), std::io::Error> {
    let _ = fs::create_dir_all(storage_dir);
    let path = storage_dir.join("latest_anchor.json");
    let json = serde_json::to_string_pretty(anchor)?;
    fs::write(path, json)?;
    Ok(())
}

/// Loads latest resume anchor from disk
pub fn load_latest_anchor(storage_dir: &Path) -> Option<ResumeAnchor> {
    let path = storage_dir.join("latest_anchor.json");
    let data = fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resume_anchor_creation() {
        let current_dir = std::env::current_dir().unwrap();
        let anchor = ResumeAnchor::capture(&current_dir, &[("Rust Documentation".to_string(), "https://docs.rs".to_string())]);

        assert!(!anchor.git_branch.is_empty());
        assert!(anchor.summary_line1.contains("You were working on branch"));
        assert!(anchor.total_active_tabs >= 1);

        let card = anchor.render_terminal_card();
        assert!(card.contains("WHERE YOU LEFT OFF"));
    }
}
