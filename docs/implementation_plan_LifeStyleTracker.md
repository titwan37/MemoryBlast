# Implementation Plan: Chromium Tab Lifecycle Tracker & Behavioral Shelf-Life Engine

Extend MemoryBlast with real-time **Chromium Tab Lifecycle Tracking**, recording exact opening and closing timestamps, computing shelf-life duration ($\Delta T$), classifying closure and suspension causes, and synthesizing long-term tab utility patterns (Mortality Curve, Graveyard Ratio, Ephemeral vs. Anchor domains, and Context Decay Threshold $\tau$).

---

## User Review Required

> [!IMPORTANT]
> **Zero Browser Modification**: The lifecycle tracker operates non-destructively through non-invasive polling of CDP `/json/list` endpoints and Win32 process lifecycle monitoring. It requires no browser flags or invasive extensions.
>
> **Privacy Protection**: URLs are domain-stripped into high-level origin metrics for analytics, while specific URLs in `tab_lifecycles` are stored strictly in local SQLite (`memoryblast_data/knowledge.db`) with zero external telemetry.

---

## Architecture & Proposed Changes

```
src/
├── chromium/
│   ├── mod.rs                  # [MODIFY] Export lifecycle module
│   └── lifecycle.rs            # [NEW] Real-time Tab Lifecycle Tracker, CDP diff engine, closure causes
└── knowledge/
    ├── mod.rs                  # [MODIFY] Export tab_history and tab_analytics
    ├── tab_history.rs          # [NEW] SQLite persistence for tab lifecycles and daily aggregation
    └── tab_analytics.rs        # [NEW] Behavioral synthesizer (Mortality curve, Graveyard ratio, Anchor vs Ephemeral)
```

---

### 1. Chromium Lifecycle Engine
#### [NEW] [`src/chromium/lifecycle.rs`](file:///c:/Dev/MemoryBlast/src/chromium/lifecycle.rs)
- **Lifecycle Event Tracker**:
  - Compares active CDP tab states and Win32 renderer processes between sampling ticks.
  - Generates:
    - `TargetCreated` ➔ records $T_{\text{open}}$, URL, domain, title.
    - `TargetDestroyed` ➔ records $T_{\text{close}}$, calculates $\text{ShelfLife} = T_{\text{close}} - T_{\text{open}}$.
- **Closure & Suspension Cause Classification**:
  - `UserManual`: Tab closed while active or recently focused.
  - `CopilotStashed`: Tab parked/closed via MemoryBlast triage (`Ctrl+Shift+F` or `[ 📦 Stash ]`).
  - `BrowserMemorySaver`: Tab suspended or discarded to reclaim RAM by browser.
  - `RendererCrash`: Sudden process termination with non-zero exit code.
  - `SessionShutdown`: Closed upon browser or system exit.
  - `StaleCleanup`: Closed after $>120$ minutes of continuous idle time.
- **Dwell & Peak Memory Tracking**:
  - Tracks cumulative active foreground dwell seconds and peak working set (MB) during tab lifespan.

---

### 2. SQLite Tab History & Persistence
#### [NEW] [`src/knowledge/tab_history.rs`](file:///c:/Dev/MemoryBlast/src/knowledge/tab_history.rs)
- **Schema `tab_lifecycles` in `knowledge.db`**:
  - Stores `target_id`, `domain`, `url`, `title`, `opened_at_epoch`, `closed_at_epoch`, `shelf_life_secs`, `active_dwell_secs`, `closure_cause`, `peak_memory_mb`.
- **Query & Aggregation API**:
  - Insert / update tab lifecycle record.
  - Aggregate metrics: average shelf-life by domain, closure cause breakdown, mortality statistics.

---

### 3. Behavioral Insight Synthesizer
#### [NEW] [`src/knowledge/tab_analytics.rs`](file:///c:/Dev/MemoryBlast/src/knowledge/tab_analytics.rs)
- **Tab Mortality & Utility Curve**:
  - Computes distribution of tab lifespans: Quick lookups ($<2\text{ min}$), medium research ($2\text{--}30\text{ min}$), and persistent tabs ($>30\text{ min}$).
- **Graveyard Ratio**:
  - Ratio of abandoned/stale tabs vs. intentionally completed and closed tasks.
- **Domain Archetyping (Ephemeral vs. Anchor)**:
  - Classifies domains:
    - *Anchor Tools*: High total dwell time and regular daily returns (`github.com`, `docs.rs`, `localhost`, `jira`).
    - *Ephemeral Lookup*: Low shelf-life, rapid exit (`google.com`, `stackoverflow.com`, search redirects).
- **Context Decay Threshold ($\tau$)**:
  - Computes the optimal idle duration (e.g. $\tau = 52\text{ min}$) beyond which return probability drops below 5%, feeding into the Copilot's adaptive triage triggers.

---

### 4. Integration & UI Delivery
#### [MODIFY] [`src/main.rs`](file:///c:/Dev/MemoryBlast/src/main.rs)
- Ingest lifecycle transitions in background telemetry sampling loop.
- Stream tab lifecycle insights over SSE in the enriched payload.

#### [MODIFY] [`src/gui/server.rs`](file:///c:/Dev/MemoryBlast/src/gui/server.rs)
- Expose REST endpoint:
  - `GET /api/knowledge/tab-analytics`: Returns mortality curve, graveyard ratio, anchor domains, and decay threshold.

#### [MODIFY] [`src/gui/assets/index.html`](file:///c:/Dev/MemoryBlast/src/gui/assets/index.html) & [`app.js`](file:///c:/Dev/MemoryBlast/src/gui/assets/app.js)
- Add a "Tab Lifecycles & Mortality Analytics" card to View 5 (Focus Copilot).
- Display Graveyard Ratio gauge, Anchor vs. Ephemeral domain breakdown, and Context Decay threshold ($\tau$).

---

## Verification Plan

### Automated Tests
- `cargo test --bin memoryblast chromium::lifecycle`
  - Tests lifecycle state machine transitions (open ➔ update ➔ close).
  - Tests closure cause classification.
- `cargo test --bin memoryblast knowledge::tab_analytics`
  - Tests mortality curve calculation.
  - Tests Graveyard Ratio and Context Decay threshold derivation.
- `cargo check` to verify full compilation.

### Manual Verification
- Launch MemoryBlast, open a few tabs in Chrome/Edge, close one manually and stash another:
  - Verify `tab_lifecycles` records closure causes correctly (`UserManual` vs. `CopilotStashed`).
  - View tab analytics card in GUI: verify Graveyard Ratio and domain breakdowns render smoothly.
