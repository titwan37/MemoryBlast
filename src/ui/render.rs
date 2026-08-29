use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Cell, Clear, Gauge, Paragraph, Row, Table,
};
use ratatui::Frame;

use crate::ui::app::{App, FlattenedRow};

pub fn render(frame: &mut Frame, app: &mut App) {
    let size = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Top system memory gauges
            Constraint::Length(if app.is_searching || !app.search_query.is_empty() { 3 } else { 0 }), // Search bar
            Constraint::Min(8),    // Process table
            Constraint::Length(2), // Status & Keybindings footer
        ])
        .split(size);

    let flattened = app.get_flattened_rows();

    // Clamp selection
    if !flattened.is_empty() && app.selected_index >= flattened.len() {
        app.selected_index = flattened.len() - 1;
    }

    render_top_gauges(frame, app, chunks[0]);

    if app.is_searching || !app.search_query.is_empty() {
        render_search_bar(frame, app, chunks[1], flattened.len());
    }

    render_process_table(frame, app, chunks[2], &flattened);
    render_footer(frame, app, chunks[3]);

    if let Some((pid, name)) = &app.pending_kill_pid {
        render_kill_modal(frame, *pid, name, size);
    }
}

fn render_top_gauges(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            " ⚡ MemoryBlast — Windows Host Memory Monitor ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if let Some(snapshot) = &app.snapshot {
        let mem = &snapshot.mem_stats;

        let col_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(38), // Physical RAM
                Constraint::Percentage(38), // Committed Pagefile
                Constraint::Percentage(24), // Kernel Pools & Admin Tag
            ])
            .split(inner);

        // 1. Physical RAM Gauge
        let phys_ratio = if mem.total_physical_bytes > 0 {
            (mem.used_physical_bytes as f64 / mem.total_physical_bytes as f64).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let phys_label = format!(
            "RAM: {} / {} ({:.1}%)",
            format_bytes(mem.used_physical_bytes),
            format_bytes(mem.total_physical_bytes),
            phys_ratio * 100.0
        );

        let phys_gauge = Gauge::default()
            .block(Block::default().borders(Borders::NONE))
            .gauge_style(
                Style::default()
                    .fg(if phys_ratio > 0.85 {
                        Color::Red
                    } else if phys_ratio > 0.70 {
                        Color::Yellow
                    } else {
                        Color::Green
                    })
                    .bg(Color::DarkGray),
            )
            .ratio(phys_ratio)
            .label(phys_label);
        frame.render_widget(phys_gauge, col_chunks[0]);

        // 2. Commit / Pagefile Gauge
        let commit_ratio = if mem.total_page_file_bytes > 0 {
            (mem.committed_bytes as f64 / mem.total_page_file_bytes as f64).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let commit_label = format!(
            "Commit: {} / {} ({:.1}%)",
            format_bytes(mem.committed_bytes),
            format_bytes(mem.total_page_file_bytes),
            commit_ratio * 100.0
        );

        let commit_gauge = Gauge::default()
            .block(Block::default().borders(Borders::NONE))
            .gauge_style(
                Style::default()
                    .fg(if commit_ratio > 0.85 {
                        Color::LightRed
                    } else {
                        Color::Cyan
                    })
                    .bg(Color::DarkGray),
            )
            .ratio(commit_ratio)
            .label(commit_label);
        frame.render_widget(commit_gauge, col_chunks[1]);

        // 3. Kernel Pools & Mode
        let admin_span = if mem.is_admin {
            Span::styled(" [ADMIN] ", Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD))
        } else {
            Span::styled(" [STANDARD USER] ", Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD))
        };

        let pools_line1 = Line::from(vec![
            admin_span,
            Span::raw(" "),
            Span::styled(format!("Avail: {}", format_bytes(mem.available_physical_bytes)), Style::default().fg(Color::White)),
        ]);

        let pools_line2 = Line::from(vec![
            Span::styled(format!("Paged: {}", format_bytes(mem.paged_pool_bytes)), Style::default().fg(Color::DarkGray)),
            Span::raw(" │ "),
            Span::styled(format!("NonPaged: {}", format_bytes(mem.non_paged_pool_bytes)), Style::default().fg(Color::DarkGray)),
        ]);

        let pool_paragraph = Paragraph::new(vec![pools_line1, pools_line2])
            .alignment(Alignment::Right);
        frame.render_widget(pool_paragraph, col_chunks[2]);
    }
}

fn render_search_bar(frame: &mut Frame, app: &App, area: Rect, match_count: usize) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow))
        .title(Span::styled(
            format!(" 🔍 Filter Processes / Tab Titles ({match_count} matches) "),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ));

    let cursor = if app.is_searching { "█" } else { "" };
    let input_text = format!(" {}{}", app.search_query, cursor);
    let paragraph = Paragraph::new(input_text).block(block);
    frame.render_widget(paragraph, area);
}

fn render_process_table(
    frame: &mut Frame,
    app: &mut App,
    area: Rect,
    flattened: &[FlattenedRow],
) {
    let header_cells = [
        Cell::from(Span::styled("PID", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Cell::from(Span::styled("Process Name / Resolved Tab Title", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Cell::from(Span::styled("Process Type", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Cell::from(Span::styled("Private RAM", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Cell::from(Span::styled("Working Set", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Cell::from(Span::styled("% RAM", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Cell::from(Span::styled("CPU %", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
    ];

    let header = Row::new(header_cells)
        .style(Style::default().bg(Color::Rgb(20, 25, 40)))
        .height(1)
        .bottom_margin(0);

    let rows_visible = area.height.saturating_sub(3) as usize;

    // Adjust scroll offset
    if app.selected_index < app.scroll_offset {
        app.scroll_offset = app.selected_index;
    } else if app.selected_index >= app.scroll_offset + rows_visible {
        app.scroll_offset = app.selected_index.saturating_sub(rows_visible).saturating_add(1);
    }

    let rows: Vec<Row> = flattened
        .iter()
        .enumerate()
        .skip(app.scroll_offset)
        .take(rows_visible)
        .map(|(idx, row)| {
            let is_selected = idx == app.selected_index;
            create_table_row(row, is_selected)
        })
        .collect();

    let widths = [
        Constraint::Length(8),
        Constraint::Min(32),
        Constraint::Length(22),
        Constraint::Length(14),
        Constraint::Length(14),
        Constraint::Length(8),
        Constraint::Length(8),
    ];

    let sort_label = format!(" Active Sort: {} ", app.sort_field.display_name());
    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Gray))
                .title(Span::styled(
                    format!(" Processes & Tabs ({}) ", flattened.len()),
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                ))
                .title_alignment(Alignment::Left)
                .title(
                    Line::from(vec![Span::styled(
                        sort_label,
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::ITALIC),
                    )])
                ),
        );

    frame.render_widget(table, area);
}

fn create_table_row<'a>(row: &'a FlattenedRow, is_selected: bool) -> Row<'a> {
    let item = &row.item;

    let base_style = if is_selected {
        Style::default().bg(Color::Rgb(30, 60, 100)).fg(Color::White).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    // Format PID
    let pid_span = Span::styled(item.pid.to_string(), base_style);

    // Format Name / Tab Title
    let mut name_spans = Vec::new();
    if row.depth > 0 {
        let tree_prefix = if row.is_last_child { "  └─ " } else { "  ├─ " };
        name_spans.push(Span::styled(tree_prefix, Style::default().fg(Color::DarkGray)));

        let icon = match item.proc_type_label.as_str() {
            "GPU Process" => "⚡ ",
            "Network Service" => "🌐 ",
            "Audio Service" => "🔊 ",
            "Web Extension" => "🧩 ",
            _ => "📄 ",
        };
        name_spans.push(Span::styled(icon, Style::default().fg(Color::Cyan)));

        if !item.display_title.is_empty() {
            name_spans.push(Span::styled(&item.display_title, base_style));
        } else {
            name_spans.push(Span::styled(&item.name, base_style));
        }
    } else {
        if row.is_expandable {
            let toggle = if row.is_expanded { "▼ " } else { "▶ " };
            name_spans.push(Span::styled(toggle, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
        } else {
            name_spans.push(Span::raw("  "));
        }

        name_spans.push(Span::styled(&item.name, base_style));

        if row.is_expandable && !row.is_expanded {
            name_spans.push(Span::styled(
                format!(" ({} children)", row.child_count),
                Style::default().fg(Color::DarkGray),
            ));
        }

        if !item.display_title.is_empty() {
            name_spans.push(Span::styled(
                format!(" — {}", item.display_title),
                Style::default().fg(Color::LightCyan),
            ));
        }
    }

    // Process Type
    let type_style = if row.depth > 0 {
        Style::default().fg(Color::DarkGray)
    } else if item.is_chromium {
        Style::default().fg(Color::LightGreen)
    } else {
        Style::default().fg(Color::Gray)
    };
    let type_span = Span::styled(&item.proc_type_label, type_style);

    // Private RAM
    let priv_bytes = if row.depth == 0 && row.is_expandable {
        row.total_group_private_bytes
    } else {
        item.private_bytes
    };
    let priv_color = if priv_bytes > 1_073_741_824 {
        Color::LightRed
    } else if priv_bytes > 524_288_000 {
        Color::LightYellow
    } else {
        Color::White
    };
    let priv_span = Span::styled(format_bytes(priv_bytes), Style::default().fg(priv_color));

    // Working Set
    let ws_bytes = if row.depth == 0 && row.is_expandable {
        row.total_group_working_set_bytes
    } else {
        item.working_set_bytes
    };
    let ws_span = Span::styled(format_bytes(ws_bytes), Style::default().fg(Color::Gray));

    // % Memory
    let mem_pct_span = Span::styled(
        format!("{:.1}%", item.memory_percent),
        Style::default().fg(Color::DarkGray),
    );

    // CPU %
    let cpu_val = if row.depth == 0 && row.is_expandable {
        row.total_group_cpu_percent
    } else {
        item.cpu_percent
    };
    let cpu_color = if cpu_val > 20.0 {
        Color::Red
    } else if cpu_val > 5.0 {
        Color::LightYellow
    } else {
        Color::Green
    };
    let cpu_span = Span::styled(format!("{cpu_val:.1}%"), Style::default().fg(cpu_color));

    Row::new(vec![
        Cell::from(pid_span),
        Cell::from(Line::from(name_spans)),
        Cell::from(type_span),
        Cell::from(priv_span),
        Cell::from(ws_span),
        Cell::from(mem_pct_span),
        Cell::from(cpu_span),
    ])
    .style(base_style)
    .height(1)
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let now = std::time::Instant::now();
    let status_span = if let Some((msg, timestamp)) = &app.status_message {
        if now.duration_since(*timestamp).as_secs() < 4 {
            Span::styled(format!("  🔔 {msg}  "), Style::default().fg(Color::Black).bg(Color::Cyan))
        } else {
            Span::raw("")
        }
    } else if let Some(snapshot) = &app.snapshot {
        let age_ms = now.duration_since(snapshot.timestamp).as_millis();
        Span::styled(format!("  ⏱️ {age_ms}ms ago"), Style::default().fg(Color::DarkGray))
    } else {
        Span::raw("")
    };

    let rate_ms = app.current_refresh_rate().as_millis();

    let shortcuts = Line::from(vec![
        Span::styled(" [/] ", Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD)),
        Span::raw("Search  "),
        Span::styled(" [s] ", Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD)),
        Span::raw("Sort  "),
        Span::styled(" [Space/Enter] ", Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD)),
        Span::raw("Expand  "),
        Span::styled(" [f] ", Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw("Focus  "),
        Span::styled(" [t] ", Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw("Detect Tabs  "),
        Span::styled(" [k] ", Style::default().fg(Color::Black).bg(Color::Red).add_modifier(Modifier::BOLD)),
        Span::raw("Kill  "),
        Span::styled(" [+/-] ", Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD)),
        Span::raw(format!("Rate ({}ms)  ", rate_ms)),
        Span::styled(" [q] ", Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD)),
        Span::raw("Quit"),
        status_span,
    ]);

    let paragraph = Paragraph::new(shortcuts).alignment(Alignment::Left);
    frame.render_widget(paragraph, area);
}

fn render_kill_modal(frame: &mut Frame, pid: u32, name: &str, area: Rect) {
    let popup_area = centered_rect(60, 25, area);
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Red))
        .title(Span::styled(
            " ⚠️ TERMINATE PROCESS ",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ));

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("Are you sure you want to terminate: "),
            Span::styled(name, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::raw("Process ID: "),
            Span::styled(pid.to_string(), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" [Y] Confirm ", Style::default().fg(Color::Black).bg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw("      "),
            Span::styled(" [N / Esc] Cancel ", Style::default().fg(Color::Black).bg(Color::Gray).add_modifier(Modifier::BOLD)),
        ]),
    ];

    let paragraph = Paragraph::new(text).block(block).alignment(Alignment::Center);
    frame.render_widget(paragraph, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
