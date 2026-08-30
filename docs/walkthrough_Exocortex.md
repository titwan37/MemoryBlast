# Walkthrough: Windows File Explorer Lifecycle & Workspace Intelligence Engine

We have extended **MemoryBlast** with an end-to-end **Windows File Explorer (`explorer.exe`) Lifecycle & Workspace Intelligence Engine**, tracking active folder windows and tabs, resolving real directory paths via COM `IShellWindows` and Windows UI Automation, calculating shelf-life and foreground dwell durations, consolidating top workspace projects, and integrating folder stashing into `Ctrl+Shift+F`.

---

## 1. System Architecture & Components

```
Windows Explorer (explorer.exe)
  ├─ Win32 Window Scanner ──▶ EnumWindows matching "CabinetWClass"
  ├─ Address Bar UIA ───────▶ IUIAutomation / UIA_EditControlTypeId ➔ Path extraction
  └─ COM Fallback ──────────▶ IShellWindows / LocationURL (file:///C:/... ➔ C:\...)
                                    │
                                    ▼
                        ExplorerLifecycleTracker
                                    │
               ┌────────────────────┴────────────────────┐
               ▼                                         ▼
       explorer_lifecycles (SQLite)              StashManager Integration
    • Exact T_open & T_close                   • Ctrl+Shift+F snapshots folders
    • Folder path & project name               • Restores open directories
    • Active dwell time & shelf-life           • Reopens via explorer.exe <path>
    • Forgotten folder loops (>90m idle)
```

### 1. Explorer Tab & Path Resolver
- **Module**: [`src/explorer/watcher.rs`](file:///c:/Dev/MemoryBlast/src/explorer/watcher.rs) & [`src/explorer/mod.rs`](file:///c:/Dev/MemoryBlast/src/explorer/mod.rs)
- **Path Resolution Pipeline**:
  - `resolve_open_explorer_folders()`:
    1. Enumerates all visible `CabinetWClass` windows.
    2. Uses Windows UI Automation (`CUIAutomation` / `IUIAutomation`) to inspect the address bar edit control (`UIA_EditControlTypeId`) to extract the canonical directory path.
    3. Handles URI decoding (`file:///C:/Users/...` $\rightarrow$ `C:\Users\...`).
    4. Fallback to window title filesystem path parsing.
- **Lifecycle Tracking**:
  - `ExplorerLifecycleTracker` tracks active windows/tabs across sampling ticks.
  - Measures cumulative foreground dwell duration (`GetForegroundWindow`).
  - Records $T_{\text{open}}$, $T_{\text{close}}$, and flags forgotten folder loops (idle $>90\text{ min}$).

### 2. SQLite Workspace History & Analytics
- **Module**: [`src/knowledge/explorer_history.rs`](file:///c:/Dev/MemoryBlast/src/knowledge/explorer_history.rs)
- **Table `explorer_lifecycles`** in `memoryblast_data/knowledge.db`:
  - Stores `tab_id`, `folder_path`, `project_name`, `opened_at_epoch`, `closed_at_epoch`, `shelf_life_secs`, `active_dwell_secs`, and `is_forgotten_loop`.
- **Workspace Intelligence**:
  - `get_top_workspace_roots(limit)`: Discovers primary active development root directories based on cumulative dwell time.
  - `get_forgotten_explorer_tabs()`: Identifies abandoned folder tabs open for $>90\text{ min}$ without user focus.

### 3. Session Park Integration (`Ctrl+Shift+F`)
- **Module**: [`src/copilot/stashing.rs`](file:///c:/Dev/MemoryBlast/src/copilot/stashing.rs)
- **Folder Stashing**:
  - `park_current_context` now captures active Explorer folders alongside Chromium tabs.
  - Stashed sessions record `is_explorer_folder: true` and embed directories in the generated markdown session checklist (`## 📁 Stashed File Explorer Workspaces`).
- **1-Click Restore**:
  - `restore_session(id)` reopens all stashed Explorer paths using `std::process::Command::new("explorer.exe").arg(&path).spawn()`.

### 4. REST API & Fluent GUI Presentation
- **REST Endpoint** in [`src/gui/server.rs`](file:///c:/Dev/MemoryBlast/src/gui/server.rs):
  - `GET /api/knowledge/explorer-analytics`: Delivers active Explorer folders, top workspace roots, and forgotten tabs.
- **Section 5 in View 5 (Focus Copilot)**:
  - Added "📁 Windows File Explorer Workspaces & Folders" card in [`src/gui/assets/index.html`](file:///c:/Dev/MemoryBlast/src/gui/assets/index.html).
  - Displays project name, full path, status badge (`Active` vs `Idle >90m`), and cumulative focus duration (`⏱️ Xm focused`).

---

## 2. Automated Test Verification

All 23 automated unit tests across the entire crate passed cleanly:
```
running 23 tests
test copilot::clipboard::tests::test_classification ... ok
test chromium::lifecycle::tests::test_extract_domain ... ok
test copilot::fixation::tests::test_tangent_topic_classification ... ok
test copilot::clipboard::tests::test_secret_redaction ... ok
test copilot::drift::tests::test_working_set_cap_alert ... ok
test explorer::watcher::tests::test_clean_address_bar ... ok
test explorer::watcher::tests::test_extract_project_name ... ok
test copilot::fixation::tests::test_command_failure_loop ... ok
test explorer::watcher::tests::test_decode_file_uri ... ok
test treemap::tests::test_squarified_layout_aspect_ratios ... ok
test knowledge::tab_analytics::tests::test_tab_analytics_synthesis ... ok
test knowledge::schema::tests::test_schema_initialization ... ok
test chromium::lifecycle::tests::test_lifecycle_open_and_close ... ok
test knowledge::adaptive_engine::tests::test_adaptive_profile_default ... ok
test knowledge::orientation::tests::test_executive_briefing_generation ... ok
test copilot::drift::tests::test_forgotten_loop_detection ... ok
test copilot::clipboard::tests::test_clipboard_manager_and_frustration ... ok
test knowledge::explorer_history::tests::test_explorer_lifecycle_storage ... ok
test knowledge::tab_history::tests::test_tab_history_lifecycle_storage ... ok
test knowledge::consolidator::tests::test_consolidation_and_archetyping ... ok
test copilot::anchor::tests::test_resume_anchor_creation ... ok
test copilot::stashing::tests::test_stash_manager_save_and_list ... ok
test analytics::tests::test_leak_velocity_detection ... ok

test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.76s
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.76s
```

## 3. Exocortex Cognitive Engine & Evening Closure (Phase 1 & 2)

We extended MemoryBlast into a true proactive Exocortex with real-time cognitive tracking and dynamic UI theming.

### 1. Unified Cognitive State Machine
- **Module**: `src/copilot/exocortex.rs` & `src/copilot/flow_mode.rs`
- **Mechanism**: Correlates drift severity, frustration (via `ClipboardManager`), and context switching (via `DriftAssessment`) into a single `CognitiveState` (`CalmFlow`, `DeepFocus`, `Drifting`, `Overloaded`).
- **Zen Mode**: Suppresses UI alerts if deep focus persists for $>45\text{ mins}$.

### 2. Evening Closure Wrap-Up
- **Module**: `src/knowledge/closure.rs`
- **Mechanism**: Triggered gracefully on `ctrl_c` application shutdown.
- **SQLite Persistence**: Logs daily summaries (memory freed, tabs stashed, flow sessions, and the day's archetype) into `memoryblast_data/knowledge.db`.

### 3. Neuro-Ergonomic UI (Synaptic Pulse)
- **Module**: `src/gui/assets/style.css` & `src/gui/assets/app.js`
- **Feature**: Replaced static theming with dynamic `cognitive-*` classes driven by the SSE `/api/stream` endpoint. Background hues, borders, and animations shift intuitively based on the user's inferred cognitive load. Added dedicated cards for the **Evening Closure** summary and **Zen Mode** banner to `index.html`.
