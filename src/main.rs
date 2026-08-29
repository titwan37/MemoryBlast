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

mod chromium;
mod gui;
mod proc;
mod sampler;
mod sys;
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
        println!("⚡ MemoryBlast — High-Performance Windows Process Manager");
        println!();
        println!("USAGE:");
        println!("    memoryblast.exe [OPTIONS]");
        println!();
        println!("OPTIONS:");
        println!("    --gui       Launch Windows 11 Fluent Desktop GUI (Default)");
        println!("    --cli       Launch Terminal TUI Dashboard (Ratatui)");
        println!("    --help, -h  Print help information");
        return Ok(());
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

    // Bridge crossbeam channel to Tokio broadcast & latest_snapshot on a dedicated thread
    // to guarantee that no Tokio async worker threads are ever blocked by crossbeam recv()!
    let bcast_tx_clone = bcast_tx.clone();
    let latest_snap_clone = latest_snapshot.clone();
    std::thread::spawn(move || {
        while let Ok(snapshot) = snapshot_rx.recv() {
            if let Ok(json) = serde_json::to_string(&snapshot) {
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
