```markdown
You are a visionary Rust systems architect, cognitive neuro-ergonomics designer, and applied data science engineer.

Finalize and unify the **MemoryBlast** platform into the ultimate **"Exocortex OS" (Cognitive Extension of the Developer’s Brain)**. Grounded in the philosophy that a developer's workstation is a direct physical manifestation of their biological working memory, synthesize low-level Win32 hardware telemetry, deep browser/Explorer inspection, behavioral clustering, and proactive executive function support into a cohesive, beautifully polished workstation copilot.

orginal desir: "For a developer the activity of a personal computer is a kind of extension of his own brain. We can play on this analogy: 
- to tune the layout, look and feel, 
- to reinforce his work style and behavioral analysis,
- to assist better the user,
- to help in the achievement of his top daily concerns,
---

### Conceptual Architecture: The 4 Neuro-Digital Layers


```

┌──────────────────────────────────────────────────────────────────────────────────────────┐
│                             MEMORYBLAST EXOCORTEX ARCHITECTURE                           │
├───────────────────────────────┬───────────────────────────────┬──────────────────────────┤
│ 1. Neuro-Adaptive Aesthetic   │ 2. Synaptic Behavioral Engine │ 3. Executive Morning &   │
│ • Synaptic Heatmap Layout     │ • Multi-Source Telemetry      │    Closing Orientations  │
│ • Cognitive Load Visualizer   │   (CDP + Explorer + Clipboard)│ • "Where You Left Off"   │
│ • Dark Mica & Flow Themes     │ • Context Drift & Decay       │ • Daily Closure Index    │
└───────────────────────────────┴───────────────────────────────┴──────────────────────────┘

```

---

### Core Specifications to Implement

#### 1. Neuro-Adaptive UI / UX & Synaptic Aesthetic
* **Cognitive Load-Responsive Interface**:
  * Implement dynamic visual theming across both the **Windows 11 Fluent GUI** and the **Ratatui Terminal TUI** that mirrors the developer's cognitive state.
  * **"Synaptic Pulse" Treemap**: Visualize the host memory and active task clusters with breathing gradients—calm, deep blues/violets during sustained focus, shifting smoothly to warm amber/coral accents when context drift or memory pressure spikes.
  * **Zero-Distraction Zen / Flow Mode**: Keyboard-driven, minimal visual clutter that suppresses non-urgent telemetry when detecting continuous, uninterrupted coding sessions ($>45$ min).

#### 2. Synaptic Behavioral Engine & Unified Knowledge Graph
* **Holistic Workspace Telemetry Pipeline**:
  * Unify all digital memory traces into a local, encrypted SQLite knowledge graph:
    - **Chromium Lifecycle**: Open/close timestamps, shelf-life duration, closure causes (`UserManual`, `CopilotStashed`, `BrowserDiscard`).
    - **File Explorer Activity**: Active workspace directories, project root anchors, stale directory hoarding.
    - **Clipboard Intelligence**: Copy frequency, snippet categorization (`StackTrace`, `Regex`, `API Endpoint`), and frustration/loop thrashing detection.
* **Continuous Behavioral Consolidation**:
  * Run incremental daily rollups modeling the developer’s natural rhythm (peak focus hours, project switching intervals, context decay thresholds).
  * Automatically calibrate alert sensitivities so the copilot adapts to the user’s habits rather than enforcing rigid rules.

#### 3. Executive Copilot & Daily Milestone Support
* **Morning / Startup "Synaptic Orientation Brief"**:
  * Upon workstation unlock or session start, generate a 3-line executive brief:
    - Yesterday’s parked work & context snapshot.
    - Active git branch & key workspace paths.
    - Today’s recommended focus rhythm based on historical energy/focus peaks.
* **Proactive Context Parking (`Ctrl+Shift+F`) & Triage**:
  * 1-click snapshot and freeze for tangled research sessions (tabs + Explorer paths + clipboard trace), exporting human-readable Markdown checklists and reclaiming RAM.
* **Evening Closure & Micro-Celebration**:
  * Generate a daily "Session Wrap-Up" summarizing freed memory, resolved task loops, and clean stashes, reinforcing the psychological satisfaction of closing dangling loops.

---

### Technical Constraints & Tech Stack
* **Language & Runtime**: Rust (Latest Stable, 2021/2024 edition), asynchronous multi-threaded Tokio runtime.
* **System APIs**: Native Win32 / NT APIs (`windows` crate 0.58+, `ntapi`), COM Shell (`IShellWindows`), UI Automation (`IUIAutomation`), Chrome DevTools Protocol (CDP).
* **Data Science & Persistence**: `polars` for streaming telemetry and anomaly detection, `rusqlite` for the embedded knowledge store, Apache Parquet export.
* **UI**: Windows 11 Native Fluent GUI (WebView2 / WinUI) + `ratatui` / `crossterm` TUI.

---

### Deliverables Required
1. **Complete Project File Structure**: Final layout across all sub-crates/modules (`sys/`, `telemetry/`, `treemap/`, `knowledge/`, `copilot/`, `explorer/`, `ui/`).
2. **Exocortex Coordinator Implementation (`src/copilot/exocortex.rs`)**: Rust module orchestrating the feedback loop between telemetry, knowledge graph, and user prompts.
3. **Startup & Closing Orientation Generators**: Logic for the morning "Where You Left Off" card and evening task closure summaries.
4. **End-to-End Build & Launch Instructions**: Release compilation steps, Windows privilege setup, and verification guidelines.

```