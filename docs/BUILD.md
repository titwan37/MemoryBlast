# Building MemoryBlast

MemoryBlast is a high-performance Windows systems utility built in Rust. It monitors process telemetry and provides an Executive Focus Copilot for developers.

## Prerequisites

1. **Rust Toolchain**: Install via [rustup.rs](https://rustup.rs/). You need the `stable-x86_64-pc-windows-msvc` toolchain.
2. **Windows 11**: The application requires Windows 11 due to the usage of native UI Automation (UIA), COM interfaces, and the Fluent desktop GUI overlay.
3. **Microsoft Visual Studio Build Tools**: Ensure the C++ Build Tools (MSVC v143+) are installed.

## Building from Source

To build a standard release binary:
```powershell
cargo build --release
```

## Running the Application

By default, the application launches with the Fluent Desktop GUI server:
```powershell
cargo run --release
```

If you prefer the Terminal User Interface (TUI) Dashboard:
```powershell
cargo run --release -- --cli
```

### Quick Commands

MemoryBlast provides quick subcommands for instant retrieval:
- `memoryblast.exe brief`: View the Morning & Session Orientation Briefing.
- `memoryblast.exe anchor`: View the "Where you left off" anchor.
- `memoryblast.exe park`: Park all active, non-essential tabs immediately.

## Distributing the App

When distributing the `memoryblast.exe` executable, remember that it creates a `memoryblast_data` folder in its working directory to store local SQLite telemetry (e.g. `knowledge.db`), Parquet analytics, and stash archives.

### Known Limitations
- The GUI requires port 8080 or dynamically allocates one. Ensure it isn't blocked by Windows Firewall.
- To use the advanced Process trim features, MemoryBlast must be run as **Administrator**.
