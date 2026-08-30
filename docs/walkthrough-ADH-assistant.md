# Walkthrough: Executive Function & Focus Copilot (ADH Assistant)

We have extended the **MemoryBlast** architecture to introduce an **Executive Function & Focus Copilot** designed specifically for users prone to task-switching, cognitive overload, tab hoarding, and open-loop task sprawl, adhering to the specifications in [`docs/roadmap-ADH-assistant.md`](file:///c:/Dev/MemoryBlast/docs/roadmap-ADH-assistant.md).

---

## 1. Architecture Overview

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                      MEMORYBLAST EXECUTIVE FOCUS COPILOT ARCHITECTURE                  │
├───────────────────────────────┬───────────────────────────────┬────────────────────────┤
│ 1. Task Sprawl & Drift Engine │ 2. Non-Intrusive Guidance     │ 3. One-Click Triage    │
│ • Dwell Time vs. Idle Tabs    │ • Win11 Native Interactive    │ • Stash & Freeze Tabs  │
│ • Context Switching Velocity  │   Toast & Bubble Prompts      │ • SQLite Checklist     │
│ • "Forgotten Loop" Detection  │ • Empathetic Micro-Prompts    │ • [Ctrl+Shift+F]       │
│ • Cognitive Working Set Cap   │ • Throttle & Snooze (30m)     │ • Closure Celebration  │
└───────────────────────────────┴───────────────────────────────┴────────────────────────┘
```

---

## 2. Implemented Modules & Blueprint

### 1. Task Sprawl & Cognitive Drift Analytics
- **File**: [`src/copilot/drift.rs`](file:///c:/Dev/MemoryBlast/src/copilot/drift.rs)
- **Forgotten Loop & Stale Tab Engine**:
  - Tracks interaction dwell time, peak memory, and last-active timestamps across all detected windows and Chromium tabs.
  - Identifies abandoned open tasks: tabs untouched for $>90\text{ minutes}$ consuming $>300\text{ MB}$ of RAM.
  - Computes an individual **Cognitive Cost Score**:
    $$\text{CognitiveCost} = \left(\frac{\text{IdleMinutes}}{30}\right) \times \left(\frac{\text{MemoryMB}}{250}\right)$$
- **Context Drift Index Calculation**:
  - Quantifies task fragmentation by measuring rapid window-switching velocity without sustained focus:
    $$\text{ContextDriftIndex} = \left(\frac{\text{SwitchesPerMin}}{\max(\text{AvgFocusSecs}, 5.0)}\right) \times \left(\frac{\text{ActiveTabs}}{\text{WorkingMemoryCap}}\right) \times 10$$
  - Categorizes severity into: `Calm`, `Focused Flow`, `Elevated Drift`, and `Task Fragmentation`.
- **Cognitive Working Set Ceiling**:
  - Configurable soft ceiling (default: 12 tabs).
  - Warns the user when nearing capacity ($\ge 10$) and triggers gentle intervention when exceeded ($\ge 12$).

### 2. Native Windows Interactive Toast Messaging
- **File**: [`src/notifications/toast.rs`](file:///c:/Dev/MemoryBlast/src/notifications/toast.rs)
- **Windows 11 Actionable Notifications**:
  - Built using WinRT (`windows::UI::Notifications` and `windows::Data::Xml::Dom`).
  - Includes actionable buttons directly inside the native toast:
    - `[ 🎯 Focus & Finish ]` ➔ Instantly brings the target tab / window to the foreground.
    - `[ 📦 Stash & Close ]` ➔ Saves tab URL & metadata to SQLite and terminates the renderer process.
    - `[ ⏳ Snooze 30m ]` ➔ Silences alerts for that task for 30 minutes.
  - Safe, non-panicking PowerShell fallback ensuring toasts appear even in unpackaged desktop environments.
- **Empathetic Micro-Prompts**:
  - Brief, supportive phrasing: *"You opened '{title}' 95 minutes ago consuming 450 MB. Ready to wrap it up or park it for tomorrow?"*
- **Notification Throttle & Snooze Manager**:
  - Exponential backoff throttling (minimum 3 minutes between automated prompts) to eliminate notification fatigue.

### 3. Session Park & Reclaim Engine
- **File**: [`src/copilot/stashing.rs`](file:///c:/Dev/MemoryBlast/src/copilot/stashing.rs)
- **Global Hotkey `Ctrl+Shift+F`**:
  - Dedicated background thread listening via Win32 `RegisterHotKey` (`MOD_CONTROL | MOD_SHIFT`, `VK_F`).
  - Pressing `Ctrl+Shift+F` anywhere in Windows immediately triggers `park_current_context()`.
- **Session Freezing & Reclaim**:
  - Gathers all open Chromium tabs (via CDP `/json/list` and UIA window enumeration).
  - Serializes tabs into SQLite (`memoryblast_data/sessions.db`).
  - Automatically exports a human-readable Markdown checklist to `memoryblast_data/stashes/session_<id>.md`.
  - Gracefully closes tabs via CDP `/json/close/{id}` and terminates candidate renderer processes.
  - Immediately updates global memory reclamation metrics.
- **Micro-Closure Celebration**:
  - Fires celebration toast: *"🎉 Parked {N} tabs and reclaimed {MB} MB RAM! Your active working set is refreshed."*
- **Session Rehydration**:
  - 1-click restoration endpoint (`POST /api/copilot/restore`) and CLI command (`memoryblast.exe restore <id>`) to reopen stashed tab URLs.

### 4. Background Status & Badge Tracker
- **File**: [`src/notifications/badge.rs`](file:///c:/Dev/MemoryBlast/src/notifications/badge.rs)
- Thread-safe `OnceLock` singleton tracking `CopilotStatus`: active tab count, drift index, pending alert count, total tabs stashed, and total RAM liberated.

---

## 3. Server Endpoints & Integration

In [`src/gui/server.rs`](file:///c:/Dev/MemoryBlast/src/gui/server.rs):
- `POST /api/copilot/park`: Parks all open tabs, saves session to SQLite, and frees RAM.
- `POST /api/copilot/action`: Executes triage action (`focus`, `stash`, `snooze`).
- `GET /api/copilot/stashes`: Returns historical list of stashed sessions.
- `POST /api/copilot/restore`: Reopens all tab URLs from a stashed session.
- `GET /api/copilot/status`: Returns current drift index and celebration statistics.

In [`src/main.rs`](file:///c:/Dev/MemoryBlast/src/main.rs):
- CLI support:
  - `memoryblast.exe park` ➔ Parks open tabs immediately from the command line.
  - `memoryblast.exe restore <id>` ➔ Restores saved session.
- Real-time telemetry pipeline integrates Copilot assessment into the SSE broadcast stream.

---

## 4. UI & Ergonomics Integration

- **Titlebar**:
  - Added `[ 📦 Park Context Ctrl+Shift+F ]` button for immediate 1-click execution.
- **Sidebar**:
  - Added `🎯 Focus Copilot` navigation item with a dynamic red alert pill displaying pending open loops count.
- **Focus Copilot View (`#view-copilot`)**:
  - **KPI Banner**:
    - Context Drift Index gauge with color-coded severity badges (`Calm Flow`, `Elevated Drift`, `Task Fragmentation`).
    - Cognitive Working Set progress bar showing `Active Tabs / 12 Soft Cap` with warning colors when approaching limit.
    - Celebration Card displaying cumulative RAM reclaimed and cleanly finished tasks.
  - **Forgotten Loops Grid**:
    - Interactive cards showing tab title, idle time, RAM usage, cognitive cost, and empathetic prompts.
    - Action buttons: `[ 🎯 Focus ]`, `[ 📦 Stash ]`, `[ ⏳ Snooze 30m ]`.
  - **Stashed Sessions History**:
    - Displays past saved sessions with timestamps, tab counts, and RAM reclaimed.
    - `[ 🔄 Restore Session ]` button to rehydrate parked research without leaving the dashboard.
- **Terminal TUI**:
  - Added `[z]` keyboard shortcut to park tabs directly inside the Ratatui dashboard.

---

## 5. Verification & Tests

- **Unit Tests**:
  - `test_forgotten_loop_detection` in `drift.rs`: Validates dwell time and idle duration threshold triggers.
  - `test_working_set_cap_alert` in `drift.rs`: Validates soft ceiling alerts when tab count exceeds capacity.
  - `test_stash_manager_save_and_list` in `stashing.rs`: Validates SQLite storage, session serialization, and markdown checklist generation.
- **Build Status**:
  - `cargo check` passes with exit code 0 on Windows.
