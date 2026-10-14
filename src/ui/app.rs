use std::collections::HashSet;
use std::time::Duration;

use crate::proc::{kill_process, ProcessItem, SortField};
use crate::sampler::SystemSnapshot;

#[derive(Debug, Clone)]
pub struct FlattenedRow {
    pub item: ProcessItem,
    pub depth: usize,
    pub is_last_child: bool,
    pub is_expandable: bool,
    pub is_expanded: bool,
    pub child_count: usize,
    pub total_group_private_bytes: u64,
    pub total_group_working_set_bytes: u64,
    pub total_group_cpu_percent: f32,
    pub total_group_gpu_percent: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiView {
    Processes,
    Treemap,
    Optimizer,
}

pub struct App {
    pub snapshot: Option<SystemSnapshot>,
    pub current_view: TuiView,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub expanded_pids: HashSet<u32>,
    pub sort_field: SortField,
    pub search_query: String,
    pub is_searching: bool,
    pub pending_kill_pid: Option<(u32, String)>,
    pub status_message: Option<(String, std::time::Instant)>,
    pub refresh_rates: Vec<Duration>,
    pub refresh_rate_idx: usize,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        let refresh_rates = vec![
            Duration::from_millis(250),
            Duration::from_millis(500),
            Duration::from_millis(1000),
            Duration::from_millis(2000),
            Duration::from_millis(5000),
        ];

        Self {
            snapshot: None,
            current_view: TuiView::Processes,
            selected_index: 0,
            scroll_offset: 0,
            expanded_pids: HashSet::new(),
            sort_field: SortField::PrivateMemory,
            search_query: String::new(),
            is_searching: false,
            pending_kill_pid: None,
            status_message: None,
            refresh_rates,
            refresh_rate_idx: 1, // Default 500ms
            should_quit: false,
        }
    }

    pub fn current_refresh_rate(&self) -> Duration {
        self.refresh_rates[self.refresh_rate_idx]
    }

    pub fn increase_refresh_rate(&mut self) -> Option<Duration> {
        if self.refresh_rate_idx > 0 {
            self.refresh_rate_idx -= 1;
            let rate = self.current_refresh_rate();
            self.set_status(format!("Sampling interval: {}ms", rate.as_millis()));
            Some(rate)
        } else {
            None
        }
    }

    pub fn decrease_refresh_rate(&mut self) -> Option<Duration> {
        if self.refresh_rate_idx + 1 < self.refresh_rates.len() {
            self.refresh_rate_idx += 1;
            let rate = self.current_refresh_rate();
            self.set_status(format!("Sampling interval: {}ms", rate.as_millis()));
            Some(rate)
        } else {
            None
        }
    }

    pub fn set_status(&mut self, msg: String) {
        self.status_message = Some((msg, std::time::Instant::now()));
    }

    pub fn cycle_sort(&mut self) {
        self.sort_field = self.sort_field.next();
        self.set_status(format!("Sort by: {}", self.sort_field.display_name()));
    }

    pub fn toggle_expand(&mut self, flattened: &[FlattenedRow]) {
        if let Some(row) = flattened.get(self.selected_index) {
            if row.is_expandable {
                if self.expanded_pids.contains(&row.item.pid) {
                    self.expanded_pids.remove(&row.item.pid);
                } else {
                    self.expanded_pids.insert(row.item.pid);
                }
            }
        }
    }

    pub fn move_selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn move_selection_down(&mut self, max_rows: usize) {
        if max_rows > 0 && self.selected_index + 1 < max_rows {
            self.selected_index += 1;
        }
    }

    pub fn trigger_kill_modal(&mut self, flattened: &[FlattenedRow]) {
        if let Some(row) = flattened.get(self.selected_index) {
            let label = if !row.item.display_title.is_empty() {
                format!("{} ({})", row.item.name, row.item.display_title)
            } else {
                row.item.name.clone()
            };
            self.pending_kill_pid = Some((row.item.pid, label));
        }
    }

    pub fn confirm_kill(&mut self) {
        if let Some((pid, name)) = self.pending_kill_pid.take() {
            match kill_process(pid) {
                Ok(_) => {
                    self.set_status(format!("Successfully terminated PID {pid} [{name}]"));
                }
                Err(err) => {
                    self.set_status(format!("Error terminating PID {pid}: {err}"));
                }
            }
        }
    }

    pub fn cancel_kill(&mut self) {
        self.pending_kill_pid = None;
    }

    /// Builds the list of flattened rows for display, applying search filter, sorting, and tree hierarchy.
    pub fn get_flattened_rows(&self) -> Vec<FlattenedRow> {
        let Some(snapshot) = &self.snapshot else {
            return Vec::new();
        };

        let mut roots = snapshot.processes.clone();

        // Sort root processes
        sort_process_items(&mut roots, self.sort_field);

        let query = self.search_query.trim().to_lowercase();
        let mut flattened = Vec::new();

        for root in roots {
            let is_expanded = self.expanded_pids.contains(&root.pid);
            let child_count = count_all_children(&root);

            let (group_priv, group_ws, group_cpu, group_gpu) = calc_group_totals(&root);

            // Filter logic: match if query matches root or any child
            let matches_root = process_matches_query(&root, &query);
            let matches_any_child = root.children.iter().any(|c| process_matches_query(c, &query));

            if !query.is_empty() && !matches_root && !matches_any_child {
                continue;
            }

            let is_expandable = !root.children.is_empty();

            flattened.push(FlattenedRow {
                item: root.clone(),
                depth: 0,
                is_last_child: false,
                is_expandable,
                is_expanded,
                child_count,
                total_group_private_bytes: group_priv,
                total_group_working_set_bytes: group_ws,
                total_group_cpu_percent: group_cpu,
                total_group_gpu_percent: group_gpu,
            });

            // Expand children if expanded OR if search is active and child matches
            if (is_expanded || !query.is_empty()) && is_expandable {
                let mut sorted_children = root.children.clone();
                sort_process_items(&mut sorted_children, self.sort_field);

                let total_children = sorted_children.len();
                for (idx, child) in sorted_children.into_iter().enumerate() {
                    if !query.is_empty() && !process_matches_query(&child, &query) && !matches_root {
                        continue;
                    }

                    flattened.push(FlattenedRow {
                        item: child,
                        depth: 1,
                        is_last_child: idx + 1 == total_children,
                        is_expandable: false,
                        is_expanded: false,
                        child_count: 0,
                        total_group_private_bytes: 0,
                        total_group_working_set_bytes: 0,
                        total_group_cpu_percent: 0.0,
                        total_group_gpu_percent: 0.0,
                    });
                }
            }
        }

        flattened
    }
}

fn process_matches_query(item: &ProcessItem, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    item.name.to_lowercase().contains(query)
        || item.display_title.to_lowercase().contains(query)
        || item.proc_type_label.to_lowercase().contains(query)
        || item.pid.to_string().contains(query)
}

fn sort_process_items(items: &mut [ProcessItem], sort_field: SortField) {
    items.sort_by(|a, b| match sort_field {
        SortField::PrivateMemory => {
            let a_tot = a.private_bytes + a.children.iter().map(|c| c.private_bytes).sum::<u64>();
            let b_tot = b.private_bytes + b.children.iter().map(|c| c.private_bytes).sum::<u64>();
            b_tot.cmp(&a_tot)
        }
        SortField::WorkingSet => {
            let a_tot = a.working_set_bytes + a.children.iter().map(|c| c.working_set_bytes).sum::<u64>();
            let b_tot = b.working_set_bytes + b.children.iter().map(|c| c.working_set_bytes).sum::<u64>();
            b_tot.cmp(&a_tot)
        }
        SortField::Cpu => {
            let a_tot = a.cpu_percent + a.children.iter().map(|c| c.cpu_percent).sum::<f32>();
            let b_tot = b.cpu_percent + b.children.iter().map(|c| c.cpu_percent).sum::<f32>();
            b_tot.partial_cmp(&a_tot).unwrap_or(std::cmp::Ordering::Equal)
        }
        SortField::Gpu => {
            let a_tot = a.gpu_percent + a.children.iter().map(|c| c.gpu_percent).sum::<f32>();
            let b_tot = b.gpu_percent + b.children.iter().map(|c| c.gpu_percent).sum::<f32>();
            b_tot.partial_cmp(&a_tot).unwrap_or(std::cmp::Ordering::Equal)
        }
        SortField::Pid => a.pid.cmp(&b.pid),
        SortField::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });
}

fn count_all_children(item: &ProcessItem) -> usize {
    let mut count = item.children.len();
    for c in &item.children {
        count += count_all_children(c);
    }
    count
}

fn calc_group_totals(item: &ProcessItem) -> (u64, u64, f32, f32) {
    let mut priv_b = item.private_bytes;
    let mut ws_b = item.working_set_bytes;
    let mut cpu = item.cpu_percent;
    let mut gpu = item.gpu_percent;

    for c in &item.children {
        let (p, w, cp, gp) = calc_group_totals(c);
        priv_b += p;
        ws_b += w;
        cpu += cp;
        gpu += gp;
    }

    (priv_b, ws_b, cpu, gpu)
}
