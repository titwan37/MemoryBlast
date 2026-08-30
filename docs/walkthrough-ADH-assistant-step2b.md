# Walkthrough: Incremental Behavioral Knowledge & Long-Term Memory Engine

We have extended **MemoryBlast** with an **Incremental Behavioral Knowledge & Long-Term Memory Engine** that continuously models user work habits, context-switching velocities, and memory consumption footprints, dynamically calibrating cognitive intervention thresholds and delivering an executive orientation briefing on startup.

---

## 1. Implemented Subsystems & Architecture

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

### 1. Longitudinal Telemetry Schema & Storage
- **Module**: [`src/knowledge/schema.rs`](file:///c:/Dev/MemoryBlast/src/knowledge/schema.rs)
- **Local Sovereignty & WAL Mode**:
  - Embedded SQLite store (`memoryblast_data/knowledge.db`) using WAL mode for zero-latency concurrent writes.
  - Tables implemented:
    1. `behavioral_events`: Window/tab dwell times, context switch spikes, clipboard thrashing, notification actions (`Dwell`, `SwitchSpike`, `ToastResponse`).
    2. `daily_archetype_profiles`: Daily rollups of focus minutes, average/peak active tabs, context switch frequency, and total RAM reclaimed.
    3. `process_memory_curves`: Learned baseline memory and growth/leak rates ($\text{MB/hr}$) for recurring tools (e.g. `rust-analyzer.exe`, `chrome.exe`, `code.exe`).
    4. `learned_user_preferences`: Dynamic thresholds (personalized working memory cap, peak focus windows).

---

### 2. Session Consolidation & Archetyping Engine
- **Module**: [`src/knowledge/consolidator.rs`](file:///c:/Dev/MemoryBlast/src/knowledge/consolidator.rs)
- **Incremental Daily Rollups**:
  - Automatically consolidates real-time events into daily developer productivity profiles.
- **Behavioral Archetyping**:
  - Classifies daily developer work modes:
    - *Deep Coding Flow*: Low switch velocity ($<2.0/\text{min}$), sustained IDE dwell, focused memory.
    - *High-Sprawl Research*: High active tabs ($>12$), moderate switches, broad documentation reading.
    - *Divergent Tab Hoarding*: Rapid tab sprawl with high switch rates ($>4.5/\text{min}$).
    - *Debugging & Thrashing*: High compiler churn and clipboard errors.
    - *Balanced Flow*: Standard baseline operation.
- **Rolling 7-Day Compaction**:
  - Automatically purges raw granular event rows older than 7 days while preserving permanent consolidated daily profiles.

---

### 3. Adaptive Threshold Recalibration
- **Module**: [`src/knowledge/adaptive_engine.rs`](file:///c:/Dev/MemoryBlast/src/knowledge/adaptive_engine.rs)
- **Personalized Working Memory Cap**:
  - Learns from the user's historical capacity: if the user productively maintains $>14$ tabs with low context switching, scales the soft cap upward (from default 12 to 14, 16, 18, 20) to eliminate false-positive nudges.
- **Adaptive Forgotten Loop Threshold**:
  - Calibrates the 90-minute default based on individual task dwell percentiles.
- **Peak Focus Window Discovery**:
  - Identifies optimal daily focus periods (e.g. `09:30 – 11:30`).

---

### 4. Executive Morning & Orientation Briefing
- **Module**: [`src/knowledge/orientation.rs`](file:///c:/Dev/MemoryBlast/src/knowledge/orientation.rs)
- **Executive Orientation Briefing Card**:
  - Synthesizes yesterday's focus time, RAM reclaimed, primary archetype, and waiting parked tabs.
  - Formats clean output across all interfaces:
    - **Windows 11 Fluent GUI**: Top executive briefing hero card in View 5 (Focus Copilot).
    - **Terminal TUI**: Integrated into `render_optimizer_view` in [`src/ui/render.rs`](file:///c:/Dev/MemoryBlast/src/ui/render.rs).
    - **CLI**: Available anytime via `memoryblast.exe brief` or `memoryblast.exe morning`.

---

## 2. API & Ergonomics Integration

- **Server Endpoints** in [`src/gui/server.rs`](file:///c:/Dev/MemoryBlast/src/gui/server.rs):
  - `GET /api/knowledge/briefing`: Delivers JSON payload for the Executive Briefing card.
  - `GET /api/knowledge/profile`: Returns learned adaptive thresholds and focus capacity scores.
  - `GET /api/knowledge/history`: Returns historical 14-day consolidated daily profiles.
- **Enriched SSE Telemetry Snapshot** in [`src/main.rs`](file:///c:/Dev/MemoryBlast/src/main.rs):
  - Ingests `knowledge.briefing` directly into the real-time event stream.
- **CLI Commands**:
  - `memoryblast.exe brief` ➔ Prints the formatted ASCII executive orientation card in any terminal.

---

## 3. Verification & Test Results

All 15 unit tests in the suite passed cleanly:
```
running 15 tests
test copilot::clipboard::tests::test_secret_redaction ... ok
test copilot::clipboard::tests::test_classification ... ok
test copilot::drift::tests::test_working_set_cap_alert ... ok
test treemap::tests::test_squarified_layout_aspect_ratios ... ok
test copilot::fixation::tests::test_tangent_topic_classification ... ok
test knowledge::schema::tests::test_schema_initialization ... ok
test copilot::fixation::tests::test_command_failure_loop ... ok
test knowledge::adaptive_engine::tests::test_adaptive_profile_default ... ok
test knowledge::orientation::tests::test_executive_briefing_generation ... ok
test copilot::clipboard::tests::test_clipboard_manager_and_frustration ... ok
test knowledge::consolidator::tests::test_consolidation_and_archetyping ... ok
test copilot::drift::tests::test_forgotten_loop_detection ... ok
test copilot::anchor::tests::test_resume_anchor_creation ... ok
test copilot::stashing::tests::test_stash_manager_save_and_list ... ok
test analytics::tests::test_leak_velocity_detection ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.76s
```

CLI test execution (`memoryblast.exe brief`):
```
╔════════════════════════════════════════════════════════════════════╗
║ ☀️ MEMORYBLAST EXECUTIVE ORIENTATION BRIEFING                     ║
╠════════════════════════════════════════════════════════════════════╣
║ Good afternoon! Ready for a high-focus flow state today.           ║
║ First session of the day. MemoryBlast is continuously modeling ... ║
║ Peak Focus Window: 09:30 – 11:30                                  ║
║ Adaptive Working Memory Cap: 12 tabs (personalized)               ║
║ Clean working set. MemoryBlast Focus Copilot is active in the b... ║
╚════════════════════════════════════════════════════════════════════╝
```
