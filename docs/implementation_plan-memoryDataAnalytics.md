# Upgrade MemoryBlast Architecture

## Goal Description

Upgrade the **MemoryBlast** project from a standard system monitor into an **Intelligent Memory Observatory & Cognitive Knowledge Engine**. This includes implementing high‑throughput telemetry with Polars, a dynamic squarified treemap UI, behavioral knowledge management, and an assistant that suggests proactive memory optimizations.

## User Review Required

> [!IMPORTANT]
> The plan proposes major restructuring: new crate modules (`telemetry/`, `analytics/`, `treemap/`, `knowledge/`, `optimizer/`), new dependencies (`polars`, `arrow2`, `parquet`, `heed` or `rusqlite`, `linfa`, `ratatui`, `windows`, `ntapi`), and breaking changes to the public API. Please confirm you are comfortable with these changes and the added dependencies.

> [!WARNING]
> The implementation will require Windows‑specific low‑level code and may need administrative privileges for certain telemetry calls. Ensure the build environment has the Windows SDK installed.

## Open Questions

- **Database choice**: Do you prefer `heed` (LMDB) for the embedded knowledge store or `rusqlite`?
- **UI target**: Should the treemap be implemented for both the Fluent GUI and the TUI (`ratatui`), or focus on one initially?
- **Testing scope**: Do you want unit tests for each new module now, or add integration tests later?
- **Release strategy**: Incremental releases per module or a single monolithic release?

## Proposed Changes

---
### Crate Structure

- **[NEW] `src/telemetry/mod.rs`** – Core telemetry collection, tier‑1 sampling loop, and Polars streaming pipeline.
- **[NEW] `src/analytics/mod.rs`** – Analytics functions: rolling Z‑score, leak velocity, anomaly detection.
- **[NEW] `src/treemap/mod.rs`** – Squarified treemap algorithm implementation.
- **[NEW] `src/knowledge/mod.rs`** – Embedded knowledge store abstraction (Heed/SQLite) and online learning models.
- **[NEW] `src/optimizer/mod.rs`** – Proactive assistant providing recommendations and acting on them.
- **[MODIFY] `Cargo.toml`** – Add dependencies: `polars`, `arrow2`, `parquet`, `heed` (or `rusqlite`), `linfa`, `ratatui`, `windows`, `ntapi`.
- **[MODIFY] `src/main.rs`** – Initialize new modules, configure async runtime, and launch GUI/TUI based on args.
- **[MODIFY] `src/gui.rs`** – Hook treemap layout into the Fluent UI, add recommendation overlay.
- **[MODIFY] `src/tui.rs`** – Render treemap in terminal, integrate with existing TUI navigation.

---
### Telemetry & Polars Pipeline

- Implement tier‑1 high‑frequency sampling (500 ms) pulling memory stats via Win32 APIs.
- Stream samples into a Polars `LazyFrame`, write rolling buffers to Parquet files.
- Compute derivative `dRAM/dt` and Z‑score anomaly detection in real time.

---
### Squarified Treemap

- Port the Bruls‑Huizing‑van Wijk algorithm to Rust.
- Provide API for hierarchical data (system → app clusters → tabs).
- Ensure layout calculations are deterministic and support label placement.

---
### Behavioral Knowledge Management

- Persist daily memory footprints and session graphs.
- Train lightweight online regression models (Bayesian linear) to forecast OOM.
- Store clusters and patterns in the embedded DB.

---
### Proactive Optimization Assistant

- Generate human‑readable recommendations based on knowledge models.
- Offer actions: bring tab to foreground, hibernate tab, trim working set, defragment memory.
- Provide UI hooks for both GUI and TUI to display suggestions.

## Verification Plan

### Automated Tests
- Unit tests for telemetry sampling mock (no real Win32 calls).
- Property‑based tests for treemap layout (area preservation, aspect ratio bounds).
- Integration test: end‑to‑end pipeline using synthetic memory data.
- CI pipeline to run on Windows CI runner.

### Manual Verification
- Run the upgraded binary, verify real‑time graphs update, treemap displays correctly.
- Simulate memory leak scenario, ensure assistant suggests tab hibernation.
- Validate that knowledge DB persists across restarts and predictions improve over time.

---
*Please review the above plan, answer the open questions, and approve to proceed with implementation.*
