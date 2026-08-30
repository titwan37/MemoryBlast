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
- Multi-View System: Toggle between Processes Table, Squarified Treemap, and AI Optimizer views using the `[Tab]` key.

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

### 4. High-Throughput Columnar Telemetry & Polars Pipeline
- **In-Memory Columnar DataFrames**: Uses `polars` for fast representation of Tier-1 metrics and Tier-2 metadata.
- **Rolling Ephemeral Buffers**: Tracks historical system commit charges, OOM pressure, and pool allocations.
- **Parquet Export**: One-click export of time-series telemetry logs to Snappy-compressed Apache Parquet files (`POST /api/export-parquet`).

### 5. Interactive Squarified Treemap Visualization
- **Dynamic Geometry**: Pure Rust squarified algorithm for hierarchical memory mapping.
- **Clustered Hierarchy**: Organizes memory blocks logically from `Host System` ➔ `App Clusters` ➔ `Sub-processes / Tabs`.
- **Cross-UI Support**: Available as a dedicated view in both the Fluent GUI and the Terminal TUI.

### 6. Dynamic Data Science Engine & Statistical Anomaly Detection
- **Real-time Memory Leak Detection**: Calculates process memory leak velocity in MB/min using rolling linear regression slopes.
- **Z-Score Anomaly Detection**: Applies statistical anomaly scoring to identify unusual memory spikes.
- **Categorization**: Classifies behaviors as `Normal`, `ModerateGrowth`, `RapidSpike`, or `SustainedLeak`.

### 7. Behavioral Knowledge Management & Cognitive Learner
- **Semantic Lifecycle Clustering**: Automatically classifies tabs and processes into *Active Task Focus*, *Stale Background Tab*, *Memory Leak Suspect*, *Periodic Background Daemon*, or *Essential System Service*.
- **Embedded SQLite Store**: Persists behavior archetypes, daily footprints, and optimization event logs.
- **OOM Forecasting**: Local online linear regressor computes commit percentages and estimates time-to-limit exhaustion.

### 8. Proactive Optimization Assistant & Working Set Trimming
- **Urgency-Ranked Recommendations**: Generates actionable suggestions (`INFO`, `RECOMMENDED`, `URGENT`) to manage idle tabs and leaking processes.
- **Win32 Memory Reclamation**: Safely purges resident pages to the standby pool using `K32EmptyWorkingSet` with zero process interruption.
- **One-Click Triggers**: Quickly trim RAM via the UI titlebar, recommendation cards, or TUI shortcuts (`[x]`).

### 9. Executive Function & Focus Copilot (ADH Assistant)
A suite of tools designed to prevent task sprawl, context fragmentation, and tab hoarding.
- **Exocortex Cognitive Engine & Flow Mode**: Correlates drift severity, clipboard frustration, and context switching into a unified `CognitiveState`. "Zen Mode" suppresses non-urgent UI alerts during deep focus (>45 mins).
- **Neuro-Ergonomic UI (Synaptic Pulse)**: Dynamic CSS theming that shifts UI hues and animations in real-time based on the user's inferred cognitive load (Calm, Focus, Drifting, Overloaded).
- **Context Parking (`Ctrl+Shift+F`)**: Press `Ctrl+Shift+F` globally to instantly park the current context and freeze open tabs. Generates a Markdown checklist before closing them to free RAM, with 1-Click Rehydration.
- **Chromium Tab & File Explorer Lifecycle Tracker**: Records opening/closing timestamps, tracks active folder windows, calculates shelf-life duration, and synthesizes long-term utility patterns (Graveyard Ratio, Ephemeral vs. Anchor domains).
- **Morning Briefing & Evening Closure**:
  - `Morning Briefing`: Offers a daily "Where You Left Off" orientation summary.
  - `Evening Closure`: Auto-generates a daily session wrap-up summarizing reclaimed memory and resolved tabs.

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
