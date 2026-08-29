# Implementation Plan: MemoryBlast Windows TUI Process Manager

MemoryBlast is a high-performance, ultra-low-overhead CLI/TUI process manager in Rust designed for Windows (`x86_64-pc-windows-msvc`). It tracks global host memory allocation (Working Set, Private Commit, Pagefile, Paged/Non-Paged pools) and breaks down Chromium-based multi-process applications (Chrome, Edge, Brave, Arc, Opera, Vivaldi) down to individual tab titles and process types.

## User Guidance Incorporations

1. **Admin Privileges**: MemoryBlast checks token privileges at startup. If not running as admin, it offers an elevation launch prompt / fallback cleanly in user mode, clearly indicating status `[Admin]` vs `[Standard User (Elevate for complete stats)]` in the TUI banner without crashing on `ERROR_ACCESS_DENIED`.
2. **CDP Handling**: Auto-detects `--remote-debugging-port` from the browser process command-line arguments. **Never** restarts or terminates the user's browser instances to enable flags. If the flag is not set, it gracefully leverages window enumeration (`EnumWindows`) and UI Automation / title inspection.
3. **Multi-Chromium Support**: First-class detection and tree grouping for Google Chrome (`chrome.exe`), Microsoft Edge (`msedge.exe`), Brave (`brave.exe`), Arc (`arc.exe`), Opera (`opera.exe`), and Vivaldi (`vivaldi.exe`).

---

## Proposed Architecture

```
                               ┌───────────────────────────┐
                               │   Win32 OS & Kernel APIs  │
                               └─────────────┬─────────────┘
                                             │
               ┌─────────────────────────────┼─────────────────────────────┐
               │                             │                             │
    [System Memory Collector]      [Process Memory Scanner]      [Chromium Inspector]
    • GlobalMemoryStatusEx         • K32GetProcessMemoryInfo     • NtQueryInformationProcess
    • NtQuerySystemInformation     • OpenProcess / ExitCode        (ProcessCommandLineInfo)
      (Paged/Non-Paged Pools)      • GetProcessTimes (CPU delta) • CDP HTTP client (json/list)
                                                                 • EnumWindows / HWND resolve
               │                             │                             │
               └─────────────────────────────┼─────────────────────────────┘
                                             │
                                   [Sampling Engine]
                                   • Differential PID Cache
                                   • Tokio / Worker Thread
                                   • crossbeam-channel
                                             │ (State snapshot)
                                             ▼
                                     [Ratatui TUI]
                    ┌──────────────────────────────────────────────┐
                    │ • Top Header: RAM Gauge & Pool Counters      │
                    │ • Process Table: Tree with Collapsible Nodes │
                    │ • Filter / Search Bar (`/`)                  │
                    │ • Process Termination Prompt (`k`)           │
                    │ • Help / Keybindings Bar                     │
                    └──────────────────────────────────────────────┘
```

---

## Proposed Changes

### Project Foundation & Dependencies

#### [NEW] [Cargo.toml](file:///c:/Dev/MemoryBlast/Cargo.toml)
- Configure release optimizations: `opt-level = 3`, `lto = true`, `codegen-units = 1`, `strip = true`.
- Dependencies:
  - `windows`: Win32 APIs for memory, processes, threads, security tokens, window messaging, and accessibility.
  - `ntapi`: Low-level NT structures and APIs for command-line buffer extraction and pool info.
  - `ratatui` + `crossterm`: Modern terminal dashboard rendering and raw input event handling.
  - `tokio`: Asynchronous runtime for periodic sampling and concurrent CDP HTTP inspection.
  - `crossbeam-channel`: High-performance MPMC/MPSC message passing between sampling thread and TUI rendering loop.
  - `reqwest` + `serde` + `serde_json`: Lightweight CDP JSON endpoint parser.
  - `sysinfo`: Auxiliary host stats fallback.

#### [NEW] [src/main.rs](file:///c:/Dev/MemoryBlast/src/main.rs)
- Entry point initializing console mode, admin privilege detection, panic hook (to restore terminal state on error), Tokio runtime, and starting the main event loop.

---

### Low-Level Win32 & System Stats

#### [NEW] [src/sys/mod.rs](file:///c:/Dev/MemoryBlast/src/sys/mod.rs)
- Exports memory statistics data structures and query functions.

#### [NEW] [src/sys/memory.rs](file:///c:/Dev/MemoryBlast/src/sys/memory.rs)
- `GlobalMemoryStatusEx`: Total physical RAM, available RAM, committed memory, commit limit.
- `NtQuerySystemInformation(SystemPerformanceInformation)`:
  - Paged Pool (bytes)
  - Non-Paged Pool (bytes)
  - System Cache size
- Admin privileges checker via `OpenProcessToken` and `GetTokenInformation(TokenElevation)`.

---

### Process Scanner & Performance Counters

#### [NEW] [src/proc/mod.rs](file:///c:/Dev/MemoryBlast/src/proc/mod.rs)
- Models for `ProcessItem`, `ProcessGroup`, `ProcessType`, and sorting modes (`MemoryDesc`, `CpuDesc`, `PidAsc`, `NameAsc`).

#### [NEW] [src/proc/scanner.rs](file:///c:/Dev/MemoryBlast/src/proc/scanner.rs)
- `CreateToolhelp32Snapshot` / `EnumProcesses` to enumerate all active processes.
- Memory info queries via `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ)`:
  - `K32GetProcessMemoryInfo` → `PROCESS_MEMORY_COUNTERS_EX` to extract `WorkingSetSize` and `PrivateUsage` (Private Commit).
- CPU percentage delta calculation tracking previous `FILETIME` kernel/user times per PID:
  $$\Delta \text{CPU} = \frac{\Delta \text{KernelTime} + \Delta \text{UserTime}}{\Delta \text{SystemTime} \times \text{NumCores}} \times 100\%$$
- Terminate process utility (`TerminateProcess`) when user confirms termination.

---

### Chromium Deep Inspection (Chrome, Edge, Brave, Arc, Opera, Vivaldi)

#### [NEW] [src/chromium/mod.rs](file:///c:/Dev/MemoryBlast/src/chromium/mod.rs)
- Chromium browser identifier matching: `chrome.exe`, `msedge.exe`, `brave.exe`, `arc.exe`, `opera.exe`, `vivaldi.exe`.

#### [NEW] [src/chromium/cmdline.rs](file:///c:/Dev/MemoryBlast/src/chromium/cmdline.rs)
- Extracts full command-line of target process via `NtQueryInformationProcess(ProcessCommandLineInformation)`.
- Parses `--type=`:
  - Absent: Browser Main / Core
  - `renderer`: Webpage Tab or Background App
  - `gpu-process`: GPU Acceleration Process
  - `utility`: Network Service, Audio Service, Storage, etc.
  - `crashpad-handler`: Crash reporter
  - `extension`: Web extension process
- Inspects command line for `--remote-debugging-port=<PORT>`.

#### [NEW] [src/chromium/tab_resolver.rs](file:///c:/Dev/MemoryBlast/src/chromium/tab_resolver.rs)
- **Strategy 1 (CDP)**: If `--remote-debugging-port` is present, query `http://127.0.0.1:<PORT>/json/list` asynchronously. Map target WebSocket URLs and page titles to renderer PIDs / tab titles.
- **Strategy 2 (HWND Window Enumeration)**: Correlate top-level and child HWNDs (`EnumWindows`, `GetWindowThreadProcessId`, `GetWindowTextW`) to extract tab / window titles when CDP is disabled.
- **Strategy 3 (Classification fallback)**: Parse `--extension-process`, `--utility-sub-type`, or domain flags to label tabs nicely.

---

### UI Dashboard (Ratatui & Crossterm)

#### [NEW] [src/ui/mod.rs](file:///c:/Dev/MemoryBlast/src/ui/mod.rs)
- Main TUI application state: selected row, scroll offset, search filter string, expanded tree nodes, active modal (e.g. kill confirm dialog), refresh rate setting.

#### [NEW] [src/ui/dashboard.rs](file:///c:/Dev/MemoryBlast/src/ui/dashboard.rs)
- Top Memory Gauge: In-Use vs Total RAM bar, Committed vs Limit bar, Paged / Non-Paged pool indicators, Privilege indicator.
- Process Table:
  - Tree rendering: Chromium browser instances are parent rows with an expand/collapse toggle (`▼`/`▶`).
  - Indented child rows for each renderer tab, GPU process, and service with dedicated memory footprints.
  - Clear columns: `PID`, `Process Name / Tab Title`, `Type`, `Private RAM`, `Working Set`, `% RAM`, `CPU %`.
- Search & Filter bar overlay.
- Kill Confirmation popup dialog.
- Status and Keybindings footer.

#### [NEW] [src/ui/events.rs](file:///c:/Dev/MemoryBlast/src/ui/events.rs)
- Handles user inputs:
  - `Up` / `Down` / `j` / `k` (when not killing): Navigate process list.
  - `Space` / `Enter`: Expand or collapse tree node.
  - `s`: Cycle sort criteria (Private Memory, Working Set, CPU, PID).
  - `/`: Activate search mode.
  - `k`: Open kill confirmation for highlighted process.
  - `+` / `-`: Accelerate or decelerate sampling rate (250ms, 500ms, 1s, 2s).
  - `q` / `Ctrl+C`: Gracefully exit.

---

## Verification Plan

### Automated / Compilation Verification
1. `cargo check` and `cargo build` in `c:\Dev\MemoryBlast`.
2. Ensure zero compiler warnings and correct linking of Win32/NT libraries (`ntdll`, `psapi`, `user32`, `advapi32`).

### Manual & Interactive Verification
1. Run MemoryBlast in terminal. Verify that:
   - System memory stats match Task Manager / Resource Monitor values (Total RAM, Committed, Paged/Non-Paged Pools).
   - Chrome, Edge, and other Chromium browsers display as grouped trees.
   - Expanding Chrome shows individual child processes with resolved types (Renderer, GPU, Utility).
   - Search/filter (`/`) filters both process names and tab titles in real time.
   - Sort toggling (`s`) re-orders the list instantaneously without UI lag.
   - Non-admin mode gracefully handles access denied without panics.
   - Sampling CPU usage remains ultra-low (<0.5%).
