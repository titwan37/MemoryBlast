# Walkthrough: Portfolio Booklet Matcher & Golden Vignettes Cleanser

We have resolved all issues regarding non-portfolio CV document ingestion, corrupted vignette parsing, and XeLaTeX compilation failures. The **Portfolio Booklet Matcher** is now fully operational, producing tailored executive portfolio booklets and maintaining a clean, standalone golden repository of project vignettes in `vignettes/`.

---

## 1. Key Accomplishments

### A. Non-Portfolio Filtering & Multi-Page Vignette Extraction
- **Filtered Non-Portfolio Files**: Added strict regex exclusion in `PortfolioSegmenter.is_file_excluded()` to automatically ignore CVs, resumes, cover letters, references, work certificates, and dossiers (e.g. `CV_*.tex`, `2026-0828_EmbeddedSystemCppDeveloper.tex`, `2026_0828_ProductManagerTecan2.tex`).
- **Multi-Page Slicing**: Correctly parses 10-page master portfolio documents along `% === PAGE X: ...` and `\spotlightheader` boundaries into atomic, high-quality project vignettes.
- **Canonical Key Deduplication**: Consolidated multiple duplicate or versioned files into 22 canonical golden project vignettes (The Simulacrum, Rust BildBlitz, Secure AI Service Mesh, IoT Observability, Clinical Spec Mapping, Full-Stack AI Ecosystem, OpenClaw Agent, etc.).

### B. Golden Vignettes Repository (`vignettes/`)
- All 22 cleansed project vignettes are automatically exported as standalone, compilable `.tex` files into:
  - `L:\My Drive\RAV\2025rav\2026-Portfolio-Tex\vignettes\`
  - `l:\My Drive\Work\Code\JobApply\VectorDB_Portfolio\vignettes\`
- Stale legacy test files are automatically removed upon synchronization, keeping the directory clean and organized.

### C. Bulletproof XeLaTeX Multi-Pass Compilation
- **Robust Typography & Fonts**: Switched from missing system font names to `Segoe UI` with native XeLaTeX support, eliminating all `nullfont` warnings and font drop errors.
- **Unified Macro Preambles**: Normalized `\spotlightheader[2]`, `\techbadge[1]`, `\impact[1]`, `\pagefooter[2]`, and `\cvachievements` across all vignettes.
- **FontAwesome5 Compatibility**: Automatically replaced legacy star syntax (e.g., `\faTachometer*`, `\faShield*`, `\faExternalLink*`) with valid modern `fontawesome5` icons.
- **Graceful Blueprint Image Placeholders**: Automatically replaces missing image assets with stylized `tcolorbox` architecture diagram placeholders.

---

## 2. Verification Results

### Test 1: MiniRAG Sync & Golden Vignettes Cleanse
```
[MINIRAG INGESTION] Parsing and cleansing portfolio vignettes across 53 files...
[VIGNETTES EXPORT] Exported 22 golden project vignettes into 'vignettes/' subfolders.
[SUCCESS] Ingested 107 elements -> Cleansed to 22 unique golden portfolio vignettes.
```

### Test 2: Standalone Vignette Compilation
- Compiled `vignette_10_PROJECT_SPOTLIGHT_BILDBLITZ.tex` individually:
- **Result**: `Exit Code: 0`, 0 errors, rendered `vignette_10_test.pdf` (14,189 bytes).

### Test 3: Master Multi-Page Booklet Compilation (Swisslinx AG)
- Assembled bespoke 4-page booklet (Executive Overview + BildBlitz + Simulacrum + IoT Observability):
- **Result**: `Exit Code: 0`, 0 errors.
- Generated `2026-0902_Portfolio-Booklet_SwisslinxAG_EmbeddedRustEngineer_AFalempin_en.pdf` (85,084 bytes).

---

## 3. How to Use

From the terminal:
```powershell
python "l:\My Drive\Work\Code\JobApply\JobApply_Portfolio_BookletMatcher.py"
```
Or launch via the **Master Menu**:
```powershell
python "l:\My Drive\Work\Code\JobApply\JobApply_MasterMenu.py"
# Select Option [16] Portfolio Booklet Matcher
```
