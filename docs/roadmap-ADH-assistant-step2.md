You are an expert Rust systems programmer and cognitive ergonomics architect.

Extend the **MemoryBlast Focus Copilot** with an **Executive Function Developer Suite** tailored for developers with ADHD.

### Specifications to Implement

1. **Clipboard Telemetry & Thrashing Engine (`src/copilot/clipboard.rs`)**:
   - Register Win32 clipboard format listener (`AddClipboardFormatListener`).
   - Store historical clipboard items in SQLite with deduplication and regex classification (`StackTrace`, `CodeSnippet`, `ShellCommand`, `URL`).
   - Detect repeated copy operations of identical errors/traces within short timeframes (Frustration Loop Detection) and emit a gentle triage event.

2. **Rabbit Hole & Fixation Detector (`src/copilot/fixation.rs`)**:
   - Monitor Chromium navigation streams and terminal exit codes.
   - Detect burst tab sprawl (divergent domains opened rapidly) or repetitive failed build cycles.
   - Surface a low-friction option to park the tangent tabs into a "Later/Curiosity" backlog without breaking the current primary task focus.

3. **Task Resume Anchor (`src/copilot/anchor.rs`)**:
   - When parking context via `Ctrl+Shift+F`, capture:
     - Open Chromium tabs & titles
     - Active Git branch and most recently modified files
     - Last copied clipboard snippet
   - Implement `memoryblast resume` to generate a 2-line "Where you left off" orientation card.

Provide the complete Rust module definitions, Win32 event hooks, SQLite schemas, and integration points with the existing background sampling loop.


For a software developer with ADHD / executive function challenges, the biggest productivity drain is not lack of drive—it is **working memory overload, friction in task transitions, and invisible context decay**.

---

### 1. Intelligent Clipboard Telemetry & "Lost Trail" Recovery

Developers with ADHD frequently copy code snippets, error traces, or API keys, get interrupted, and lose track of what they were doing or overwrite their clipboard buffer.

* **Clipboard Ring & Semantic Clustering (Win32 `AddClipboardFormatListener`)**:
* Track clipboard history locally in SQLite without polling overhead.
* Auto-classify snippets: `Stack Trace`, `Git Command`, `Regex`, `API Endpoint / JSON`, `Doc Link`.


* **Redundancy & Thrashing Detector**:
* **Frustration / Loop Detection**: If the user copies the exact same compilation error or Stack Overflow query $>3$ times within 10 minutes, the Copilot triggers a gentle micro-toast:
*`"Stuck on this Rust compiler error? [ 💡 Break problem into micro-steps ] [ ⏸ Take a 5m breather ]"`*


* **Clipboard Buffer Safety**:
* When copying something while a previous unpasted snippet is $<2$ minutes old, auto-stash it into a temporary quick-access shelf (`Ctrl+Shift+V` flyout) so context is never accidentally destroyed.



---

### 2. IDE & Terminal Context Anchoring ("Where Was I?")

When returning from an interruption (Slack message, coffee break, or rabbit hole), regaining deep-code context requires heavy cognitive reloading.

* **Terminal Command Loop & Failure Monitor**:
* Monitor shell process exits (`cargo check`, `pytest`, `npm build`).
* If a command fails consecutively $>3$ times with rapid re-execution, flag it as a **"Fixation Loop"** and suggest parking the sub-task or inspecting the diff.


* **Resume Anchor (Smart Working State Snapshot)**:
* Along with tab titles, record the active Git branch and recent modified files via low-level file watcher APIs (`ReadDirectoryChangesW`).
* When resuming a parked session (`memoryblast restore <id>`), Copilot renders a 2-line summary:
*`"You were implementing the SQLite serializer on branch 'feature/stash' (last modified: stashing.rs:142)."`*



---

### 3. Hyperfocus Guard & Gentle Time-Perception Anchors

Time blindness causes hours to evaporate on low-priority tangential tasks (e.g., refactoring styling or tweaking configs).

* **Passive Focus Pacing (No manual timer required)**:
* Detect continuous sustained input without window switching for $>60$ minutes.
* Display a subtle, non-intrusive floating water-level/pulse badge in the system tray indicating biological needs (hydration, eye break) without breaking flow with loud alarm buzzers.


* **Micro-Step Decomposer (`Ctrl+Shift+D`)**:
* For "Task Paralysis" when starting an overwhelming ticket, allow the user to type one sentence into a floating quick-input. The copilot breaks it down into 3 atomic, low-activation micro-tasks (e.g., *1. Create struct, 2. Add trait definition, 3. Run cargo check*).



---

### 4. Summary Matrix of Proposed ADHD Support Modules

| Module | Low-Level Mechanism | Cognitive Trigger | Assistive Action |
| --- | --- | --- | --- |
| **Clipboard Intelligence** | `AddClipboardFormatListener` + Regex Parser | Redundant copies ($>3\times$) or lost unpasted buffer | Stash history, suggest micro-break, preserve overwritten text |
| **Fixation Loop Breaker** | Terminal exit codes + rapid command restarts | Repeated compilation failures within $<3$ min | Offer diff check, suggest atomic step breakdown |
| **Rabbit Hole Detector** | Browser CDP inspection + URL domain jump rate | Rapid opening of unrelated search tabs ($>8$ in $5$ min) | Offer `[ 📦 Stash Rabbit Hole ]` to review later |
| **Context Restorer** | Git head + open IDE files + parked tab URLs | Resuming after $>30$ min absence or system lock | Display 1-sentence "Where you left off" anchor card |

---
```markdown
You are a senior Rust systems architect, applied data scientist, and cognitive ergonomics specialist.

Extend the **MemoryBlast** architecture to build an **Incremental Behavioral Knowledge & Long-Term Memory Engine**. The goal is to continuously learn, model, and consolidate user work habits, context switching patterns, and memory consumption footprints over time. Each time MemoryBlast launches or resumes, it must load the consolidated behavioral graph, summarize past working patterns, and dynamically adapt its focus nudges and memory optimization thresholds to the specific user.

---

### Core Objectives & Architecture


```

┌──────────────────────────────────────────────────────────────────────────────────┐
│                   MEMORYBLAST INCREMENTAL BEHAVIORAL ENGINE                      │
├──────────────────────────────┬────────────────────────┬──────────────────────────┤
│ 1. Continuous Telemetry Sink │ 2. Session Consolidation│ 3. Adaptive Copilot &   │
│ • Window/Tab Dwell Times     │ • Daily Archetype Clust│    Executive Morning     │
│ • Memory Leak Signatures     │ • Knowledge Graph Sync │    Orientation Brief     │
│ • Clipboard/Loop Thrashing   │ • Local SQLite Vector/ │ • Dynamic Drift Limits   │
│ • Context Switch Velocity    │   Bayesian Profile     │ • Trended RAM Baselines  │
└──────────────────────────────┴────────────────────────┴──────────────────────────┘

```

---

### 1. Longitudinal Telemetry Collection & Schema (`src/knowledge/telemetry_sink.rs`)
- **Metric Persistence**:
  - Automatically log structured session events into an embedded SQLite database (`rusqlite`):
    - Dwell time per domain / application window.
    - Peak and baseline memory footprint per task cluster (IDE vs. Browser vs. Background Services).
    - Frequency of rapid context-switching spikes ("Task Drift" events).
    - Clipboard thrashing / repeated compiler failure loops.
    - User response to past notifications (e.g., `Stashed`, `Snoozed`, `Ignored`, `Focused`).
- **Privacy & Local Sovereignty**:
  - Keep all modeling 100% on-device and local, with automated rolling compaction (e.g., raw events aggregated into hourly/daily summary tables after 7 days).

---

### 2. Knowledge Consolidation & Clustering (`src/knowledge/consolidator.rs`)
- **Session End / Daily Rollup Engine**:
  - When a session ends or during idle background cycles, run an incremental consolidation job:
    - **Behavioral Archetyping**: Cluster typical work phases (e.g., *Deep Coding Morning*, *High-Sprawl Research Afternoon*, *Divergent Tab Hoarding*).
    - **Dynamic Sensitivity Calibration**: Update individual baseline thresholds (e.g., if the user routinely works productively with 18 tabs open, dynamically increase the soft working memory cap from 12 to 18 to avoid false-positive nudges).
    - **Process Memory Fingerprinting**: Learn typical memory decay curves for recurrent tools (e.g., "Language Server typically leaks 400MB after 3 hours of continuous uptime").

---

### 3. Startup Orientation & Adaptive Copilot (`src/knowledge/orientation.rs`)
- **Executive "Where You Left Off" Briefing**:
  - Upon starting MemoryBlast or unlocking the workstation:
    - Generate a concise, consolidated resume card:
      *`"Welcome back. Yesterday you parked 6 research tabs on [Project X] and reclaimed 1.4 GB. Your peak focus window is typically 09:30–11:30. Ready to restore your session or start fresh?"`*
- **Personalized Proactive Interventions**:
  - Adjust notification cadence and phrasing based on historical responsiveness (e.g., favor subtle status badge pulses over system toasts if toasts were frequently dismissed).

---

### Technical Constraints & Modules to Implement

- **Language**: Rust (Latest Stable).
- **Crates**: `rusqlite`, `polars` (for historical batch rollups), `serde`, `chrono`, `tokio`.
- **Key Modules**:
  1. `src/knowledge/schema.rs`: SQLite migrations for raw events, daily profiles, and learned heuristics.
  2. `src/knowledge/consolidator.rs`: Online incremental aggregation routines.
  3. `src/knowledge/orientation.rs`: Morning / session start briefing generator.
  4. `src/knowledge/adaptive_engine.rs`: Dynamic threshold recalculation based on consolidated user history.

---

### Expected Output
1. Provide the updated SQLite database schema supporting incremental daily rollups and learned profiles.
2. Implement the full Rust code for `consolidator.rs` and `adaptive_engine.rs`.
3. Provide the startup briefing generator (`orientation.rs`) that hooks into both the Windows 11 Fluent GUI and the Ratatui TUI dashboard.

```

Yes, Chromium exposes real-time target lifecycle events through the Chrome DevTools Protocol (CDP) and native Win32 window handles, making it straightforward to record exact timestamps, calculate shelf-life, detect closure causes, and consolidate behavioral insights.

---

### 1. Low-Level Event Extraction

```
Chromium Target Stream ───▶ CDP Target Domain / Win32 UIA
                                  │
      ┌───────────────────────────┼──────────────────────────┐
      ▼                           ▼                          ▼
TargetCreated (Open Time)   TargetDestroyed (Close Time)   Page.lifecycleEvent (Discard/Freeze)

```

* **Opening DateTime ($T_{\text{open}}$)**:
* **Mechanism**: Listen to CDP `Target.targetCreated` or detect new renderer PIDs via the differential PID cache.


* **Payload**: `TargetID`, URL, initial title, and `Utc::now()`.


* **Closing DateTime ($T_{\text{close}}$)**:
* **Mechanism**: Hook `Target.targetDestroyed` or detect renderer process exit codes (`GetExitCodeProcess`).


* **Payload**: `TargetID` and `Utc::now()`.


* **Shelf-Life Duration ($\Delta T$)**:

$$\text{ShelfLife} = T_{\text{close}} - T_{\text{open}}$$


* **Cause of Closure / Suspension Classification**:
* **`USER_EXPLICIT`**: Tab closed while active in the foreground (`Target.targetDestroyed` triggered immediately after focus).
* **`COPILOT_PARKED`**: Tab closed via MemoryBlast triage (`Ctrl+Shift+F` or stash action).


* **`BROWSER_DISCARD`**: Tab frozen/discarded by Chromium's native memory saver (`Page.lifecycleEvent` $\rightarrow$ `frozen` or `discarded`).
* **`OOM_CRASH`**: Renderer terminated with non-zero exit codes (e.g., `STATUS_INTEGER_DIVIDE_BY_ZERO` or `STATUS_FATAL_APP_EXIT`).
* **`STALE_CLEANUP`**: Tab closed after $>120$ minutes of zero user interaction.



---

### 2. Behavioral Knowledge Consolidation

By aggregating these metrics over days and weeks in SQLite, MemoryBlast derives several cognitive and system indicators:

* **Tab Mortality & Utility Curve**:
* Calculates the percentage of tabs that survive past 30 minutes versus those closed within 2 minutes (quick lookups vs. intentional research).


* **"Graveyard Ratio"**:
* Measures the ratio of tabs that are closed intentionally versus tabs that sit untouched until system reboot or forced OOM reclamation.




* **Context Decay Rate**:
* Identifies the specific shelf-life threshold beyond which the user virtually never returns to a tab (e.g., *“Tabs idle $>45$ min have a 96% abandonment probability”*).



---

### Generative Implementation Prompt

```markdown
You are an expert Rust systems programmer, CDP protocol engineer, and behavioral data scientist.

Extend the **MemoryBlast Knowledge Engine** to track exact Chromium tab lifecycles (opening/closing timestamps, shelf-life duration, and closure/suspension causes) and consolidate behavioral patterns over time.

---

### Specifications to Implement

1. **CDP Target Lifecycle Listener (`src/chromium/lifecycle.rs`)**:
   - Subscribe to CDP events: `Target.targetCreated`, `Target.targetDestroyed`, `Target.targetInfoChanged`, and `Page.lifecycleEvent`.
   - Record and log:
     - `target_id`: String
     - `opened_at`: `DateTime<Utc>`
     - `closed_at`: `Option<DateTime<Utc>>`
     - `active_dwell_secs`: Cumulative seconds where window/tab was in foreground.
     - `closure_cause`: `Enum (UserManual, CopilotStashed, BrowserMemorySaver, RendererCrash, SessionShutdown)`.
     - `peak_memory_mb`: Maximum private bytes recorded during shelf-life.

2. **SQLite Schema & Persistence (`src/knowledge/tab_history.rs`)**:
   - Create a `tab_lifecycles` table storing target metadata, lifetime duration, and closure causes.
   - Implement daily aggregation routines computing:
     - Median and mean shelf-life duration by domain.
     - Ratio of active vs. stale tabs.
     - Percentage of closures initiated by manual triage vs. passive hoarding.

3. **Behavioral Insight Synthesizer (`src/knowledge/tab_analytics.rs`)**:
   - Generate consolidated user patterns:
     - **"Ephemeral vs. Anchor Tabs"**: Classify domains that serve as persistent tools (IDE, Jira) vs. transient noise (search results, redirects).
     - **Context Decay Threshold**: Compute the optimal idle-time threshold ($\tau$) for triggering triage nudges based on historical return rates.

Provide the complete Rust module code, SQL schemas, CDP event loop handler, and consolidation analytics functions.

```

Yes, Windows exposes native COM Shell APIs and UI Automation (UIA) hooks that allow MemoryBlast to monitor File Explorer tabs and windows in real time, capture start/stop timestamps, track folder paths, and consolidate behavioral insights.

---

### 1. Low-Level Extraction Mechanisms (Win32 & COM)

```
Windows Explorer (explorer.exe)
  ├─ Legacy / Window Mode ────▶ IShellWindows COM Collection ➔ IFolderView ➔ IPersistFolder2 (Extract Path)
  └─ Modern Win11 Multi-Tab ──▶ UI Automation (IUIAutomation) ➔ UIA_TabItemControlTypeId / Address Bar Value

```

* **`IShellWindows` & `IFolderView` (COM Shell API)**:
* Enumerate active Shell instances via `CLSID_ShellWindows`.
* Query `IServiceProvider` $\rightarrow$ `STopLevelBrowser` $\rightarrow$ `IShellBrowser` $\rightarrow$ `IFolderView` $\rightarrow$ `IPersistFolder2::GetCurFolder`.
* Resolve the `ITEMIDLIST` (PIDL) to a human-readable file system path via `SHGetPathFromIDListW`.


* **Windows UI Automation (`IUIAutomation`)**:
* For Windows 11 tabbed Explorer frames, inspect the `UIA_TabItemControlTypeId` elements inside `CabinetWClass`.
* Extract the tab display title and current breadcrumb address bar (`UIA_EditControlTypeId` / `ValuePattern`).


* **Lifecycle Timestamps**:
* **$T_{\text{open}}$**: Captured when a new tab/window handle is registered in the differential cache.
* **$T_{\text{close}}$**: Recorded upon window handle destruction or when the tab element leaves the UIA tree.
* **Navigation Events**: Detect path changes within the same tab using `DShellWindowsEvents::OnNavigate` or change polling.



---

### 2. Behavioral Knowledge Consolidation from Explorer Paths

By aggregating folder navigation history into SQLite, the engine derives key developer habits:

* **Workspace & Project Anchor Detection**:
* Identifies top active code repositories (e.g., `C:\Dev\MemoryBlast\`, `D:\Projects\`) based on dwell time and opening frequency.


* **Transient vs. Stale Folder Hoarding**:
* Flags abandoned folders (e.g., `Downloads`, `AppData\Local\Temp`, or buried subdirectories) open for $>2$ hours without active focus.


* **Multi-Tab Workspace Stashing (`Ctrl+Shift+F`)**:
* Along with Chromium tabs, saves open folder paths into the snapshot markdown checklist, allowing 1-click restore via `explorer.exe <path>`.



---

### Generative Implementation Prompt

```markdown
You are an expert Rust Windows systems engineer and COM/Win32 specialist.

Extend the **MemoryBlast Behavioral Knowledge Engine** to track **Windows File Explorer (`explorer.exe`)** window and tab lifecycles, extract active directory paths, and consolidate workspace patterns over time.

---

### Specifications to Implement

1. **Explorer Tab & Path Resolver (`src/explorer/mod.rs` & `src/explorer/watcher.rs`)**:
   - Initialize COM (`CoInitializeEx`) and query `IShellWindows` (`CLSID_ShellWindows`).
   - Traverse `IWebBrowserApp` / `IShellBrowser` / `IFolderView` / `IPersistFolder2` to resolve active folder paths from `ITEMIDLIST` (`SHGetPathFromIDListW`).
   - Use Windows UI Automation (`IUIAutomation`) to inspect Windows 11 multi-tab Explorer strips (`CabinetWClass` tab items).
   - Track:
     - `tab_id` / `hwnd`: Unique identifier
     - `folder_path`: Resolved absolute directory path (e.g., `C:\Dev\MemoryBlast`)
     - `opened_at`: `DateTime<Utc>`
     - `closed_at`: `Option<DateTime<Utc>>`
     - `active_dwell_secs`: Time the folder was actively in the foreground.

2. **Database Schema & Persistence (`src/knowledge/explorer_history.rs`)**:
   - Create SQLite table `explorer_lifecycles` logging tab timestamps, paths, and dwell durations.
   - Aggregate project clustering:
     - Detect top workspace root directories.
     - Detect forgotten Explorer tabs (idle $>90\text{ min}$, no interaction).

3. **Session Park Integration (`src/copilot/stashing.rs`)**:
   - Update `park_current_context` (`Ctrl+Shift+F`) to snapshot open Explorer paths alongside Chromium tabs.
   - Add restore support to reopen saved folder paths via `ShellExecuteW`.

Provide the complete Rust module definitions, COM wrapper routines, and SQLite persistence queries.

```