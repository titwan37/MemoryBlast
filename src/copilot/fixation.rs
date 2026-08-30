//! Rabbit Hole & Fixation Loop Detector
//!
//! Identifies divergent tab sprawl (rapid opening of tangential domains)
//! and developer build failure doom loops (repeated failing commands).
//! Surfaces the "Curiosity Drawer" to park rabbit holes without guilt or distraction.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::proc::ProcessItem;

/// Detected Rabbit Hole Alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RabbitHoleAlert {
    pub burst_count: usize,
    pub divergent_titles: Vec<String>,
    pub suggested_drawer_name: String,
    pub suggested_prompt: String,
}

/// Detected Build Failure Fixation Alert
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildFixationAlert {
    pub consecutive_failures: usize,
    pub command_name: String,
    pub duration_secs: u64,
    pub suggested_prompt: String,
}

/// State machine tracking navigation streams and build cycles
pub struct FixationDetector {
    known_tab_titles: VecDeque<(String, Instant)>,
    #[allow(dead_code)]
    build_failures: VecDeque<(String, Instant)>,
    last_rabbit_hole_alert: Option<Instant>,
    #[allow(dead_code)]
    last_build_alert: Option<Instant>,
    drawer_dir: PathBuf,
}

impl FixationDetector {
    pub fn new(storage_dir: &Path) -> Self {
        let drawer_dir = storage_dir.join("curiosity_drawers");
        let _ = fs::create_dir_all(&drawer_dir);

        Self {
            known_tab_titles: VecDeque::new(),
            build_failures: VecDeque::new(),
            last_rabbit_hole_alert: None,
            last_build_alert: None,
            drawer_dir,
        }
    }

    /// Evaluates current active Chromium tabs to detect sudden bursts of tangent rabbit holes
    pub fn evaluate_tabs(&mut self, processes: &[ProcessItem]) -> Option<RabbitHoleAlert> {
        let now = Instant::now();
        // Keep 10 minutes of history
        self.known_tab_titles.retain(|(_, t)| now.duration_since(*t) < Duration::from_secs(600));

        let mut newly_seen = Vec::new();
        for p in processes {
            if p.is_chromium && !p.display_title.is_empty() {
                let title = p.display_title.trim();
                let already_known = self.known_tab_titles.iter().any(|(t, _)| t == title);
                if !already_known {
                    self.known_tab_titles.push_back((title.to_string(), now));
                    newly_seen.push(title.to_string());
                }
            }
        }

        // Identify divergent / tangent tabs
        let mut divergent = Vec::new();
        for title in &newly_seen {
            if is_tangent_topic(title) {
                divergent.push(title.clone());
            }
        }

        // Trigger rabbit hole alert if >= 4 tangent tabs opened within the recent window
        if divergent.len() >= 4 {
            let can_alert = self
                .last_rabbit_hole_alert
                .map(|t| now.duration_since(t) >= Duration::from_secs(300))
                .unwrap_or(true);

            if can_alert {
                self.last_rabbit_hole_alert = Some(now);
                let drawer_name = format!(
                    "RabbitHole_{}",
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                );

                return Some(RabbitHoleAlert {
                    burst_count: divergent.len(),
                    divergent_titles: divergent,
                    suggested_drawer_name: drawer_name,
                    suggested_prompt: "Deep into a rabbit hole? [ 📦 Stash into Curiosity Drawer ] [ 🎯 Back to Code ]".to_string(),
                });
            }
        }

        None
    }

    /// Records a build/command failure event (e.g. exit code != 0 from cargo or test runner)
    #[allow(dead_code)]
    pub fn record_command_failure(&mut self, command: &str) -> Option<BuildFixationAlert> {
        let now = Instant::now();
        self.build_failures.retain(|(_, t)| now.duration_since(*t) < Duration::from_secs(300));
        self.build_failures.push_back((command.to_string(), now));

        let failure_count = self
            .build_failures
            .iter()
            .filter(|(cmd, _)| cmd == command)
            .count();

        if failure_count >= 3 {
            let can_alert = self
                .last_build_alert
                .map(|t| now.duration_since(t) >= Duration::from_secs(300))
                .unwrap_or(true);

            if can_alert {
                self.last_build_alert = Some(now);
                return Some(BuildFixationAlert {
                    consecutive_failures: failure_count,
                    command_name: command.to_string(),
                    duration_secs: 300,
                    suggested_prompt: format!(
                        "Command '{}' failed {} times in a row. [ 🦆 Rubber Duck ] [ 🔍 Inspect Git Diff ] [ ⏸ 5m Pause ]",
                        command, failure_count
                    ),
                });
            }
        }

        None
    }

    /// Sweeps tangent tabs into a dedicated Curiosity Drawer Markdown file
    pub fn stash_into_curiosity_drawer(
        &self,
        drawer_name: &str,
        tabs: &[(String, String)], // (title, url)
    ) -> Result<PathBuf, std::io::Error> {
        let safe_name = drawer_name.replace(|c: char| !c.is_alphanumeric() && c != '_', "_");
        let file_path = self.drawer_dir.join(format!("{}.md", safe_name));

        let now_str = chrono_like_now();
        let mut content = format!(
            "# 🐇 Curiosity Drawer: {}\n\n*Saved on {}*\n*Stored here guilt-free so you can return to deep focus.*\n\n---\n\n",
            drawer_name, now_str
        );

        for (title, url) in tabs {
            content.push_str(&format!("- [ ] [{}]({})\n", title, url));
        }

        content.push_str("\n---\n*Preserved by MemoryBlast Focus Copilot.*\n");
        fs::write(&file_path, content)?;

        Ok(file_path)
    }
}

/// Global singleton for fixation detection
pub fn global_fixation() -> &'static Mutex<FixationDetector> {
    static FIXATION: OnceLock<Mutex<FixationDetector>> = OnceLock::new();
    FIXATION.get_or_init(|| {
        let dir = Path::new("memoryblast_data");
        Mutex::new(FixationDetector::new(dir))
    })
}

/// Determines if a window title or search query represents a tangent topic
pub fn is_tangent_topic(title: &str) -> bool {
    let lower = title.to_lowercase();

    // Primary dev / work indicators
    let work_indicators = [
        "rust", "cargo", "github", "gitlab", "pull request", "commit", "diff",
        "docs.rs", "stackoverflow", "crates.io", "api", "sqlite", "windows",
        "terminal", "powershell", "debug", "compiler", "error[", "localhost",
    ];
    for w in &work_indicators {
        if lower.contains(w) {
            return false;
        }
    }

    // Tangent / entertainment / rabbit hole indicators
    let tangent_indicators = [
        "wikipedia", "reddit", "youtube", "twitter", "x.com", "hacker news",
        "imdb", "tv", "movie", "review", "forum", "wiki", "fandom", "news",
        "buy", "amazon", "price", "deal", "gaming", "steam",
    ];
    for t in &tangent_indicators {
        if lower.contains(t) {
            return true;
        }
    }

    // Generic search query indicating topic hopping
    if lower.contains("- google search") || lower.contains("- bing") || lower.contains("- duckduckgo") {
        return true;
    }

    false
}

fn chrono_like_now() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let hours = (now / 3600) % 24;
    let mins = (now / 60) % 60;
    format!("{now} (UTC {hours:02}:{mins:02})")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tangent_topic_classification() {
        assert!(is_tangent_topic("History of the Byzantine Empire - Wikipedia"));
        assert!(is_tangent_topic("Best mechanical keyboards 2026 - Reddit"));
        assert!(is_tangent_topic("Funny cat moments - YouTube"));

        assert!(!is_tangent_topic("rusqlite::Connection - Rust docs.rs"));
        assert!(!is_tangent_topic("Fix lifetime error on Mutex in Rust - Stack Overflow"));
        assert!(!is_tangent_topic("PR #42: Add Copilot Engine · titwan37/MemoryBlast GitHub"));
    }

    #[test]
    fn test_command_failure_loop() {
        let temp_dir = std::env::temp_dir().join(format!("test_fix_{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
        let mut detector = FixationDetector::new(&temp_dir);

        let a1 = detector.record_command_failure("cargo test");
        assert!(a1.is_none());

        let a2 = detector.record_command_failure("cargo test");
        assert!(a2.is_none());

        let a3 = detector.record_command_failure("cargo test");
        assert!(a3.is_some());
        let alert = a3.unwrap();
        assert_eq!(alert.consecutive_failures, 3);
        assert!(alert.suggested_prompt.contains("Rubber Duck"));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
