# ⚡ MemoryBlast

A high-performance, ultra-low-overhead process manager in Rust designed for Windows (`x86_64-pc-windows-msvc`). 
MemoryBlast monitors total host memory allocation in real time and breaks down Chromium-based multi-process applications (**Google Chrome, Microsoft Edge, Brave, Arc, Opera, Vivaldi**) down to their individual tab titles and process types.

MemoryBlast features **dual interfaces**:
1. **Windows 11 Fluent GUI**: A modern, native desktop window modeled after the Windows 11 Task Manager (dark Mica theme, real-time memory heatmaps, 60-second live memory performance graphs, and search).
2. **Terminal TUI Dashboard**: An ultra-fast, keyboard-driven `ratatui` console interface.

---

## Features

### 1. Windows 11 Fluent Desktop GUI (Default)
- **Processes Tab**:
  - **Deep Chromium Inspection**:
  - First-class support for **Chrome, Edge, Brave, Arc, Opera, and Vivaldi**.
  - Identifies process roles via command line arguments (`NtQueryInformationProcess`):
    - 📄 Renderer / Web Page Tab
    - ⚡ GPU Acceleration Process
    - 🌐 Network Service
    - 🔊 Audio Service
    - 🧩 Extension
    - 🛡️ Crashpad Handler
  - **Native Tab Title Resolution (Zero Configuration)**:
    1. **Windows UI Automation (`IUIAutomation`)**: Natively inspects the Chromium Tab Strip (`UIA_TabItemControlTypeId`) and Web Document accessibility tree across all running browser instances. Extracts active and background tab titles without flags or restarts.
    2. **Chrome DevTools Protocol (CDP)**: Non-destructively queries `--remote-debugging-port` if enabled.
    3. **Window Handle Enumeration (`EnumWindows`)**: Fallback matching visible window titles.
  - **Memory Heatbars**: Blue proportional heatbars in the Memory column visually showing high-consumption processes at a glance.
  - **Locate & Focus Tab (`🎯 Focus Tab`)**: Select any tab or process and click **Focus Tab** to restore the browser window to the foreground and switch directly to that exact tab!
  - **Retrigger Detection (`🔍 Detect Tabs`)**: Instant button to re-scan and refresh all browser tab names, titles, and accessibility trees on demand.
  - Interactive search bar to filter processes, PIDs, or tab titles.
  - Safe **End task** confirmation dialog.
- **Performance Tab**:
  - Real-time 60-second scrolling SVG/Canvas memory graph showing hardware commit charge trends.
  - Memory Composition breakdown (In Use, Modified, Standby, Free, Paged/Non-Paged pool).
  - Hardware specifications summary (Physical RAM, Speed, Hardware reserved).

### 2. Terminal TUI Dashboard (`--cli`)
- Full console interface powered by `ratatui` and `crossterm`.
- Physical RAM and Commit gauges with adaptive color grading.
- Tree expand/collapse (`Space`/`Enter`), sorting cycles (`s`), search (`/`), and kill (`k`).

### 3. Low-Level Win32 & Decoupled 2-Tier Sampling Engine
- **Tier 1 (High-Frequency Memory Loop — 500ms)**:
  - Queries `GlobalMemoryStatusEx`, `SYSTEM_PERFORMANCE_INFORMATION`, and `K32GetProcessMemoryInfo`.
  - Dispatches process snapshots instantly (<10ms execution time) with zero blocking or UI stuttering.
  - Takes instantaneous $\mathcal{O}(1)$ reads from the shared title cache.
- **Tier 2 (Asynchronous Title Enrichment Worker — Background)**:
  - Isolated background pipeline discovering open Chromium tab titles via **Windows UI Automation (`IUIAutomation`)** and CDP.
  - Populates tab names asynchronously without delaying memory statistics.
- **Precise CPU Delta Tracking**: Cumulative process and tree CPU % using `GetProcessTimes`.
- **Graceful Privilege Detection**: Clearly displays `[ADMIN]` vs `[STANDARD USER]` and safely handles restricted system services.

---

## How to Run

### Launch Desktop GUI (Default)
```powershell
cargo run
```
or compile and run the binary:
```powershell
cargo build --release
.\target\release\memoryblast.exe
```

### Launch Terminal TUI Dashboard
```powershell
cargo run -- --cli
```
or:
```powershell
.\target\release\memoryblast.exe --cli
```

---

## TUI Keybindings (in `--cli` mode)

| Key | Action |
|---|---|
| `↑` / `↓` | Navigate process list |
| `PgUp` / `PgDn` | Scroll 10 rows |
| `Home` / `End` | Jump to top / bottom |
| `Space` or `Enter` | Expand / Collapse process tree node |
| `/` | Enter Search / Filter mode |
| `Esc` | Clear search / Cancel dialog |
| `s` | Cycle sort criteria (RAM, Working Set, CPU, PID, Name) |
| `f` | Focus browser window & switch to selected tab |
| `k` | Terminate selected process (opens confirmation dialog) |
| `+` / `=` | Increase refresh rate (250ms, 500ms, 1s, 2s, 5s) |
| `-` / `_` | Decrease refresh rate |
| `r` | Force immediate refresh |
| `q` or `Ctrl+C` | Quit MemoryBlast |
