# MemoryBlast Feature List

MemoryBlast is a high-performance, ultra-low-overhead process manager and Intelligent Memory Observatory built in Rust for Windows (`x86_64-pc-windows-msvc`). It acts as a cognitive knowledge engine and an Executive Focus Copilot, primarily targeting power users, developers, and those prone to cognitive overload and tab hoarding.

---

## 1. Core Process & System Monitoring
* **High-Performance Rust Backend:** Ultra-low CPU overhead (<0.5%) asynchronous sampling loop using native Win32/NT APIs (`GlobalMemoryStatusEx`, `NtQueryInformationProcess`, `K32GetProcessMemoryInfo`).
* **Deep Chromium Inspection:** Natively resolves processes for Chromium-based browsers (Chrome, Edge, Brave, Arc, Opera, Vivaldi). 
* **Tab Title Resolution:** Identifies exact tab titles via Windows UI Automation, Chrome DevTools Protocol (CDP), and Window Handle Enumeration without requiring browser restarts.
* **Process Categorization:** Differentiates between Browser Core, GPU Processes, Network Services, Audio Services, Extensions, and individual Renderer Tabs.

## 2. Interactive User Interfaces
MemoryBlast provides dual interfaces to match any workflow:

### Windows 11 Fluent Desktop GUI
* **Native Look & Feel:** Modeled after Windows 11 Task Manager with a dark Mica theme.
* **Real-time Memory Heatbars:** Visual representation of memory usage directly in the data grids.
* **Interactive Navigation:** Breadcrumbs, click-to-focus tab actions, and dynamic node inspectors.
* **Locate & Focus Tab (`🎯 Focus Tab`):** Instantly bring any buried browser tab or process to the foreground.

### Terminal TUI Dashboard
* **Ultra-Fast Ratatui Console:** Keyboard-driven, zero-distraction terminal UI.
* **Multi-View System:** Toggle between Processes Table, Squarified Treemap, and AI Optimizer views using the `[Tab]` key.
* **Rich Keybindings:** Quick filters (`/`), sorting (`s`), trimming RAM (`x`), parking context (`z`), and process killing (`k`).

## 3. High-Throughput Columnar Telemetry & Polars Pipeline
* **In-Memory Columnar DataFrames:** Uses `polars` for fast representation of Tier-1 metrics (Working Set, Private Bytes, CPU %) and Tier-2 metadata (Chromium flags, Tab titles).
* **Rolling Ephemeral Buffers:** Tracks historical system commit charges, OOM pressure, and pool allocations.
* **Parquet Export:** One-click export of time-series telemetry logs to Snappy-compressed Apache Parquet files (`POST /api/export-parquet`).

## 4. Interactive Squarified Treemap Visualization
* **Dynamic Geometry:** Pure Rust squarified algorithm (Bruls, Huizing, and van Wijk 2000) for hierarchical memory mapping.
* **Clustered Hierarchy:** Organizes memory blocks logically from `Host System` ➔ `App Clusters` ➔ `Sub-processes / Tabs`.
* **Cross-UI Support:** Available as a dedicated view in both the Fluent GUI and the Terminal TUI.

## 5. Dynamic Data Science Engine & Statistical Anomaly Detection
* **Real-time Memory Leak Detection:** Calculates process memory leak velocity ($\frac{d\text{RAM}}{dt}$) in MB/min using rolling linear regression slopes.
* **Z-Score Anomaly Detection:** Applies statistical anomaly scoring to identify unusual memory spikes.
* **Categorization:** Classifies behaviors as `Normal`, `ModerateGrowth`, `RapidSpike`, or `SustainedLeak`.

## 6. Behavioral Knowledge Management & Cognitive Learner
* **Semantic Lifecycle Clustering:** Automatically classifies tabs and processes into *Active Task Focus*, *Stale Background Tab*, *Memory Leak Suspect*, *Periodic Background Daemon*, or *Essential System Service*.
* **Embedded SQLite Store:** Persists behavior archetypes, daily footprints, and optimization event logs.
* **OOM Forecasting:** Local online linear regressor computes commit percentages and estimates time-to-limit exhaustion.

## 7. Proactive Optimization Assistant & Working Set Trimming
* **Urgency-Ranked Recommendations:** Generates actionable suggestions (`INFO`, `RECOMMENDED`, `URGENT`) to manage idle tabs and leaking processes.
* **Win32 Memory Reclamation:** Safely purges resident pages to the standby pool using `K32EmptyWorkingSet` with zero process interruption.
* **One-Click Triggers:** Quickly trim RAM via the UI titlebar, recommendation cards, or TUI shortcuts (`[x]`).

## 8. Executive Function & Focus Copilot (ADH Assistant)
A suite of tools designed to prevent task sprawl, context fragmentation, and tab hoarding.

* **Task Sprawl & Cognitive Drift Analytics:**
  * **Forgotten Loop Detection:** Flags tabs untouched for $>90\text{ minutes}$ consuming $>300\text{ MB}$ of RAM, assigning them a Cognitive Cost Score.
  * **Context Drift Index:** Quantifies task fragmentation by measuring rapid window-switching velocity without sustained focus.
  * **Cognitive Working Set Ceiling:** Configurable soft cap (default: 12 tabs) that warns users when nearing mental bandwidth limits.
* **Native Windows Interactive Toasts:**
  * Actionable WinRT toast notifications with `[ 🎯 Focus & Finish ]`, `[ 📦 Stash & Close ]`, and `[ ⏳ Snooze 30m ]` buttons.
  * **Empathetic Micro-Prompts:** Gentle nudges ("You opened this 95 minutes ago...") with exponential backoff throttling to prevent notification fatigue.
* **Session Park & Reclaim Engine:**
  * **Global Hotkey:** Press `Ctrl+Shift+F` globally in Windows to instantly park the current context and freeze open tabs.
  * **SQLite & Markdown Checklists:** Serializes tabs into the database and generates a Markdown checklist before closing them to free RAM.
  * **Micro-Closure Celebrations:** Toast notifications celebrating the amount of RAM reclaimed.
  * **1-Click Rehydration:** Restore parked tabs seamlessly from the UI, API (`POST /api/copilot/restore`), or CLI (`memoryblast.exe restore <id>`).
