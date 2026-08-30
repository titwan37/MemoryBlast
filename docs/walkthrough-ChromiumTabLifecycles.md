# Walkthrough: Chromium Tab Lifecycle Tracker & Behavioral Shelf-Life Engine

We have extended **MemoryBlast** with an end-to-end **Chromium Tab Lifecycle & Behavioral Shelf-Life Engine**, tracking exact opening/closing timestamps, calculating tab survival curves ($\Delta T = T_{\text{close}} - T_{\text{open}}$), categorizing closure causes, and synthesizing cognitive tab usage indicators (Mortality Curve, Graveyard Ratio, Anchor vs. Ephemeral Domains, and Context Decay Threshold $\tau$).

---

## 1. System Architecture & Components

```
Chromium Target Stream ───▶ CDP Target Domain / Win32 Renderer Scanner
                                      │
            ┌─────────────────────────┼──────────────────────────┐
            ▼                         ▼                          ▼
   TargetCreated (Open Time)   TargetDestroyed (Close Time)   Page.lifecycleEvent (Discard)
            │                         │                          │
            └─────────────────────────┼──────────────────────────┘
                                      ▼
                        ChromiumLifecycleTracker
                                      │
               ┌──────────────────────┴──────────────────────┐
               ▼                                             ▼
       tab_lifecycles (SQLite)                      TabAnalyticsReport
    • Exact T_open & T_close                      • Tab Mortality Curve (<2m vs >30m)
    • Shelf-life duration (ΔT)                    • "Graveyard Ratio"
    • Active dwell time & peak RAM                • Context Decay Threshold (τ)
    • Closure Cause Classification                • Ephemeral vs Anchor Domains
```

### 1. Real-Time Lifecycle Tracker
- **Module**: [`src/chromium/lifecycle.rs`](file:///c:/Dev/MemoryBlast/src/chromium/lifecycle.rs)
- **Differential State Machine**:
  - Tracks open tabs across sampling ticks to calculate exact shelf-life $\Delta T$.
  - Measures cumulative foreground dwell duration and peak working set consumption.
  - Classifies closure and suspension causes:
    - `UserManual`: Tab closed while active or recently in foreground.
    - `CopilotStashed`: Tab parked/closed via MemoryBlast triage (`Ctrl+Shift+F` or `[ 📦 Stash ]`).
    - `BrowserMemorySaver`: Tab frozen or discarded by Chromium's native memory saver.
    - `RendererCrash`: Sudden process termination with non-zero exit code.
    - `SessionShutdown`: Closed upon browser or system exit.
    - `StaleCleanup`: Closed after $>120$ minutes of zero interaction.

### 2. SQLite Tab History & Persistence
- **Module**: [`src/knowledge/tab_history.rs`](file:///c:/Dev/MemoryBlast/src/knowledge/tab_history.rs)
- **Table `tab_lifecycles`** in local `knowledge.db`:
  - Fields: `target_id`, `domain`, `url`, `title`, `opened_at_epoch`, `closed_at_epoch`, `shelf_life_secs`, `active_dwell_secs`, `closure_cause`, `peak_memory_mb`.
  - Non-blocking WAL-mode concurrent writes.

### 3. Behavioral Insight Synthesizer
- **Module**: [`src/knowledge/tab_analytics.rs`](file:///c:/Dev/MemoryBlast/src/knowledge/tab_analytics.rs)
- **Cognitive & System Indicators**:
  - **Tab Mortality & Utility Curve**: % tabs surviving $< 2\text{ min}$ (quick lookups) vs $2\text{--}30\text{ min}$ (active tasks) vs $> 30\text{ min}$ (persistent references).
  - **Graveyard Ratio**: Measures proportion of tabs that sit untouched until passive cleanup or memory-saver discard vs. intentional triage.
  - **Anchor vs. Ephemeral Domains**:
    - *Anchor Tools*: High dwell time and persistent shelf-life (`github.com`, `docs.rs`, `localhost`, `atlassian.net`).
    - *Ephemeral Lookups*: Low shelf-life, rapid exit (`google.com`, `stackoverflow.com`, search redirects).
  - **Context Decay Threshold ($\tau$)**: Computes optimal idle-time threshold beyond which user return probability drops below 5% (e.g. $\tau = 50\text{ mins}$), tuning the Copilot's forgotten loop triggers.

### 4. Windows 11 Fluent GUI Integration
- **Section 4 in View 5 (Focus Copilot)**:
  - **Graveyard Ratio Card**: Real-time percentage badge.
  - **Median Shelf-Life**: 50th percentile lifespan of user tabs.
  - **Context Decay Threshold ($\tau$)**: Nudge trigger point.
  - **Mortality Curve**: Breakdown of quick vs. active vs. anchor tabs.
  - **Domain Archetypes**: Visual pills for ⚓ Anchor Work Domains and ⚡ Ephemeral Lookup Domains.
- **REST Endpoint**:
  - `GET /api/knowledge/tab-analytics` in [`src/gui/server.rs`](file:///c:/Dev/MemoryBlast/src/gui/server.rs).

---

## 2. Automated Test Verification

All 19 automated unit tests across the entire crate passed cleanly:
```
running 19 tests
test copilot::clipboard::tests::test_classification ... ok
test copilot::clipboard::tests::test_secret_redaction ... ok
test copilot::drift::tests::test_working_set_cap_alert ... ok
test chromium::lifecycle::tests::test_extract_domain ... ok
test copilot::fixation::tests::test_tangent_topic_classification ... ok
test knowledge::tab_analytics::tests::test_tab_analytics_synthesis ... ok
test treemap::tests::test_squarified_layout_aspect_ratios ... ok
test copilot::fixation::tests::test_command_failure_loop ... ok
test knowledge::schema::tests::test_schema_initialization ... ok
test chromium::lifecycle::tests::test_lifecycle_open_and_close ... ok
test knowledge::adaptive_engine::tests::test_adaptive_profile_default ... ok
test copilot::clipboard::tests::test_clipboard_manager_and_frustration ... ok
test knowledge::tab_history::tests::test_tab_history_lifecycle_storage ... ok
test knowledge::consolidator::tests::test_consolidation_and_archetyping ... ok
test knowledge::orientation::tests::test_executive_briefing_generation ... ok
test copilot::drift::tests::test_forgotten_loop_detection ... ok
test copilot::anchor::tests::test_resume_anchor_creation ... ok
test analytics::tests::test_leak_velocity_detection ... ok
test copilot::stashing::tests::test_stash_manager_save_and_list ... ok

test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.83s
```
