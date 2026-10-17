# Implementation Plan: Redesign MemoryBlast into an iOS-Inspired System Health & Memory Hub

Transform MemoryBlast from a developer-centric Windows Task Manager clone into an intuitive, visually stunning, Apple/iOS-inspired System Health & Memory Hub that non-technical users can understand effortlessly while preserving all underlying diagnostic depth for power users.

---

## User Review Required

> [!IMPORTANT]
> - **Default View Paradigm**: The primary "Processes" tab is transformed into an **App & Health Hub**. It features **3 Circular Arc Gauges** (System Pressure, Active Workload, Background Apps), a **Dynamic Health Status Pill**, and an **Interactive Card/Pill Hierarchy** with friendly app icons, recognizable brand names, and visual memory meters.
> - **Slide-Over Technical Inspector**: Non-technical users won't be overwhelmed by raw PIDs and obscure thread counts. Instead, clicking any app or tapping the **Inspect** button smoothly opens a frosted-glass slide-over drawer displaying technical metrics (PID, exact bytes, working set, child processes, debug ports).
> - **Magic Clean RAM Action**: A glowing gradient pill button in the top action bar performs instant memory compaction across idle/heavy background working sets with haptic/visual spring feedback.
> - **Retained Full Capabilities**: All existing features (Performance graphs, Squarified Treemap, Cognitive AI, Focus Copilot, Parquet export, SSE live stream) remain fully operational and upgraded to match the frosted glass aesthetic.

---

## Proposed Changes

### 1. Design System & CSS Overhaul (`src/gui/assets/style.css`)
- **Apple/visionOS Dark Translucency**:
  - Palette: `#0B0D13` (canvas), `#141721` (elevated surface), `#1C2130` (card surface), `#283046` (hover/active).
  - Multi-layered acrylic glass with `backdrop-filter: blur(24px) saturate(180%)`, soft 1px inner border highlights (`rgba(255, 255, 255, 0.08)` to `rgba(255, 255, 255, 0.14)`), and subtle drop shadows (`0 8px 32px rgba(0, 0, 0, 0.37)`).
  - Apple SF / Inter typography hierarchy with crisp weights (400, 500, 600, 700) and `font-variant-numeric: tabular-nums`.
- **Circular Gauge & Activity Ring Components**:
  - SVG radial stroke dasharray/dashoffset animation gauges with vibrant Apple-style gradients (Emerald ➔ Amber ➔ Coral/Rose for pressure; Cyan ➔ Blue for memory; Purple ➔ Indigo for active apps).
- **Humanized App Cards & Progress Bars**:
  - Responsive grid layout (`grid-template-columns: repeat(auto-fill, minmax(360px, 1fr))`) with toggle for compact list view.
  - Distinct badges for browser tab clusters (`55 Active Tabs • 1.1 GB`) that expand cleanly with accordion animation.
  - Memory consumption pills with subtle gradient progress fills and threshold-based colors.
- **Slide-Over Inspector Drawer**:
  - Off-canvas frosted glass drawer (`transform: translateX(100%)` ➔ `translateX(0)`) for detailed process diagnostics.
- **Primary Action Center**:
  - "Magic Clean" glowing pill button with ambient pulse and animated sparkle.
  - Frosted capsule search bar with clear button and keyboard shortcut indicators (`Ctrl+K` / `Ctrl+Shift+F`).

#### [MODIFY] [style.css](file:///c:/Dev/MemoryBlast/src/gui/assets/style.css)

---

### 2. Markup & Component Structure (`src/gui/assets/index.html`)
- **Header & Titlebar**:
  - Modern iOS/macOS floating navigation bar with app glyph, frosted search capsule, and streamlined primary actions:
    - `[ ✨ Magic Clean ]`
    - `[ 📦 Park Tabs ]`
    - `[ 🔍 Retrigger Detection ]`
    - Admin status indicator styled as a subtle security capsule.
- **At-a-Glance Health Dashboard (Hero Section)**:
  - Dynamic System Status Banner:
    - Left: Human-readable status chip (e.g. `● System Running Smoothly — 42% Load` with breathing ambient glow).
    - Quick actions: One-tap optimize button.
  - 3 Circular Ring Gauge Cards:
    1. **System Pressure**: Arc meter with real-time %, color-coded status (`Normal`, `Elevated`, `Heavy Load`), and commit load.
    2. **Active Workload**: Memory used vs total capacity (`27.1 GB of 31.4 GB used`), available headroom.
    3. **Background Apps**: Count of active foreground applications, grouped background services, and open web browser tabs.
- **App & Memory Hub Controls**:
  - Quick category filters: `All Apps`, `Browsers`, `Memory Hogs (>500MB)`, `Background Services`.
  - View switcher: `[ ⊞ Cards ]` / `[ ☰ List ]`.
- **Humanized Process Cards Container**:
  - Container for dynamic rendering of application cards and grouped browser clusters.
- **Slide-Over Technical Inspector Drawer**:
  - Drawer markup with tab header, performance telemetry, raw PID/parent info, child tab list, and action buttons (`Switch To`, `Put to Sleep`, `Close App`).
- **Confirmation & Modal dialogs**:
  - Redesigned with iOS-styled rounded sheet styling, frosted backdrop, and friendly button copy.

#### [MODIFY] [index.html](file:///c:/Dev/MemoryBlast/src/gui/assets/index.html)

---

### 3. View Controller & ViewModel Logic (`src/gui/assets/app.js`)
- **Friendly Application Catalog & Heuristics**:
  - Extensive dictionary mapping Windows executables (`chrome.exe`, `msedge.exe`, `code.exe`, `spotify.exe`, `discord.exe`, `slack.exe`, `explorer.exe`, `windowsterminal.exe`, `powershell.exe`, etc.) to:
    - Curated Human Name (e.g. "Visual Studio Code", "Windows Explorer", "Spotify Music").
    - Category ("Web Browser", "Developer Tool", "Audio & Media", "System Utility", "Background Service").
    - Curated SVG App Icon (modern, vector-crisp icons for all major apps, plus clean generic fallback icons per category).
- **Humanized Metric Translators**:
  - Memory pressure classifier: `Normal (<60%)`, `Moderate (60-80%)`, `Elevated (80-90%)`, `Critical (>90%)`.
  - Status text generator: e.g. `"All systems operating efficiently. 18.2 GB headroom available."`
  - Friendly byte formatting (`1.2 GB`, `450 MB`).
- **Interactive Card & Group Rendering Engine**:
  - Renders apps as interactive cards with:
    - App icon, friendly name, category pill.
    - Relative memory share meter.
    - Browser tab count badge with instant accordion expansion for tabs.
    - Quick action buttons on card: `Switch To`, `Put to Sleep` (Trim), `Close`.
    - Clicking the card or `Inspect` button opens the technical inspector drawer with live-updated stats.
- **"Magic Clean" Handler**:
  - Triggers working set trimming on background memory hogs or top memory-consuming processes via `/api/trim`.
  - Calculates reclaimed memory and displays a celebratory toast with particle/sparkle feedback.
- **Maintains SSE Stream & Multi-View Coordination**:
  - Preserves real-time 500ms updates via `/api/stream`.
  - Keeps Treemap, Performance, Cognitive AI, and Focus Copilot views seamlessly synchronized.

#### [MODIFY] [app.js](file:///c:/Dev/MemoryBlast/src/gui/assets/app.js)

---

## Verification Plan

### Automated Verification
- Verify Rust build & test suite compile with zero errors:
  ```powershell
  cargo check
  cargo test --lib
  ```

### Manual & Interactive Verification
- Launch the application or test the HTTP GUI server with the updated assets:
  ```powershell
  cargo run -- --gui
  ```
  *(Or inspect the rendered DOM and test locally using browser subagent / local mock if needed)*
- Verify:
  1. Circular arc gauges animate smoothly with real-time memory stats.
  2. Friendly names and SVG icons display correctly for common apps.
  3. Raw PIDs and executables are neatly sequestered in the Inspector Drawer.
  4. "Magic Clean" button executes and provides positive feedback.
  5. Browser tab expansion groups work smoothly.
  6. Switching between Views (Processes, Treemap, Performance, Cognitive AI, Focus Copilot) retains state and functions.
  7. Search filtering and category pills update cards in real time.
