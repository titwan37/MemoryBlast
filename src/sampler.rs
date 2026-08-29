use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use crossbeam_channel::{Receiver, Sender};
use tokio::sync::Notify;
use tokio::time::sleep;

use crate::chromium::{
    get_browser_tab_titles_via_uia, get_window_titles_by_pid, query_cdp_tabs,
};
use crate::proc::{ProcessItem, ProcessScanner};
use crate::sys::{get_system_memory_stats, SystemMemoryStats};

#[derive(Debug, Clone, Serialize)]
pub struct SystemSnapshot {
    pub mem_stats: SystemMemoryStats,
    pub processes: Vec<ProcessItem>,
    pub timestamp_ms: u64,
    #[serde(skip)]
    pub timestamp: Instant,
}

pub enum SamplerCommand {
    SetInterval(Duration),
    ForceRefresh,
    RetriggerTabDetection,
}

/// Spawns the decoupled 2-tier process sampling engine:
/// Tier 1: Ultra-fast 500ms memory & CPU loop (<10ms execution time, zero blocking).
/// Tier 2: Async background title enrichment worker (populates tab titles without stalling memory updates).
pub fn spawn_sampler(
    snapshot_tx: Sender<SystemSnapshot>,
    cmd_rx: Receiver<SamplerCommand>,
    mut interval_duration: Duration,
) {
    // Shared thread-safe caches for titles
    let window_titles_cache: Arc<RwLock<HashMap<u32, Vec<String>>>> =
        Arc::new(RwLock::new(HashMap::new()));
    let tab_titles_cache: Arc<RwLock<HashMap<u32, Vec<String>>>> =
        Arc::new(RwLock::new(HashMap::new()));

    // Shared list of active Chromium root PIDs & all child PIDs for the background worker to inspect
    // Tuple: (root_pid, Option<debug_port>, Vec<all_pids_in_family>)
    let active_chromium_pids: Arc<RwLock<Vec<(u32, Option<u16>, Vec<u32>)>>> =
        Arc::new(RwLock::new(Vec::new()));

    // Notify handle to trigger immediate re-detection
    let retrigger_notify = Arc::new(Notify::new());

    // -------------------------------------------------------------
    // TIER 2: ASYNC TITLE ENRICHMENT WORKER (Background Pipeline)
    // -------------------------------------------------------------
    let win_cache_worker = window_titles_cache.clone();
    let tab_cache_worker = tab_titles_cache.clone();
    let pids_worker = active_chromium_pids.clone();
    let notify_worker = retrigger_notify.clone();

    tokio::spawn(async move {
        // Initial brief sleep to let the fast loop emit the first memory snapshot immediately
        sleep(Duration::from_millis(150)).await;

        loop {
            // 1. Refresh window titles in the background
            let win_titles = get_window_titles_by_pid();
            if let Ok(mut lock) = win_cache_worker.write() {
                *lock = win_titles;
            }

            // 2. Read current Chromium targets to inspect
            let targets: Vec<(u32, Option<u16>, Vec<u32>)> = {
                pids_worker.read().map(|l| l.clone()).unwrap_or_default()
            };

            // 3. Resolve tab titles for each browser instance
            for (root_pid, debug_port, all_pids) in targets {
                let mut resolved_tabs = Vec::new();

                // Strategy A: Check CDP if remote debugging port is detected
                if let Some(port) = debug_port {
                    let cdp_tabs = query_cdp_tabs(port).await;
                    for tab in cdp_tabs {
                        if tab.target_type.as_deref() == Some("page") {
                            if let Some(title) = tab.title {
                                if !title.is_empty() {
                                    resolved_tabs.push(title);
                                } else if let Some(url) = tab.url {
                                    resolved_tabs.push(url);
                                }
                            } else if let Some(url) = tab.url {
                                resolved_tabs.push(url);
                            }
                        }
                    }
                }

                // Strategy B: Windows UI Automation + Win32 window text fallback
                if resolved_tabs.is_empty() {
                    let pids_clone = all_pids.clone();
                    let uia_tabs = tokio::task::spawn_blocking(move || {
                        get_browser_tab_titles_via_uia(&pids_clone)
                    })
                    .await
                    .unwrap_or_default();

                    resolved_tabs = uia_tabs;
                }

                if !resolved_tabs.is_empty() {
                    if let Ok(mut lock) = tab_cache_worker.write() {
                        lock.insert(root_pid, resolved_tabs);
                    }
                }
            }

            // Rest between enrichment cycles (2 seconds), or wake up immediately if retriggered!
            tokio::select! {
                _ = sleep(Duration::from_millis(2000)) => {}
                _ = notify_worker.notified() => {}
            }
        }
    });

    // -------------------------------------------------------------
    // TIER 1: ULTRA-FAST HIGH-FREQUENCY MEMORY LOOP (Foreground Pipeline)
    // -------------------------------------------------------------
    let tab_cache_foreground = tab_titles_cache.clone();
    tokio::spawn(async move {
        let mut scanner = ProcessScanner::new();
        let mut persistent_tab_assignments: HashMap<u32, String> = HashMap::new();

        loop {
            // Check for pending control commands
            while let Ok(cmd) = cmd_rx.try_recv() {
                match cmd {
                    SamplerCommand::SetInterval(d) => interval_duration = d,
                    SamplerCommand::ForceRefresh => {}
                    SamplerCommand::RetriggerTabDetection => {
                        // Clear caches and wake up Tier 2 immediately!
                        if let Ok(mut lock) = tab_cache_foreground.write() {
                            lock.clear();
                        }
                        persistent_tab_assignments.clear();
                        retrigger_notify.notify_one();
                    }
                }
            }

            let start = Instant::now();

            // 1. Collect global host memory stats (<1ms)
            let mem_stats = get_system_memory_stats();

            // 2. Instant O(1) read from cached window titles (guard dropped immediately)
            let win_titles = window_titles_cache
                .read()
                .map(|l| l.clone())
                .unwrap_or_default();

            // 3. Scan raw process memory counters and CPU deltas (<8ms)
            let mut processes =
                scanner.scan_processes(mem_stats.total_physical_bytes, &win_titles);

            // 4. Update the active Chromium targets list for the background enrichment worker
            let mut current_chromium_targets = Vec::new();
            for p in &processes {
                if p.is_chromium {
                    let mut family_pids = vec![p.pid];
                    for c in &p.children {
                        family_pids.push(c.pid);
                    }
                    current_chromium_targets.push((p.pid, p.debug_port, family_pids));
                }
            }
            if let Ok(mut lock) = active_chromium_pids.write() {
                *lock = current_chromium_targets;
            }

            // 5. Stable, persistent PID -> Tab Title assignment
            if let Ok(tab_cache) = tab_titles_cache.read() {
                // Prune dead PIDs from persistent assignments
                let current_pids: HashSet<u32> = processes.iter().flat_map(|p| {
                    std::iter::once(p.pid).chain(p.children.iter().map(|c| c.pid))
                }).collect();
                persistent_tab_assignments.retain(|pid, _| current_pids.contains(pid));

                for root in &mut processes {
                    if root.is_chromium {
                        if let Some(tabs) = tab_cache.get(&root.pid) {
                            // Find which tab titles are already assigned to active children of this root
                            let mut assigned_titles: HashSet<String> = HashSet::new();
                            for child in &root.children {
                                if let Some(title) = persistent_tab_assignments.get(&child.pid) {
                                    if tabs.contains(title) {
                                        assigned_titles.insert(title.clone());
                                    }
                                }
                            }

                            // Pool of available unassigned tabs
                            let mut unassigned_tabs: Vec<String> = tabs
                                .iter()
                                .filter(|t| !assigned_titles.contains(*t))
                                .cloned()
                                .collect();

                            for child in &mut root.children {
                                if child.proc_type_label.contains("Renderer") {
                                    // If child already has a valid persistent title, keep it pinned!
                                    if let Some(existing) = persistent_tab_assignments.get(&child.pid) {
                                        if tabs.contains(existing) {
                                            child.display_title = existing.clone();
                                            continue;
                                        }
                                    }

                                    // Otherwise assign the next available tab title and pin it
                                    if let Some(new_title) = unassigned_tabs.pop() {
                                        persistent_tab_assignments.insert(child.pid, new_title.clone());
                                        child.display_title = new_title;
                                    }
                                }
                            }

                            if root.display_title.is_empty() {
                                if let Some(first) = tabs.first() {
                                    root.display_title = first.clone();
                                }
                            }
                        }
                    }
                }
            }

            let timestamp_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            let snapshot = SystemSnapshot {
                mem_stats,
                processes,
                timestamp_ms,
                timestamp: Instant::now(),
            };

            // Immediately emit snapshot to UI thread (<10ms total execution time!)
            let _ = snapshot_tx.send(snapshot);

            // Sleep for the remaining interval
            let elapsed = start.elapsed();
            if elapsed < interval_duration {
                sleep(interval_duration - elapsed).await;
            }
        }
    });
}
