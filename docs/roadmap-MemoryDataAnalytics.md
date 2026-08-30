```markdown
You are a senior Rust systems architect, low-level Windows engineer, and data science visualizer.

Upgrade the **MemoryBlast** architecture from a standard system monitor into an **Intelligent Memory Observatory & Cognitive Knowledge Engine**. The goal is to combine sub-millisecond kernel/Win32 sampling and deep Chromium tab resolution with dynamic data visualization, continuous behavioral learning, and automated optimization workflows.

---

### Key Architectural Upgrades


```

┌──────────────────────────────────────────────────────────────────────────────────┐
│                           MEMORYBLAST AI ARCHITECTURE                            │
├──────────────────────────────┬────────────────────────┬──────────────────────────┤
│ 1. Telemetry & Analytics     │ 2. Interactive Treemap │ 3. Behavioral Knowledge  │
│ • Fast Win32 Sampling        │ • Squarified Treemap   │ • Process Clustering     │
│ • Polars DataFrame Engine    │ • Dynamic Breadcrumbs  │ • Tab Lifespan Models    │
│ • Parquet Rolling Buffer     │ • Instant Tab Focus    │ • Proactive Reclamation  │
└──────────────────────────────┴────────────────────────┴──────────────────────────┘

```

---

### 1. Dynamic Data Science Engine (Polars & Streaming Telemetry)
- **High-Throughput Analytics Pipeline**:
  - Ingest Tier-1 memory samples (Working Set, Private Bytes, Pagefile Commit, Non-Paged pool) and Tier-2 Chromium tab metadata into high-performance columnar data frames using **`polars`**.
  - Stream metrics into an ephemeral rolling buffer and persist partitioned time-series logs using **Apache Parquet** (`parquet` / `arrow2`).
  - Calculate online statistical metrics: Rolling $Z$-score memory anomaly detection, process leak velocity ($\frac{d\text{RAM}}{dt}$), and commit charge slope.

### 2. Interactive & Dynamic Squarified Treemap Visualization
- **Layout Algorithm**:
  - Implement a pure Rust **Squarified Treemap algorithm** (Bruls, Huizing, and van Wijk) maintaining ideal aspect ratios close to 1.0 ($1:1$ squares/rectangles).
  - Support hierarchical aggregation: `Host System` ➔ `App Clusters (e.g. Chrome, IDE, Services)` ➔ `Sub-processes / Tabs`.
  - Node area represents `Private Working Set / Commit Bytes`; dynamic color grading highlights leak velocity or CPU impact.
- **Interactive UI Capabilities**:
  - **Click-to-Drilldown & Zoom**: Smooth navigation from global memory down to a specific browser tab.
  - **Instant Focus & Action**: Direct trigger from any treemap rectangle to bring target window/tab to foreground (`SetForegroundWindow` / CDP activate) or reclaim memory.
  - **Rendering Targets**: Provide unified layout calculations for both the Windows Fluent GUI and Ratatui terminal buffers.

### 3. Behavioral Knowledge Management & Cognitive Learner
- **Context & Session Graphing**:
  - Classify tab/process lifecycles into semantic clusters (e.g., *Active Task Focus*, *Stale Background Tab*, *Memory Leak Suspect*, *Periodic Background Daemon*).
  - Track user interaction patterns, tab creation frequency, dwell times, and context switching.
- **Continuous Knowledge Consolidation**:
  - Persist daily user memory footprints and behavioral archetypes in a lightweight embedded database (e.g., **`heed`** LMDB or **`sqlite`**).
  - Train local online linear/Bayesian regressors predicting when the host will cross commit limits (OOM forecasting).
- **Proactive Optimization Assistant**:
  - Provide human-readable, non-intrusive optimization recommendations (e.g., "Tab *[X]* has consumed +600MB idle over 4 hours with 0% user interaction — Hibernate tab?").
  - Support automatic/assisted tab sleeping, working set trimming (`EmptyWorkingSet`), and memory defragmentation heuristics.

---

### Technical Requirements & Rust Crates
- **Core Systems**: `windows` (0.58+), `ntapi`
- **Data Science & Streaming**: `polars` (lazy execution & streaming), `arrow2` / `parquet`
- **Embedded Knowledge Store**: `heed` (Lightning Memory-Mapped DB) or `rusqlite`
- **Algorithms & Math**: Custom squarified treemap module, `linfa` (or native lightweight online clustering/regression)
- **UI & Visualization**: `ratatui` (TUI visual layout) alongside GUI visual hooks

---

### Deliverables Required
1. **Module Architecture Map**: Provide the updated project structure (incorporating `telemetry/`, `analytics/`, `treemap/`, `knowledge/`, and `optimizer/`).
2. **Squarified Treemap Implementation**: Complete, robust Rust code implementing the recursive layout generator with bounding-box clipping and label placement.
3. **Data Science & Polars Pipeline**: Implementation of the telemetry collector, rolling memory gradient ($\frac{d\text{RAM}}{dt}$) calculation, and anomaly detection.
4. **Behavioral Assistant & Knowledge Model**: Implementation of the learning loop that correlates tab lifespans with memory leaks to suggest actionable optimizations.

```