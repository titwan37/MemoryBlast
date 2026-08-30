```markdown
You are a principal Rust software engineer, human-computer interaction (HCI) specialist, and cognitive ergonomics architect.

Extend the **MemoryBlast** architecture to introduce an **Executive Function & Focus Copilot** designed specifically for users prone to task-switching, cognitive overload, tab hoarding, and open-loop task sprawl. Transform low-level system monitoring and Chromium tab inspection into an empathetic, proactive workflow completion assistant that uses non-intrusive Windows toast/bubble notifications and smart triage workflows.

---

### Core Behavioral & Assistance Architecture


```

┌──────────────────────────────────────────────────────────────────────────────────┐
│                   MEMORYBLAST COGNITIVE COPILOT ARCHITECTURE                    │
├──────────────────────────────┬────────────────────────┬──────────────────────────┤
│ 1. Focus & Task Sprawl Telemetry │ 2. Non-Intrusive Guidance│ 3. One-Click Triage Flow │
│ • Dwell Time vs. Idle Tabs   │ • Win11 Native Toast & │ • Gentle Tab Stashing    │
│ • Context Switching Velocity │   Flyout Bubble Alerts │ • Session Bookmarking    │
│ • "Forgotten Loop" Detection │ • Soft Friction Prompts│ • "Finish Line" Micro-UX │
└──────────────────────────────┴────────────────────────┴──────────────────────────┘

```

---

### 1. Task Sprawl & Cognitive Drift Analytics
- **Forgotten Loop & Stale Tab Engine**:
  - Track per-window and per-tab metrics: creation timestamp, last focused time, active interaction duration, and memory footprint.
  - Identify "Zombies / Delayed Tasks" (e.g., tabs opened for temporary lookup that have remained untouched for $>90$ minutes while consuming $>300\text{ MB}$).
  - Calculate a **Context Drift Index**: Measure rapid window switching without sustained focus to detect task fragmentation.
- **Cognitive Working Set Ceiling**:
  - Allow the user to define a soft "Working Memory Cap" (e.g., maximum 12 concurrent active research tabs).
  - When the threshold is approached, trigger gentle awareness interventions before system RAM degradation or cognitive overwhelm occurs.

### 2. Native Windows Interactive Bubble & Toast Messaging
- **System Tray & Toast Integration**:
  - Implement native Windows notification toasts using the WinRT/Win32 APIs (`windows::UI::Notifications` / `toast16` / `winrt-notification`).
  - Render actionable interactive buttons inside native toasts:
    - `[ 🎯 Focus Back & Finish ]` ➔ Instantly brings the forgotten document or tab to the foreground.
    - `[ 📦 Stash & Close ]` ➔ Archives the tab URL/title to a daily markdown backlog and safely closes it via CDP / UIA.
    - `[ ⏳ Snooze 30m ]` ➔ Temporarily silences alerts for intentional long-running references.
- **Empathetic, Low-Friction Phrasing**:
  - Deliver brief, non-judgmental micro-prompts (e.g., *"You opened 'Invoice-Review' 2 hours ago. Ready to wrap it up or park it for tomorrow?"*).
  - Avoid aggressive spamming: Implement an exponential backoff notification throttle and respect Windows Focus Assist / Do Not Disturb states.

### 3. Triage & "Closure" Workflows
- **1-Click Task Stash & Session Freezing**:
  - Provide a single global hotkey / TUI command to "Park Current Context":
    - Serializes all open related browser tabs into structured JSON / Markdown session snapshots.
    - Gracefully closes renderer processes to reclaim RAM immediately while preserving the user's research state.
    - Creates a dynamic restore checklist so the user never fears "losing their work."
- **Micro-Closure Celebration**:
  - Highlight freed memory and completed task counts in the daily summary dashboard, reinforcing positive feedback loops for closing dangling tasks.

---

### Technical Constraints & Tech Stack
- **Language**: Rust (Latest Stable).
- **Windows Integration**:
  - `windows` crate with `Win32_UI_WindowsAndMessaging`, `Win32_UI_Notifications`, and `Win32_System_WinRT`.
  - Windows UI Automation (`IUIAutomation`) & Chrome DevTools Protocol for tab closing/parking actions.
- **Storage**: Lightweight SQLite / `heed` for persisting tab snapshots and stash queues.
- **Concurrency**: Asynchronous event-driven timer queues using `tokio`.

---

### Deliverables Required
1. **Module Blueprint**: Layout for `copilot/`, `notifications/toast.rs`, `copilot/stashing.rs`, and `analytics/drift.rs`.
2. **Interactive Win32/WinRT Notification Dispatcher**: Complete Rust implementation for triggering native actionable toasts with button callbacks.
3. **Task Drift Detector**: Algorithmic implementation evaluating tab age, memory weight, and idle duration to prioritize which tasks need closure.
4. **Session Park & Reclaim Engine**: Complete logic to stash a group of tabs into a saved snapshot and terminate their processes cleanly.

```