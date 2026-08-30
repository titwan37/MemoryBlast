use std::io::stdout;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::broadcast;

mod analytics;
mod chromium;
mod copilot;
pub mod explorer;
mod gui;
mod knowledge;
mod notifications;
mod optimizer;
mod proc;
mod sampler;
mod sys;
mod telemetry;
mod treemap;
mod ui;

use gui::{launch_desktop_window, GuiServer};
use sampler::spawn_sampler;
use sys::is_elevated;
use ui::{handle_key_event, render, App};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    // Check for help flag
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("⚡ MemoryBlast — High-Performance Windows Process Manager & Focus Copilot");
        println!();
        println!("USAGE:");
        println!("    memoryblast.exe [OPTIONS]");
        println!("    memoryblast.exe brief           # Display Executive Morning & Session Orientation briefing");
        println!("    memoryblast.exe resume          # Show 'Where you left off' orientation card");
        println!("    memoryblast.exe park            # Immediately stash active tabs and reclaim RAM");
        println!("    memoryblast.exe restore <id>    # Restore a previously stashed session");
        println!();
        println!("OPTIONS:");
        println!("    --gui       Launch Windows 11 Fluent Desktop GUI (Default)");
        println!("    --cli       Launch Terminal TUI Dashboard (Ratatui)");
        println!("    --help, -h  Print help information");
        return Ok(());
    }

    // CLI quick subcommands
    if args.iter().any(|a| a == "brief" || a == "morning" || a == "--brief") {
        let brief = knowledge::ExecutiveBriefing::generate();
        println!("{}", brief.render_terminal_card());
        return Ok(());
    }

    if args.iter().any(|a| a == "resume" || a == "anchor" || a == "--resume") {
        let anchor = copilot::load_latest_anchor(std::path::Path::new("memoryblast_data"))
            .unwrap_or_else(|| copilot::ResumeAnchor::capture(std::path::Path::new("."), &[]));
        println!("{}", anchor.render_terminal_card());
        return Ok(());
    }

    if args.len() >= 3 && args[1] == "restore" {
        if let Ok(id) = args[2].parse::<i64>() {
            match copilot::restore_session(id) {
                Ok(count) => {
                    println!("✨ Successfully restored session #{id} ({count} URLs reopened)!");
                    return Ok(());
                }
                Err(e) => {
                    eprintln!("❌ Error restoring session #{id}: {e}");
                    return Ok(());
                }
            }
        }
    }

    if args.iter().any(|a| a == "park" || a == "--park") {
        match copilot::park_current_context(&[], &[]).await {
            Ok(sess) => {
                let mb = sess.total_memory_bytes as f64 / (1024.0 * 1024.0);
                println!("📦 Successfully parked {} tabs! Reclaimed {:.1} MB RAM.", sess.tab_count, mb);
                return Ok(());
            }
            Err(e) => {
                eprintln!("❌ Error parking context: {e}");
                return Ok(());
            }
        }
    }

    let is_cli_mode = args.iter().any(|a| a == "--cli" || a == "--tui");

    // Check elevation status
    let elevated = is_elevated();
    if !elevated {
        eprintln!("==================================================================");
        eprintln!(" [!] MemoryBlast: Running in Standard User mode.");
        eprintln!("     Some protected system processes may have restricted metrics.");
        eprintln!("     For complete system coverage, launch in an elevated terminal.");
        eprintln!("==================================================================");
    }

    if is_cli_mode {
        run_tui_mode().await
    } else {
        run_gui_mode().await
    }
}

/// Runs the Windows 11 Fluent Desktop GUI mode
async fn run_gui_mode() -> Result<(), Box<dyn std::error::Error>> {
    println!();
    println!("🚀 Starting MemoryBlast Windows 11 Fluent GUI...");

    let (snapshot_tx, snapshot_rx) = crossbeam_channel::unbounded();
    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
    let (bcast_tx, _) = broadcast::channel::<Arc<String>>(32);
    let latest_snapshot: Arc<RwLock<Option<Arc<String>>>> = Arc::new(RwLock::new(None));

    // Spawn sampling engine (default 500ms)
    spawn_sampler(snapshot_tx, cmd_rx, Duration::from_millis(500));

    // Register Win32 Global Hotkey (Ctrl+Shift+F) to Park Current Context
    let latest_snap_for_hotkey = latest_snapshot.clone();
    let _ = copilot::spawn_global_hotkey_listener(Arc::new(move || {
        let latest_snap = latest_snap_for_hotkey.clone();
        tokio::spawn(async move {
            let snap_json = latest_snap.read().ok().and_then(|l| l.clone());
            let mut candidate_pids = Vec::new();
            let mut known_ports = Vec::new();

            if let Some(json) = snap_json {
                if let Ok(snap) = serde_json::from_str::<sampler::SystemSnapshot>(&json) {
                    for p in &snap.processes {
                        if p.is_chromium && (p.proc_type_label == "Renderer" || !p.display_title.is_empty()) {
                            candidate_pids.push((p.pid, p.display_title.clone(), p.working_set_bytes));
                        }
                        if let Some(port) = p.debug_port {
                            if !known_ports.contains(&port) {
                                known_ports.push(port);
                            }
                        }
                    }
                }
            }

            let _ = copilot::park_current_context(&known_ports, &candidate_pids).await;
        });
    }));

    // Spawn Win32 Clipboard listener (secret-safe, frustration loop detection)
    let _ = copilot::spawn_clipboard_listener(Arc::new(move |_event, frustration| {
        if let Some(alert) = frustration {
            let _ = notifications::toast::send_copilot_toast(
                "Frustration Doom-Loop Detected 🦆",
                &alert.suggested_prompt,
                0,
                &alert.repeated_text_snippet,
                false,
            );
        }
    }));

    // Bridge crossbeam channel to Tokio broadcast & latest_snapshot on a dedicated thread
    // to guarantee that no Tokio async worker threads are ever blocked by crossbeam recv()!
    let bcast_tx_clone = bcast_tx.clone();
    let latest_snap_clone = latest_snapshot.clone();
    std::thread::spawn(move || {
        let mut pipeline = telemetry::TelemetryPipeline::new(120, "memoryblast_data");
        let mut learner = knowledge::CognitiveLearner::new();
        let mut copilot_engine = copilot::CopilotEngine::new(12);
        let mut exocortex = copilot::ExocortexCoordinator::new();

        while let Ok(snapshot) = snapshot_rx.recv() {
            // 1. Ingest into Polars telemetry pipeline & compute anomaly metrics
            let anomalies = pipeline.ingest(&snapshot);

            // 2. Build map of leak velocities
            let leak_velocities: std::collections::HashMap<u32, f64> = anomalies
                .iter()
                .map(|a| (a.pid, a.leak_velocity_mb_min))
                .collect();

            // 3. Cognitive behavioral evaluation of processes & tabs
            let profiles = learner.evaluate_snapshot(
                &snapshot.processes,
                &leak_velocities,
                snapshot.timestamp_ms,
            );

            // 4. Commit rate & OOM forecast
            let commit_v = anomalies
                .iter()
                .map(|a| a.leak_velocity_mb_min)
                .filter(|&v| v > 0.0)
                .sum::<f64>();
            let oom_forecast = knowledge::CognitiveLearner::forecast_oom(
                snapshot.mem_stats.committed_bytes,
                snapshot.mem_stats.total_page_file_bytes,
                commit_v,
            );

            // 5. Optimization recommendations
            let recommendations = optimizer::OptimizationAssistant::generate_recommendations(
                &profiles,
                &anomalies,
                &oom_forecast,
            );

            // 6. Interactive Squarified Treemap layout calculation
            let treemap = treemap::build_treemap_from_snapshot(&snapshot, 1200.0, 700.0);

            // 7. Track Chromium Tab Lifecycles & Explorer Lifecycles
            let fg_pid = copilot::get_current_foreground_pid();
            let _ = chromium::global_lifecycle_tracker()
                .lock()
                .map(|mut t| t.evaluate_snapshot(&snapshot.processes, fg_pid));
            let active_explorer = explorer::global_explorer_tracker()
                .lock()
                .map(|mut t| t.evaluate_snapshot())
                .unwrap_or_default();

            // 8. Executive Function & Focus Copilot evaluation
            let drift_assessment = copilot_engine.evaluate_snapshot(&snapshot);
            let copilot_status = notifications::badge::global_badge().get_status();
            let latest_anchor = copilot::load_latest_anchor(std::path::Path::new("memoryblast_data"));
            let executive_briefing = knowledge::ExecutiveBriefing::generate();
            let tab_analytics = knowledge::TabAnalyticsReport::generate();

            // 9. Exocortex Coordinator: unified cognitive state evaluation
            let mem_pct = snapshot.mem_stats.memory_load_percent as f64;
            let clipboard_frustration = copilot::global_clipboard()
                .lock()
                .map(|c| c.recent_frustration_count() > 0)
                .unwrap_or(false);
            let exo_input = copilot::ExocortexInput::from_assessment(
                &drift_assessment,
                mem_pct,
                clipboard_frustration,
            );
            let exocortex_snapshot = exocortex.evaluate(&exo_input);

            // 10. Serialize enriched payload
            let enriched = serde_json::json!({
                "mem_stats": snapshot.mem_stats,
                "processes": snapshot.processes,
                "timestamp_ms": snapshot.timestamp_ms,
                "treemap": treemap,
                "recommendations": recommendations,
                "anomalies": anomalies,
                "oom_forecast": oom_forecast,
                "profiles": profiles,
                "copilot": {
                    "assessment": drift_assessment,
                    "status": copilot_status,
                    "anchor": latest_anchor,
                },
                "knowledge": {
                    "briefing": executive_briefing,
                    "tab_analytics": tab_analytics,
                    "explorer_workspaces": active_explorer,
                },
                "exocortex": exocortex_snapshot
            });

            if let Ok(json) = serde_json::to_string(&enriched) {
                let arc_json = Arc::new(json);
                if let Ok(mut lock) = latest_snap_clone.write() {
                    *lock = Some(arc_json.clone());
                }
                let _ = bcast_tx_clone.send(arc_json);
            }
        }
    });

    // Start local embedded GUI server with instant snapshot caching
    let (_server, url) = GuiServer::start(bcast_tx, latest_snapshot, cmd_tx).await?;

    println!("⚡ Local GUI Server running at: {url}");
    println!("🖥️  Opening native desktop window...");

    // Launch native window in --app mode
    if let Err(e) = launch_desktop_window(&url) {
        eprintln!("⚠️ Notice: {e}");
        eprintln!("👉 You can also open {url} directly in your browser.");
    }

    println!();
    println!("💡 Tip: To run the Terminal TUI instead, launch with: memoryblast --cli");
    println!("🛑 Press Ctrl+C in this terminal to shut down MemoryBlast.");

    // Keep running until Ctrl+C
    tokio::signal::ctrl_c().await?;
    println!("\n👋 Shutting down MemoryBlast. Goodbye!");

    // Generate evening closure
    let closure = crate::knowledge::closure::DailyClosureSummary::generate(&[]);
    if let Err(e) = closure.persist(std::path::Path::new("memoryblast_data")) {
        eprintln!("⚠️ Failed to save evening closure summary: {}", e);
    } else {
        println!("🌙 Evening session closure saved.");
    }

    Ok(())
}

/// Runs the interactive Ratatui Terminal TUI mode
async fn run_tui_mode() -> Result<(), Box<dyn std::error::Error>> {
    // Set panic hook to ensure terminal is restored if a panic occurs
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
        default_panic(info);
    }));

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Channels for async sampler
    let (snapshot_tx, snapshot_rx) = crossbeam_channel::unbounded();
    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();

    let mut app = App::new();
    let initial_rate = app.current_refresh_rate();

    // Start async sampling worker in background
    spawn_sampler(snapshot_tx, cmd_rx, initial_rate);

    // Initial draw
    terminal.draw(|f| render(f, &mut app))?;

    // Main TUI event loop
    loop {
        // Drain any newly arrived snapshots from sampler
        while let Ok(snapshot) = snapshot_rx.try_recv() {
            app.snapshot = Some(snapshot);
        }

        // Render current state
        terminal.draw(|f| render(f, &mut app))?;

        // Poll for user keyboard input (50ms responsive timeout)
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key_event) = event::read()? {
                handle_key_event(key_event, &mut app, &cmd_tx);
            }
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
