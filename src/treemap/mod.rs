//! Interactive Squarified Treemap Visualization
//!
//! Implements the Bruls, Huizing, and van Wijk (2000) squarified treemap algorithm
//! to achieve aspect ratios close to 1:1 for intuitive memory consumption visualization.
//! Supports hierarchical aggregation: Host -> App Clusters -> Process / Tabs.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::proc::ProcessItem;
use crate::sampler::SystemSnapshot;

/// Geometric bounding box in 2D space (normalized or pixel coordinates)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TreemapRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl TreemapRect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x: x.max(0.0),
            y: y.max(0.0),
            width: width.max(0.0),
            height: height.max(0.0),
        }
    }

    pub fn area(&self) -> f64 {
        self.width * self.height
    }

    pub fn shorter_side(&self) -> f64 {
        self.width.min(self.height)
    }

    pub fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.x && px <= (self.x + self.width) && py >= self.y && py <= (self.y + self.height)
    }
}

/// A node in the hierarchical Treemap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreemapNode {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub value_bytes: u64,
    pub formatted_size: String,
    pub category: String,
    pub color: String,
    pub pid: Option<u32>,
    pub leak_velocity_mb_min: f64,
    pub z_score: f64,
    pub rect: Option<TreemapRect>,
    pub children: Vec<TreemapNode>,
}

impl TreemapNode {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        subtitle: impl Into<String>,
        value_bytes: u64,
        category: impl Into<String>,
        color: impl Into<String>,
        pid: Option<u32>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            subtitle: subtitle.into(),
            value_bytes,
            formatted_size: format_bytes(value_bytes),
            category: category.into(),
            color: color.into(),
            pid,
            leak_velocity_mb_min: 0.0,
            z_score: 0.0,
            rect: None,
            children: Vec::new(),
        }
    }

    pub fn add_child(&mut self, child: TreemapNode) {
        self.children.push(child);
    }
}

/// Formats byte counts into human-readable strings (MB, GB)
pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let b = bytes as f64;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.0} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}

/// Computes the worst aspect ratio for a row of normalized areas laid out along side `w`
fn worst_aspect_ratio(row: &[f64], w: f64) -> f64 {
    if row.is_empty() || w <= 0.0 {
        return f64::MAX;
    }
    let s: f64 = row.iter().sum();
    if s <= 0.0 {
        return f64::MAX;
    }

    let mut max_r: f64 = 0.0;
    let w_sq = w * w;
    let s_sq = s * s;

    for &r in row {
        if r <= 0.0 {
            continue;
        }
        let ratio1 = (w_sq * r) / s_sq;
        let ratio2 = s_sq / (w_sq * r);
        let ratio = ratio1.max(ratio2);
        if ratio > max_r {
            max_r = ratio;
        }
    }

    if max_r == 0.0 {
        f64::MAX
    } else {
        max_r
    }
}

/// Recursively computes squarified layout for items within `bounds`
pub fn squarify_layout(nodes: &mut [TreemapNode], bounds: TreemapRect) {
    if nodes.is_empty() || bounds.area() <= 0.0 {
        return;
    }

    let total_value: u64 = nodes.iter().map(|n| n.value_bytes).sum();
    if total_value == 0 {
        return;
    }

    // Sort descending by value
    nodes.sort_by(|a, b| b.value_bytes.cmp(&a.value_bytes));

    let total_area = bounds.area();
    let normalized_areas: Vec<f64> = nodes
        .iter()
        .map(|n| ((n.value_bytes as f64) / (total_value as f64)) * total_area)
        .collect();

    let mut current_bounds = bounds;
    let mut i = 0;
    let n = nodes.len();

    while i < n && current_bounds.area() > 0.0 {
        let mut row_indices = vec![i];
        let mut row_areas = vec![normalized_areas[i]];
        let w = current_bounds.shorter_side();

        let mut j = i + 1;
        while j < n {
            let next_area = normalized_areas[j];
            let mut test_row = row_areas.clone();
            test_row.push(next_area);

            let current_worst = worst_aspect_ratio(&row_areas, w);
            let next_worst = worst_aspect_ratio(&test_row, w);

            if next_worst <= current_worst {
                row_indices.push(j);
                row_areas.push(next_area);
                j += 1;
            } else {
                break;
            }
        }

        layout_row(nodes, &row_indices, &row_areas, &mut current_bounds);
        i = j;
    }

    // Recursively layout children inside parent bounds
    for node in nodes.iter_mut() {
        if let Some(parent_rect) = node.rect {
            if !node.children.is_empty() && parent_rect.width > 24.0 && parent_rect.height > 28.0 {
                let child_bounds = TreemapRect::new(
                    parent_rect.x + 4.0,
                    parent_rect.y + 22.0,
                    (parent_rect.width - 8.0).max(0.0),
                    (parent_rect.height - 26.0).max(0.0),
                );
                squarify_layout(&mut node.children, child_bounds);
            }
        }
    }
}

/// Positions a single row of rectangles along current_bounds.shorter_side() and clips container
fn layout_row(
    nodes: &mut [TreemapNode],
    row_indices: &[usize],
    row_areas: &[f64],
    bounds: &mut TreemapRect,
) {
    if row_indices.is_empty() || bounds.area() <= 0.0 {
        return;
    }

    let row_total_area: f64 = row_areas.iter().sum();
    let is_horizontal = bounds.width >= bounds.height;

    if is_horizontal {
        let col_width = if bounds.height > 0.0 {
            (row_total_area / bounds.height).min(bounds.width)
        } else {
            0.0
        };

        let mut curr_y = bounds.y;
        for (&node_idx, &area) in row_indices.iter().zip(row_areas.iter()) {
            let item_h = if col_width > 0.0 {
                (area / col_width).min(bounds.y + bounds.height - curr_y)
            } else {
                0.0
            };

            nodes[node_idx].rect = Some(TreemapRect::new(bounds.x, curr_y, col_width, item_h));
            curr_y += item_h;
        }

        bounds.x += col_width;
        bounds.width = (bounds.width - col_width).max(0.0);
    } else {
        let row_height = if bounds.width > 0.0 {
            (row_total_area / bounds.width).min(bounds.height)
        } else {
            0.0
        };

        let mut curr_x = bounds.x;
        for (&node_idx, &area) in row_indices.iter().zip(row_areas.iter()) {
            let item_w = if row_height > 0.0 {
                (area / row_height).min(bounds.x + bounds.width - curr_x)
            } else {
                0.0
            };

            nodes[node_idx].rect = Some(TreemapRect::new(curr_x, bounds.y, item_w, row_height));
            curr_x += item_w;
        }

        bounds.y += row_height;
        bounds.height = (bounds.height - row_height).max(0.0);
    }
}

/// Builds a hierarchical Treemap structure from a live `SystemSnapshot`
pub fn build_treemap_from_snapshot(
    snapshot: &SystemSnapshot,
    container_width: f64,
    container_height: f64,
) -> TreemapNode {
    let total_used = snapshot.mem_stats.used_physical_bytes;
    let mut root = TreemapNode::new(
        "root",
        "System Memory",
        format!("Total: {}", format_bytes(snapshot.mem_stats.total_physical_bytes)),
        total_used,
        "system",
        "#1e1e2e",
        None,
    );

    // Group processes into Chromium browsers, high-impact apps, and background tasks
    let mut browser_clusters: HashMap<String, Vec<&ProcessItem>> = HashMap::new();
    let mut other_processes: Vec<&ProcessItem> = Vec::new();

    for proc in &snapshot.processes {
        if proc.is_chromium {
            let browser_name = if let Some(family) = &proc.browser_family {
                family.clone()
            } else if proc.name.to_lowercase().contains("chrome") {
                "Google Chrome".to_string()
            } else if proc.name.to_lowercase().contains("msedge") {
                "Microsoft Edge".to_string()
            } else if proc.name.to_lowercase().contains("brave") {
                "Brave Browser".to_string()
            } else if proc.name.to_lowercase().contains("arc") {
                "Arc".to_string()
            } else if proc.name.to_lowercase().contains("opera") {
                "Opera".to_string()
            } else {
                "Chromium App".to_string()
            };
            browser_clusters.entry(browser_name).or_default().push(proc);
        } else {
            other_processes.push(proc);
        }
    }

    // Add Chromium Clusters
    for (browser_name, items) in browser_clusters {
        let cluster_bytes: u64 = items.iter().map(|p| p.working_set_bytes).sum();
        let color = match browser_name.as_str() {
            "Google Chrome" => "#4285F4",
            "Microsoft Edge" => "#0078D7",
            "Brave Browser" => "#FB542B",
            "Arc" => "#FF5964",
            _ => "#6366F1",
        };

        let mut cluster_node = TreemapNode::new(
            format!("cluster-{}", browser_name.to_lowercase().replace(' ', "-")),
            format!("{} ({} procs)", browser_name, items.len()),
            format!("Total: {}", format_bytes(cluster_bytes)),
            cluster_bytes,
            "browser_cluster",
            color,
            items.first().map(|p| p.pid),
        );

        // Add child processes/tabs under the browser cluster
        for proc in items {
            let is_tab = proc.proc_type_label.contains("Tab") || proc.display_title != proc.name;
            let (category, item_color) = if is_tab {
                ("tab", "#10B981") // emerald green for tabs
            } else if proc.proc_type_label.contains("GPU") {
                ("gpu", "#8B5CF6") // purple
            } else if proc.proc_type_label.contains("Extension") {
                ("extension", "#F59E0B") // amber
            } else if proc.proc_type_label.contains("Network") {
                ("network", "#06B6D4") // cyan
            } else {
                ("renderer", "#3B82F6") // blue
            };

            let title = proc.display_title.clone();
            let subtitle = format!("PID: {} | CPU: {:.1}%", proc.pid, proc.cpu_percent);

            let mut leaf = TreemapNode::new(
                format!("pid-{}", proc.pid),
                title,
                subtitle,
                proc.working_set_bytes.max(1024),
                category,
                item_color,
                Some(proc.pid),
            );
            leaf.children = Vec::new();
            cluster_node.add_child(leaf);
        }

        root.add_child(cluster_node);
    }

    // Group other high-RAM processes
    other_processes.sort_by(|a, b| b.working_set_bytes.cmp(&a.working_set_bytes));

    let top_limit = 25;
    for proc in other_processes.iter().take(top_limit) {
        let color = if proc.working_set_bytes > 500 * 1024 * 1024 {
            "#EF4444" // red for >500MB
        } else if proc.working_set_bytes > 150 * 1024 * 1024 {
            "#F59E0B" // amber for >150MB
        } else {
            "#0EA5E9" // light blue
        };

        let node = TreemapNode::new(
            format!("pid-{}", proc.pid),
            proc.display_title.clone(),
            format!("PID: {} | CPU: {:.1}%", proc.pid, proc.cpu_percent),
            proc.working_set_bytes.max(1024),
            "process",
            color,
            Some(proc.pid),
        );
        root.add_child(node);
    }

    // Aggregate remaining processes into "Other System Workloads"
    let remaining_bytes = other_processes
        .iter()
        .skip(top_limit)
        .map(|p| p.working_set_bytes)
        .sum::<u64>();

    if remaining_bytes > 0 {
        root.add_child(TreemapNode::new(
            "other-workloads",
            "Other System Services",
            format!("{} background tasks", other_processes.len().saturating_sub(top_limit)),
            remaining_bytes,
            "misc",
            "#475569",
            None,
        ));
    }

    // Run squarified layout
    let bounds = TreemapRect::new(0.0, 0.0, container_width, container_height);
    squarify_layout(&mut root.children, bounds);

    root.rect = Some(bounds);
    root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_squarified_layout_aspect_ratios() {
        let mut nodes = vec![
            TreemapNode::new("1", "A", "", 600, "cat", "#fff", None),
            TreemapNode::new("2", "B", "", 600, "cat", "#fff", None),
            TreemapNode::new("3", "C", "", 400, "cat", "#fff", None),
            TreemapNode::new("4", "D", "", 400, "cat", "#fff", None),
            TreemapNode::new("5", "E", "", 200, "cat", "#fff", None),
            TreemapNode::new("6", "F", "", 200, "cat", "#fff", None),
            TreemapNode::new("7", "G", "", 100, "cat", "#fff", None),
        ];

        let bounds = TreemapRect::new(0.0, 0.0, 600.0, 400.0);
        squarify_layout(&mut nodes, bounds);

        for node in &nodes {
            assert!(node.rect.is_some());
            let r = node.rect.unwrap();
            assert!(r.width > 0.0);
            assert!(r.height > 0.0);
            assert!(bounds.contains(r.x + 0.01, r.y + 0.01));
        }
    }
}
