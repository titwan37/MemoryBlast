use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crossbeam_channel::Sender;

use crate::sampler::SamplerCommand;
use crate::ui::app::App;

pub fn handle_key_event(
    key: KeyEvent,
    app: &mut App,
    cmd_tx: &Sender<SamplerCommand>,
) {
    // 1. If Kill confirmation modal is open
    if app.pending_kill_pid.is_some() {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                app.confirm_kill();
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.cancel_kill();
            }
            _ => {}
        }
        return;
    }

    // 2. If in Search / Filter input mode
    if app.is_searching {
        match key.code {
            KeyCode::Esc => {
                app.is_searching = false;
                app.search_query.clear();
            }
            KeyCode::Enter => {
                app.is_searching = false;
            }
            KeyCode::Backspace => {
                app.search_query.pop();
                app.selected_index = 0;
            }
            KeyCode::Char(c) => {
                app.search_query.push(c);
                app.selected_index = 0;
            }
            _ => {}
        }
        return;
    }

    // 3. Normal navigation & command mode
    let flattened = app.get_flattened_rows();
    let row_count = flattened.len();

    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            app.should_quit = true;
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }

        // Navigation
        KeyCode::Up | KeyCode::Char('w') => {
            app.move_selection_up();
        }
        KeyCode::Down | KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Ctrl+S: sort
            app.cycle_sort();
        }
        KeyCode::Down => {
            app.move_selection_down(row_count);
        }
        KeyCode::PageUp => {
            for _ in 0..10 {
                app.move_selection_up();
            }
        }
        KeyCode::PageDown => {
            for _ in 0..10 {
                app.move_selection_down(row_count);
            }
        }
        KeyCode::Home => {
            app.selected_index = 0;
        }
        KeyCode::End => {
            if row_count > 0 {
                app.selected_index = row_count - 1;
            }
        }

        // Tree expand / collapse
        KeyCode::Char(' ') | KeyCode::Enter => {
            app.toggle_expand(&flattened);
        }

        // Cycle sort
        KeyCode::Char('s') | KeyCode::Char('S') => {
            app.cycle_sort();
        }

        // Search trigger
        KeyCode::Char('/') => {
            app.is_searching = true;
        }

        // Process kill modal
        KeyCode::Char('k') | KeyCode::Char('K') => {
            app.trigger_kill_modal(&flattened);
        }

        // View switching
        KeyCode::Tab => {
            app.current_view = match app.current_view {
                crate::ui::app::TuiView::Processes => crate::ui::app::TuiView::Treemap,
                crate::ui::app::TuiView::Treemap => crate::ui::app::TuiView::Optimizer,
                crate::ui::app::TuiView::Optimizer => crate::ui::app::TuiView::Processes,
            };
            let name = match app.current_view {
                crate::ui::app::TuiView::Processes => "Processes Table",
                crate::ui::app::TuiView::Treemap => "Squarified Treemap",
                crate::ui::app::TuiView::Optimizer => "Cognitive Optimizer & Anomaly Detector",
            };
            app.set_status(format!("Switched view: {name}"));
        }
        KeyCode::Char('m') | KeyCode::Char('M') => {
            app.current_view = crate::ui::app::TuiView::Treemap;
            app.set_status("Switched view: Squarified Treemap".to_string());
        }
        KeyCode::Char('o') | KeyCode::Char('O') => {
            app.current_view = crate::ui::app::TuiView::Optimizer;
            app.set_status("Switched view: Cognitive Optimizer & Anomaly Detector".to_string());
        }
        KeyCode::Char('p') | KeyCode::Char('P') => {
            app.current_view = crate::ui::app::TuiView::Processes;
            app.set_status("Switched view: Processes Table".to_string());
        }

        // Working set trimming (EmptyWorkingSet)
        KeyCode::Char('x') | KeyCode::Char('X') => {
            if let Some(row) = flattened.get(app.selected_index) {
                let pid = row.item.pid;
                let name = row.item.name.clone();
                match crate::optimizer::OptimizationAssistant::trim_working_set(pid) {
                    Ok(_) => app.set_status(format!("✂️ Trimmed working set for {name} (PID: {pid})")),
                    Err(e) => app.set_status(format!("Failed to trim working set: {e}")),
                }
            }
        }

        // Park current context (Focus Copilot: Freeze/Stash tabs)
        KeyCode::Char('z') | KeyCode::Char('Z') => {
            tokio::spawn(async {
                let _ = crate::copilot::park_current_context(&[], &[]).await;
            });
            app.set_status("📦 Focus Copilot: Parked active browser tabs & freed RAM!".to_string());
        }

        // Focus tab in browser
        KeyCode::Char('f') | KeyCode::Char('F') => {
            if let Some(row) = flattened.get(app.selected_index) {
                let pid = row.item.pid;
                let ppid = row.item.ppid;
                let title = row.item.display_title.clone();
                let port = row.item.debug_port;
                let name = row.item.name.clone();

                let title_clone = title.clone();
                tokio::spawn(async move {
                    let _ = crate::chromium::focus_chromium_tab(pid, ppid, &title_clone, port).await;
                });

                let label = if !title.is_empty() { title } else { name };
                app.set_status(format!("🎯 Focusing tab: {label}"));
            }
        }

        // Retrigger tab detection
        KeyCode::Char('t') | KeyCode::Char('T') => {
            let _ = cmd_tx.send(SamplerCommand::RetriggerTabDetection);
            app.set_status("🔍 Retriggering tab detection...".to_string());
        }

        // Sampling rate adjustment
        KeyCode::Char('+') | KeyCode::Char('=') => {
            if let Some(new_rate) = app.increase_refresh_rate() {
                let _ = cmd_tx.send(SamplerCommand::SetInterval(new_rate));
                app.set_status(format!("Refresh rate: {:?}", new_rate));
            }
        }
        KeyCode::Char('-') | KeyCode::Char('_') => {
            if let Some(new_rate) = app.decrease_refresh_rate() {
                let _ = cmd_tx.send(SamplerCommand::SetInterval(new_rate));
            }
        }

        // Manual refresh
        KeyCode::Char('r') | KeyCode::Char('R') => {
            let _ = cmd_tx.send(SamplerCommand::ForceRefresh);
            app.set_status("Triggered manual refresh".to_string());
        }

        _ => {}
    }
}
