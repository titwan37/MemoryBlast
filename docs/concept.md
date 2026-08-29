You are an expert Rust systems programmer and low-level Windows OS specialist. 

Build a high-performance, ultra-low-overhead CLI/TUI process manager in Rust designed for Windows (`x86_64-pc-windows-msvc`) that monitors total host memory allocation in real time and instantly identifies memory hogs—specifically breaking down multi-process applications like Google Chrome down to their individual tab titles and process types.

---

### Core Objectives & Architecture

1. **System & Memory Tracking (Kernel / Win32 Level)**
   - Query global system memory statistics using native Win32 APIs (`GlobalMemoryStatusEx`, `NtQuerySystemInformation`, or `sysinfo` / `windows` crate).
   - Track **Working Set (Private & Total)**, **Committed Memory (Pagefile usage)**, and **Paged/Non-Paged Pools**.
   - Sort processes instantaneously by private commit / working set usage in descending order.

2. **Chrome / Chromium Deep Inspection & Tab Title Resolution**
   - Detect parent and child `chrome.exe` (or Chromium-based) processes.
   - Differentiate process types (e.g., Browser Core, GPU Process, Network Service, Renderer / Tab, Extension).
   - Resolve renderer processes to human-readable tab titles / URLs using lightweight inspection:
     - Connect via Chrome DevTools Protocol (CDP) debugging port (if available via `--remote-debugging-port`), OR
     - Query UI Automation / Accessibility tree APIs (`UIAutomationClient` / `IAccessible`), OR
     - Correlate window handles (`EnumWindows` / `GetWindowThreadProcessId` / `GetWindowTextW`) and command-line arguments (`NtQueryInformationProcess` with `ProcessCommandLineInformation` to parse `--type=renderer`).
   - Group Chrome processes dynamically under an expandable tree or display each tab as an attributed sub-item with its exact RAM footprint.

3. **Performance & Concurrency Requirements**
   - Fast sampling loop: poll every 500ms–1s with minimal CPU impact (<0.5%).
   - Avoid blocking the UI thread; use an asynchronous runtime (`tokio`) or dedicated worker threads with `crossbeam-channel` / `mpsc`.
   - Implement differential updates (cache PIDs, inspect process metadata/command lines only on spawn/PID change).

4. **User Interface (TUI / Fast CLI)**
   - Use `ratatui` (or `crossterm`) for a responsive terminal dashboard.
   - Include:
     - **Top Banner**: Host RAM bar (In Use, Available, Committed vs. Limit, Paged/Non-Paged).
     - **Process Table**: Columns for `PID`, `Process Name / Tab Title`, `Process Type`, `Private RAM (MB/GB)`, `Working Set`, `% Total Memory`, `CPU %`.
     - **Controls / Keybindings**: Filter/Search (`/`), Sort toggle (Memory vs. CPU), Expand/Collapse process tree (`Space`/`Enter`), Kill process (`k`), Refresh rate adjustment.

---

### Technical Constraints & Tech Stack
- **Language**: Rust (Latest Stable edition).
- **Crates to Use**:
  - `windows` or `windows-sys` (for fast native Win32/NT API access).
  - `ratatui` + `crossterm` (for the TUI dashboard).
  - `sysinfo` (for baseline hardware counters if preferred alongside native calls).
  - `rayon` or `tokio` (for parallel process inspection).
- **Error Handling**: Graceful fallback when accessing privileged/protected system processes (e.g., handle `ERROR_ACCESS_DENIED` cleanly).

---

### Expected Output
1. Provide the complete project structure (`Cargo.toml`, module layout).
2. Write production-ready, fully commented Rust code implementing:
   - System memory stats collector.
   - Process scanner with Win32 process memory queries (`K32GetProcessMemoryInfo`).
   - Chromium command-line & tab resolution logic.
   - Ratatui-based responsive rendering loop.
3. Include instructions for building, running with appropriate permissions, and enabling Chrome inspection hooks.