# ADH Assistant Upgrade Implementation Plan (Updated)

## Goal Description
We will extend **MemoryBlast** with an **Executive Function & Focus Copilot** (ADH Assistant) that proactively assists users prone to task‑switching, cognitive overload, and tab sprawl. The copilot will monitor system‑level telemetry and Chromium tab activity, detect forgotten loops and cognitive drift, and surface non‑intrusive Windows toast/bubble notifications with one‑click triage actions (focus back, stash & close, snooze).

---

## User Review Required
> [!IMPORTANT]
> This upgrade introduces several **breaking changes**:
> - Adds new Rust crates (`windows`, `tokio`, `serde_json`, `reqwest`) and updates the workspace to asynchronous execution.
> - Introduces new modules under `src/copilot/` and `src/notifications/` that require Windows‑specific APIs; the app will no longer compile on non‑Windows targets without conditional compilation.
> - Modifies the GUI entry point (`src/gui/assets/app.js`) to listen for copilot events via a WebSocket.
> - Requires the user to grant the app permission to send native toast notifications (Windows 11 Focus Assist settings may need to be adjusted).
>
> The plan has been adjusted to match your preferences (global hotkey, SQLite persistence, 1‑minute telemetry interval, background‑service UI). Please confirm you are comfortable with these changes.

---

## Open Questions (Answered)
- **Hotkey/Command Interface**: Use global hotkey **Ctrl+Shift+F** for "Park Current Context" (recommended).
- **Persistence Layer**: Use **SQLite** (`rusqlite`) for storing stashed tab snapshots.
- **Telemetry Frequency**: Run the drift detector **every 1 minute**.
- **Integration Point**: Deploy the copilot as a **background service with a minimal status badge** rather than embedding a full UI in Electron.

---

## Proposed Changes
### 1. Cargo & Dependency Updates
- **Add crates**: `windows = "0.56"`, `tokio = { version = "1", features = ["rt-multi-thread", "macros"] }`, `serde = { version = "1", features = ["derive"] }`, `serde_json`, `rusqlite`, `reqwest`.
- Enable `windows` crate features for `Win32_UI_Notifications` and `Win32_System_WinRT`.
- Update `Cargo.toml` with `[target.'cfg(windows)'.dependencies]` sections for platform‑specific code.

### 2. New Module Blueprint
- **src/copilot/**
  - `mod.rs` – public entry point, spawns async tasks.
  - `drift.rs` – implements `ContextDriftDetector` that consumes telemetry from `telemetry/` and Chromium CDP data, outputs `DriftEvent` structs.
  - `stashing.rs` – logic to serialize open tabs into JSON/Markdown snapshots and store them via SQLite.
  - `notifications/`
    - `toast.rs` – native Windows toast creation using `windows::UI::Notifications`.

- **src/notifications/**
  - `toast.rs` – wrapper around WinRT toast APIs, defines actionable button callbacks.
  - `badge.rs` – minimal status badge displayed in the system tray to indicate copilot activity.

### 3. Telemetry Extension
- Extend `telemetry/mod.rs` to emit per‑window/tab metrics: `creation_ts`, `last_focused_ts`, `active_duration`, `memory_mb`.
- Add a new `TabInfo` struct and expose a broadcast channel for the copilot to subscribe.
- Configure the detector to run **every 60 seconds**.

### 4. Drift Detection Algorithm
- Compute **Context Drift Index**:
  ```rust
  drift = (switch_rate / avg_focus_time) * memory_weight;
  ```
- Configurable thresholds in `config.toml`: `drift_threshold = 0.7`, `memory_cap_mb = 12000`.
- Emit `DriftEvent` when a tab exceeds thresholds or matches a "Forgotten Loop" (>90 min idle, >300 MB).

### 5. Background Service & Badge UI
- The copilot runs as a background async task launched from `main.rs`.
- A small system‑tray badge (via `badge.rs`) indicates active monitoring and shows the count of pending alerts.
- No heavy Electron UI embedding; the badge can be clicked to open a minimal overlay window listing current alerts.

### 6. Native Toast Dispatcher
- `toast.rs` provides `send_copilot_toast(event: DriftEvent)` building a `ToastNotification` with two buttons (Focus, Stash) and descriptive text.
- Button activation is handled via a COM callback that forwards the action to the background service, which then uses the CDP to focus or close the tab.

### 7. Global Hotkey for Parking Context
- Register **Ctrl+Shift+F** using the `windows` crate `RegisterHotKey` API.
- Hotkey triggers `stashing::park_current_context()` which gathers all open tabs via CDP, writes a SQLite record (`session_YYYYMMDD_HHMM`), and closes the tabs.
- Add a small notification confirming the park action.

### 8. Session Park & Restore Engine
- `stashing::park_current_context()` stores snapshots in SQLite (`sessions` table) with JSON payload of tab URLs/titles.
- Provide a command‑line sub‑command `memoryblast restore <session_id>` to reload a saved session, reopening tabs via CDP.

### 9. Configuration & Settings UI (Minimal)
- Add a lightweight settings panel (`settings/copilot.html`) reachable from the tray badge:
  - Working Memory Cap.
  - Notification throttle/back‑off.
  - Enable/disable copilot.
- Settings are persisted in `config.toml`.

### 10. Tests & CI
- Unit tests for `drift.rs` (simulate tab streams, verify detection).
- Integration test for `toast.rs` using a mock Windows toast dispatcher.
- CI runs on Windows runners to compile and execute the background service, ensuring no panics.

---

## Verification Plan
### Automated Tests
- `cargo test --package copilot` – validates drift detection logic.
- `cargo test --package notifications` – checks toast payload generation.
- `cargo test --package stashing` – verifies SQLite storage and park/restore logic.

### Manual Verification
- Build the application (`cargo build --release`).
- Launch on Windows 11; verify the tray badge appears.
- Open multiple Chrome tabs, let the app run.
- After >90 min idle and >300 MB memory, confirm a native toast appears with **Focus** and **Stash** actions.
- Press **Ctrl+Shift+F** – ensure all tabs are stashed to SQLite and closed, with a confirmation toast.
- Use `memoryblast restore <session_id>` to reopen stashed tabs.
- Adjust thresholds in `config.toml` and confirm behavior changes.

---

**Next Steps**
1. With preferences incorporated, please approve this updated plan.
2. Upon approval, we will scaffold the new modules, implement the hotkey, and integrate the background badge service.

*Your confirmation will allow us to proceed.*
