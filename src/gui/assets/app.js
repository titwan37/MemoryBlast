let currentSnapshot = null;
let expandedPids = new Set();
let selectedPid = null;
let selectedPpid = null;
let selectedName = null;
let selectedTitle = null;
let selectedPort = null;
let inspectedProcess = null;
let currentSort = { col: 'memory', desc: true };
let searchQuery = '';
let activeCategory = 'all'; // 'all', 'browsers', 'heavy', 'background'
let viewMode = 'cards';     // 'cards' or 'table'
let activeView = 'processes';

// 60-second history buffer (each sample ~500ms -> up to 120 samples)
const memoryHistory = [];
const MAX_HISTORY = 120;

// SVG Icons Catalog (Apple / SF Symbols inspired vector icons)
const ICONS = {
  chrome: `<svg viewBox="0 0 24 24" fill="none"><circle cx="12" cy="12" r="10" stroke="#38BDF8" stroke-width="1.8"/><circle cx="12" cy="12" r="4" fill="#38BDF8"/><path d="M12 2a10 10 0 0 1 8.66 5H12" stroke="#34D399" stroke-width="1.8"/><path d="M20.66 7 12 22" stroke="#FBBF24" stroke-width="1.8"/><path d="M12 22a10 10 0 0 1-8.66-15H12" stroke="#F87171" stroke-width="1.8"/></svg>`,
  edge: `<svg viewBox="0 0 24 24" fill="none"><circle cx="12" cy="12" r="10" stroke="#0284C7" stroke-width="1.8"/><path d="M12 4a8 8 0 0 1 8 8c0 4.4-3.6 8-8 8-3.5 0-6.5-2.2-7.6-5.4 1.2.9 2.8 1.4 4.6 1.4 4 0 7-3 7-7 0-1.8-.6-3.4-1.7-4.6A7.9 7.9 0 0 1 12 4Z" fill="#38BDF8"/></svg>`,
  brave: `<svg viewBox="0 0 24 24" fill="none"><path d="m12 2 7 4v6c0 5-3.5 9-7 10-3.5-1-7-5-7-10V6l7-4Z" stroke="#F97316" stroke-width="1.8" fill="rgba(249,115,22,0.15)"/><circle cx="12" cy="12" r="3" fill="#F97316"/></svg>`,
  firefox: `<svg viewBox="0 0 24 24" fill="none"><circle cx="12" cy="12" r="10" stroke="#EA580C" stroke-width="1.8"/><circle cx="12" cy="12" r="6" fill="#F97316"/><path d="M12 6a6 6 0 0 1 6 6" stroke="#FEF08A" stroke-width="2"/></svg>`,
  browserGeneric: `<svg viewBox="0 0 24 24" fill="none" stroke="#38BDF8" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="2" y1="12" x2="22" y2="12"></line><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1-4-10z"></path></svg>`,
  vscode: `<svg viewBox="0 0 24 24" fill="none"><path d="m17 2 4 2v16l-4 2-9-7.5L4 18l-2-1V7l2-1 4 3.5L17 2Z" stroke="#38BDF8" stroke-width="1.8" fill="rgba(56,189,248,0.15)"/><path d="m17 7-9 5 9 5" stroke="#38BDF8" stroke-width="1.8"/></svg>`,
  terminal: `<svg viewBox="0 0 24 24" fill="none" stroke="#A855F7" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><polyline points="4 17 10 11 4 5"></polyline><line x1="12" y1="19" x2="20" y2="19"></line><rect x="2" y="2" width="20" height="20" rx="4" stroke-width="1.5" stroke="rgba(255,255,255,0.15)"></rect></svg>`,
  spotify: `<svg viewBox="0 0 24 24" fill="none"><circle cx="12" cy="12" r="10" fill="#10B981"/><path d="M6.5 9.5c3.5-1 7.5-.8 11 1.2" stroke="#000" stroke-width="2" stroke-linecap="round"/><path d="M7.5 12.5c3-.8 6.5-.6 9.5 1" stroke="#000" stroke-width="1.8" stroke-linecap="round"/><path d="M8.5 15.5c2.5-.6 5-.5 7.5.8" stroke="#000" stroke-width="1.5" stroke-linecap="round"/></svg>`,
  discord: `<svg viewBox="0 0 24 24" fill="none"><rect width="20" height="20" x="2" y="2" rx="5" fill="#6366F1"/><circle cx="9" cy="12" r="1.5" fill="#FFF"/><circle cx="15" cy="12" r="1.5" fill="#FFF"/><path d="M7 16c2-1 3-1 5-1s3 0 5 1" stroke="#FFF" stroke-width="1.5" stroke-linecap="round"/></svg>`,
  slack: `<svg viewBox="0 0 24 24" fill="none"><circle cx="7" cy="7" r="3" fill="#E11D48"/><circle cx="17" cy="7" r="3" fill="#0EA5E9"/><circle cx="7" cy="17" r="3" fill="#10B981"/><circle cx="17" cy="17" r="3" fill="#F59E0B"/></svg>`,
  docker: `<svg viewBox="0 0 24 24" fill="none"><rect width="20" height="20" x="2" y="2" rx="4" fill="rgba(14,165,233,0.15)" stroke="#0EA5E9" stroke-width="1.5"/><rect x="5" y="10" width="3" height="3" fill="#0EA5E9"/><rect x="9" y="10" width="3" height="3" fill="#0EA5E9"/><rect x="13" y="10" width="3" height="3" fill="#0EA5E9"/><rect x="9" y="6" width="3" height="3" fill="#0EA5E9"/><path d="M4 15c2 2 12 2 16 0" stroke="#0EA5E9" stroke-width="1.5"/></svg>`,
  explorer: `<svg viewBox="0 0 24 24" fill="none" stroke="#FBBF24" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path></svg>`,
  python: `<svg viewBox="0 0 24 24" fill="none"><path d="M12 2c-4 0-4 2-4 3v3h4V7h2v3h2c2 0 4-.5 4-4s-2-4-8-4Z" fill="#38BDF8"/><path d="M12 22c4 0 4-2 4-3v-3h-4v1h-2v-3H8c-2 0-4 .5-4 4s2 4 8 4Z" fill="#FBBF24"/><circle cx="9.5" cy="5" r=".7" fill="#000"/><circle cx="14.5" cy="19" r=".7" fill="#000"/></svg>`,
  steam: `<svg viewBox="0 0 24 24" fill="none"><circle cx="12" cy="12" r="10" fill="#1E293B" stroke="#64748B" stroke-width="1.5"/><circle cx="8" cy="15" r="2.5" fill="#38BDF8"/><circle cx="15" cy="9" r="3.5" stroke="#38BDF8" stroke-width="1.5"/><line x1="10" y1="13.5" x2="13" y2="10.5" stroke="#38BDF8" stroke-width="2"/></svg>`,
  systemService: `<svg viewBox="0 0 24 24" fill="none" stroke="#64748B" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"></circle><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path></svg>`,
  genericApp: `<svg viewBox="0 0 24 24" fill="none" stroke="#94A3B8" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="4"></rect><circle cx="8.5" cy="8.5" r="1.5"></circle><polyline points="21 15 16 10 5 21"></polyline></svg>`
};

// Verified Application Catalog
const APP_CATALOG = {
  'chrome.exe': { name: 'Google Chrome', category: 'Web Browser', icon: ICONS.chrome },
  'msedge.exe': { name: 'Microsoft Edge', category: 'Web Browser', icon: ICONS.edge },
  'brave.exe': { name: 'Brave Browser', category: 'Web Browser', icon: ICONS.brave },
  'firefox.exe': { name: 'Mozilla Firefox', category: 'Web Browser', icon: ICONS.firefox },
  'code.exe': { name: 'Visual Studio Code', category: 'Code Editor', icon: ICONS.vscode },
  'cursor.exe': { name: 'Cursor AI IDE', category: 'Code Editor', icon: ICONS.vscode },
  'antigravity.exe': { name: 'Antigravity IDE', category: 'Code Editor', icon: ICONS.vscode },
  'gemini.exe': { name: 'Antigravity Studio', category: 'Code Editor', icon: ICONS.vscode },
  'windowsterminal.exe': { name: 'Windows Terminal', category: 'Developer Tool', icon: ICONS.terminal },
  'wt.exe': { name: 'Windows Terminal', category: 'Developer Tool', icon: ICONS.terminal },
  'powershell.exe': { name: 'PowerShell', category: 'Developer Tool', icon: ICONS.terminal },
  'pwsh.exe': { name: 'PowerShell Core', category: 'Developer Tool', icon: ICONS.terminal },
  'cmd.exe': { name: 'Command Prompt', category: 'Developer Tool', icon: ICONS.terminal },
  'spotify.exe': { name: 'Spotify Music', category: 'Audio & Media', icon: ICONS.spotify },
  'discord.exe': { name: 'Discord', category: 'Communication', icon: ICONS.discord },
  'slack.exe': { name: 'Slack', category: 'Communication', icon: ICONS.slack },
  'telegram.exe': { name: 'Telegram Desktop', category: 'Communication', icon: ICONS.discord },
  'teams.exe': { name: 'Microsoft Teams', category: 'Communication', icon: ICONS.slack },
  'ms-teams.exe': { name: 'Microsoft Teams', category: 'Communication', icon: ICONS.slack },
  'steam.exe': { name: 'Steam Gaming', category: 'Games & Store', icon: ICONS.steam },
  'explorer.exe': { name: 'Windows File Explorer', category: 'System Shell', icon: ICONS.explorer },
  'docker desktop.exe': { name: 'Docker Desktop', category: 'Virtualization', icon: ICONS.docker },
  'com.docker.backend.exe': { name: 'Docker Service Engine', category: 'Virtualization', icon: ICONS.docker },
  'python.exe': { name: 'Python Runtime', category: 'Programming Runtime', icon: ICONS.python },
  'pyrefly.exe': { name: 'Python Environment Worker', category: 'Programming Runtime', icon: ICONS.python },
  'node.exe': { name: 'Node.js JavaScript Service', category: 'Programming Runtime', icon: ICONS.terminal },
  'cargo.exe': { name: 'Rust Cargo Build Tool', category: 'Developer Tool', icon: ICONS.terminal },
  'rustc.exe': { name: 'Rust Compiler', category: 'Developer Tool', icon: ICONS.terminal },
  'esrv.exe': { name: 'Intel Energy Service', category: 'Background Service', icon: ICONS.systemService },
  'igfxem.exe': { name: 'Intel Graphics Framework', category: 'Hardware Driver', icon: ICONS.systemService },
  'nvcontainer.exe': { name: 'NVIDIA Display Container', category: 'Hardware Driver', icon: ICONS.systemService },
  'svchost.exe': { name: 'Windows System Host Services', category: 'Background Service', icon: ICONS.systemService },
  'taskhostw.exe': { name: 'Windows Task Host', category: 'Background Service', icon: ICONS.systemService },
  'searchhost.exe': { name: 'Windows Search Indexer', category: 'Background Service', icon: ICONS.systemService },
  'dwm.exe': { name: 'Desktop Window Manager', category: 'System Shell', icon: ICONS.systemService },
  'spoolsv.exe': { name: 'Windows Print Spooler', category: 'Background Service', icon: ICONS.systemService }
};

// Friendly App Identity Resolver
function resolveAppIdentity(item) {
  const lowerName = (item.name || '').toLowerCase();
  if (APP_CATALOG[lowerName]) {
    return { ...APP_CATALOG[lowerName] };
  }

  // Check browser family
  if (item.browser_family) {
    const fam = item.browser_family.toLowerCase();
    if (fam.includes('chrome')) return { name: 'Google Chrome', category: 'Web Browser', icon: ICONS.chrome };
    if (fam.includes('edge')) return { name: 'Microsoft Edge', category: 'Web Browser', icon: ICONS.edge };
    if (fam.includes('brave')) return { name: 'Brave Browser', category: 'Web Browser', icon: ICONS.brave };
    return { name: `${item.browser_family} Browser`, category: 'Web Browser', icon: ICONS.browserGeneric };
  }

  if (item.is_chromium) {
    return { name: 'Chromium Browser', category: 'Web Browser', icon: ICONS.browserGeneric };
  }

  // Heuristic cleanup for unknown executables
  let cleanName = item.name || 'Unknown Application';
  cleanName = cleanName.replace(/\.exe$/i, '');
  // Replace underscores and dashes with spaces, capitalize words
  cleanName = cleanName.replace(/[-_]/g, ' ').replace(/\b\w/g, c => c.toUpperCase());

  let cat = 'Application';
  let icon = ICONS.genericApp;

  if (lowerName.includes('host') || lowerName.includes('svc') || lowerName.includes('service') || lowerName.includes('daemon')) {
    cat = 'Background Service';
    icon = ICONS.systemService;
  } else if (lowerName.includes('browser') || lowerName.includes('surf')) {
    cat = 'Web Browser';
    icon = ICONS.browserGeneric;
  } else if (lowerName.includes('code') || lowerName.includes('edit') || lowerName.includes('dev')) {
    cat = 'Developer Tool';
    icon = ICONS.vscode;
  }

  return { name: cleanName, category: cat, icon };
}

// Initialize on DOM load
document.addEventListener('DOMContentLoaded', () => {
  setupThemeToggle();
  setupNavigation();
  setupSearch();
  setupCategoryFilters();
  setupViewModeToggles();
  setupSorting();
  setupColumnResizing();
  setupActions();
  setupInspector();
  setupModal();
  initCanvas();

  // 1. Immediately fetch initial snapshot so UI renders instantly
  fetch('/api/snapshot')
    .then(res => res.json())
    .then(snapshot => {
      if (snapshot && snapshot.processes) {
        onSnapshotReceived(snapshot);
      }
    })
    .catch(() => {});

  // 2. Connect to live real-time SSE stream (500ms)
  connectEventSource();
});

// Dual-Style Theme Switcher (iOS Frosted vs Windows Fluent)
function setupThemeToggle() {
  const toggleBtn = document.getElementById('btn-theme-toggle');
  const styleLink = document.getElementById('app-theme-style');
  const iconSpan = document.getElementById('theme-toggle-icon');
  const labelSpan = document.getElementById('theme-toggle-label');

  const savedTheme = localStorage.getItem('memoryblast_theme') || 'ios';
  applyTheme(savedTheme, false);

  if (toggleBtn) {
    toggleBtn.addEventListener('click', () => {
      const current = localStorage.getItem('memoryblast_theme') || 'ios';
      const next = current === 'ios' ? 'fluent' : 'ios';
      applyTheme(next, true);
      localStorage.setItem('memoryblast_theme', next);
    });
  }

  function applyTheme(theme, notify) {
    if (theme === 'fluent') {
      if (styleLink) styleLink.href = '/style.css';
      if (iconSpan) iconSpan.textContent = '🪟';
      if (labelSpan) labelSpan.textContent = 'Fluent';
      document.body.classList.remove('theme-ios');
      document.body.classList.add('theme-fluent');
      if (notify) showToast('🪟 Switched to Windows 11 Fluent Dark theme');
    } else {
      if (styleLink) styleLink.href = '/ios_style.css';
      if (iconSpan) iconSpan.textContent = '🍎';
      if (labelSpan) labelSpan.textContent = 'iOS Style';
      document.body.classList.remove('theme-fluent');
      document.body.classList.add('theme-ios');
      if (notify) showToast('🍎 Switched to Apple iOS Acrylic Frosted theme');
    }
  }
}

// Navigation Rail
function setupNavigation() {
  const navBtns = document.querySelectorAll('.nav-btn[data-view]');
  navBtns.forEach(btn => {
    btn.addEventListener('click', () => {
      navBtns.forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
      activeView = btn.dataset.view;

      document.querySelectorAll('.view-panel').forEach(p => p.classList.remove('active'));
      const targetPanel = document.getElementById(`view-${activeView}`);
      if (targetPanel) targetPanel.classList.add('active');

      if (activeView === 'performance') {
        requestAnimationFrame(drawCharts);
      } else if (activeView === 'treemap') {
        renderTreemap();
      } else if (activeView === 'analytics') {
        renderAnalytics();
      } else if (activeView === 'copilot') {
        renderCopilot();
        loadExecutiveBriefing();
        loadEveningClosure();
        loadStashedSessions();
        loadResumeAnchor();
        loadClipboardVault();
        loadTabAnalytics();
        loadExplorerWorkspaces();
      }
    });
  });

  const toggle = document.getElementById('nav-toggle');
  if (toggle) {
    toggle.addEventListener('click', () => {
      document.querySelector('.sidebar').classList.toggle('collapsed');
    });
  }
}

// Search
function setupSearch() {
  const input = document.getElementById('global-search');
  const clearBtn = document.getElementById('search-clear');

  input.addEventListener('input', (e) => {
    searchQuery = e.target.value.trim().toLowerCase();
    if (searchQuery) {
      clearBtn.classList.remove('hidden');
    } else {
      clearBtn.classList.add('hidden');
    }
    renderActiveHubView();
  });

  clearBtn.addEventListener('click', () => {
    input.value = '';
    searchQuery = '';
    clearBtn.classList.add('hidden');
    renderActiveHubView();
  });

  // Global Ctrl+K shortcut to focus search
  window.addEventListener('keydown', (e) => {
    if ((e.ctrlKey || e.metaKey) && (e.key === 'k' || e.key === 'K')) {
      e.preventDefault();
      input.focus();
      input.select();
    }
  });
}

// Category Filter Pills
function setupCategoryFilters() {
  const pills = document.querySelectorAll('.filter-pill[data-filter]');
  pills.forEach(pill => {
    pill.addEventListener('click', () => {
      pills.forEach(p => p.classList.remove('active'));
      pill.classList.add('active');
      activeCategory = pill.dataset.filter;
      renderActiveHubView();
    });
  });
}

// View Mode Toggles (Cards Grid vs Compact Table)
function setupViewModeToggles() {
  const btnCards = document.getElementById('btn-view-cards');
  const btnTable = document.getElementById('btn-view-table');
  const gridContainer = document.getElementById('apps-grid');
  const tableContainer = document.getElementById('process-table-wrap');

  if (btnCards && btnTable) {
    btnCards.addEventListener('click', () => {
      btnCards.classList.add('active');
      btnTable.classList.remove('active');
      viewMode = 'cards';
      gridContainer.classList.remove('hidden');
      tableContainer.classList.add('hidden');
      renderActiveHubView();
    });

    btnTable.addEventListener('click', () => {
      btnTable.classList.add('active');
      btnCards.classList.remove('active');
      viewMode = 'table';
      gridContainer.classList.add('hidden');
      tableContainer.classList.remove('hidden');
      renderActiveHubView();
    });
  }

  const sortSelect = document.getElementById('hub-sort-select');
  if (sortSelect) {
    sortSelect.addEventListener('change', (e) => {
      const val = e.target.value;
      if (val === 'memory-desc') currentSort = { col: 'memory', desc: true };
      else if (val === 'cpu-desc') currentSort = { col: 'cpu', desc: true };
      else if (val === 'name-asc') currentSort = { col: 'name', desc: false };
      else if (val === 'pid-asc') currentSort = { col: 'pid', desc: false };
      renderActiveHubView();
    });
  }
}

// Column Sorting & Resizing State
let preventSortUntil = 0;
const DEFAULT_COL_WIDTHS = { name: 380, pid: 80, type: 180, cpu: 90, memory: 170, ws: 130 };
const MIN_COL_WIDTHS = { name: 140, pid: 55, type: 90, cpu: 65, memory: 100, ws: 75 };

function setupSorting() {
  const headers = document.querySelectorAll('th.sortable');
  headers.forEach(th => {
    th.addEventListener('click', (e) => {
      if (e.target.classList.contains('col-resizer') || Date.now() < preventSortUntil) return;

      const col = th.dataset.sort;
      if (currentSort.col === col) {
        currentSort.desc = !currentSort.desc;
      } else {
        currentSort.col = col;
        currentSort.desc = true;
      }

      headers.forEach(h => {
        h.classList.remove('active-sort', 'desc', 'asc');
        const ind = h.querySelector('.sort-indicator');
        if (ind) ind.textContent = '';
      });

      th.classList.add('active-sort', currentSort.desc ? 'desc' : 'asc');
      const indicator = th.querySelector('.sort-indicator');
      if (indicator) indicator.textContent = currentSort.desc ? '▼' : '▲';

      renderActiveHubView();
    });
  });
}

function setupColumnResizing() {
  const table = document.getElementById('process-table');
  if (!table) return;

  try {
    const saved = localStorage.getItem('memoryblast_col_widths');
    if (saved) {
      const widths = JSON.parse(saved);
      applySavedWidths(table, widths);
    }
  } catch (_) {}

  const resizers = table.querySelectorAll('.col-resizer');
  resizers.forEach(resizer => {
    resizer.addEventListener('mousedown', initResize);
    resizer.addEventListener('dblclick', resetWidth);
  });

  function initResize(e) {
    e.preventDefault();
    e.stopPropagation();
    const th = e.target.parentElement;
    const resizer = e.target;
    const startX = e.pageX;
    const startWidth = th.offsetWidth;
    const colKey = th.dataset.col;

    resizer.classList.add('resizing');
    document.body.classList.add('col-resizing');

    function onMouseMove(moveEvent) {
      const delta = moveEvent.pageX - startX;
      const minW = MIN_COL_WIDTHS[colKey] || 60;
      const newWidth = Math.max(minW, startWidth + delta);
      th.style.width = newWidth + 'px';
    }

    function onMouseUp() {
      resizer.classList.remove('resizing');
      document.body.classList.remove('col-resizing');
      window.removeEventListener('mousemove', onMouseMove);
      window.removeEventListener('mouseup', onMouseUp);
      preventSortUntil = Date.now() + 250;
      saveColumnWidths(table);
    }

    window.addEventListener('mousemove', onMouseMove);
    window.addEventListener('mouseup', onMouseUp);
  }

  function resetWidth(e) {
    e.preventDefault();
    e.stopPropagation();
    const th = e.target.parentElement;
    const colKey = th.dataset.col;
    if (DEFAULT_COL_WIDTHS[colKey]) {
      th.style.width = DEFAULT_COL_WIDTHS[colKey] + 'px';
      saveColumnWidths(table);
    }
  }
}

function applySavedWidths(table, widths) {
  Object.keys(widths).forEach(key => {
    const th = table.querySelector(`th[data-col="${key}"]`);
    if (th && widths[key]) th.style.width = widths[key] + 'px';
  });
}

function saveColumnWidths(table) {
  try {
    const widths = {};
    table.querySelectorAll('th[data-col]').forEach(th => {
      const key = th.dataset.col;
      const w = parseInt(th.style.width, 10);
      if (key && w) widths[key] = w;
    });
    localStorage.setItem('memoryblast_col_widths', JSON.stringify(widths));
  } catch (_) {}
}

// Action Bar & Ergonomic Handlers
function setupActions() {
  // Magic Clean RAM Button
  const magicBtn = document.getElementById('btn-magic-clean');
  if (magicBtn) {
    magicBtn.addEventListener('click', () => executeMagicClean());
  }

  const quickOptBtn = document.getElementById('btn-quick-optimize');
  if (quickOptBtn) {
    quickOptBtn.addEventListener('click', () => executeMagicClean());
  }

  // Park Context / Workspace Hotkey
  const parkBtn = document.getElementById('btn-park-context');
  if (parkBtn) {
    parkBtn.addEventListener('click', () => parkCurrentContext());
  }

  const heroParkBtn = document.getElementById('btn-copilot-park-hero');
  if (heroParkBtn) {
    heroParkBtn.addEventListener('click', () => parkCurrentContext());
  }

  // Detect Tabs
  const detectTabsBtn = document.getElementById('btn-detect-tabs');
  if (detectTabsBtn) {
    detectTabsBtn.addEventListener('click', () => {
      fetch('/api/detect-tabs', { method: 'POST' }).catch(() => {});
      showToast('🔍 Scanning open tabs & window titles...');
    });
  }

  // Export Parquet
  const exportParquetBtn = document.getElementById('btn-export-parquet');
  if (exportParquetBtn) {
    exportParquetBtn.addEventListener('click', () => exportParquet());
  }
  const exportParquetInlineBtn = document.getElementById('btn-export-parquet-inline');
  if (exportParquetInlineBtn) {
    exportParquetInlineBtn.addEventListener('click', () => exportParquet());
  }

  // Refresh metrics
  const refreshBtn = document.getElementById('btn-refresh');
  if (refreshBtn) {
    refreshBtn.addEventListener('click', () => {
      fetch('/api/refresh', { method: 'POST' }).catch(() => {});
      showToast('Refreshing process metrics...');
    });
  }

  // Copilot ancillary buttons
  const refreshStashesBtn = document.getElementById('btn-refresh-stashes');
  if (refreshStashesBtn) refreshStashesBtn.addEventListener('click', () => loadStashedSessions());

  const refreshClipBtn = document.getElementById('btn-refresh-clipboard');
  if (refreshClipBtn) refreshClipBtn.addEventListener('click', () => loadClipboardVault());

  const stashRabbitBtn = document.getElementById('btn-stash-rabbithole');
  if (stashRabbitBtn) stashRabbitBtn.addEventListener('click', () => stashRabbitHole());

  // Window-level hotkey listeners
  window.addEventListener('keydown', (e) => {
    if (e.ctrlKey && e.shiftKey && (e.key === 'F' || e.key === 'f')) {
      e.preventDefault();
      parkCurrentContext();
    } else if (e.ctrlKey && e.shiftKey && (e.key === 'R' || e.key === 'r')) {
      e.preventDefault();
      stashRabbitHole();
    }
  });
}

// Magic Clean RAM Compaction
function executeMagicClean() {
  const magicBtn = document.getElementById('btn-magic-clean');
  if (magicBtn) magicBtn.classList.add('pulsing');

  if (!currentSnapshot || !currentSnapshot.processes) {
    showToast('✨ Optimizing system memory...');
    setTimeout(() => { if (magicBtn) magicBtn.classList.remove('pulsing'); }, 1000);
    return;
  }

  // Identify top memory consumers & background candidates for trimming
  const candidates = currentSnapshot.processes
    .filter(p => p.working_set_bytes > 50 * 1024 * 1024) // > 50MB
    .sort((a, b) => b.working_set_bytes - a.working_set_bytes)
    .slice(0, 8); // Top 8 memory hogs

  showToast(`✨ Magic Clean: Compacting working sets for ${candidates.length} apps...`);

  // Fire trim requests in parallel
  const trimPromises = candidates.map(c => 
    fetch('/api/trim', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ pid: c.pid })
    }).catch(() => {})
  );

  Promise.all(trimPromises).then(() => {
    setTimeout(() => {
      if (magicBtn) magicBtn.classList.remove('pulsing');
      showToast(`🎉 Magic Clean complete! Reclaimed memory across ${candidates.length} active apps.`);
    }, 600);
  });
}

// Slide-Over Technical Inspector Drawer Setup
function setupInspector() {
  const backdrop = document.getElementById('inspector-backdrop');
  const closeBtn = document.getElementById('insp-close-btn');

  if (backdrop) backdrop.addEventListener('click', closeInspector);
  if (closeBtn) closeBtn.addEventListener('click', closeInspector);

  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') closeInspector();
  });

  // Wire Drawer Footer Actions
  const focusBtn = document.getElementById('insp-action-focus');
  if (focusBtn) {
    focusBtn.addEventListener('click', () => {
      if (inspectedProcess) {
        focusTab(inspectedProcess.pid, inspectedProcess.ppid, inspectedProcess.display_title, inspectedProcess.debug_port);
      }
    });
  }

  const trimBtn = document.getElementById('insp-action-trim');
  if (trimBtn) {
    trimBtn.addEventListener('click', () => {
      if (inspectedProcess) {
        trimWorkingSet(inspectedProcess.pid, inspectedProcess.name);
      }
    });
  }

  const killBtn = document.getElementById('insp-action-kill');
  if (killBtn) {
    killBtn.addEventListener('click', () => {
      if (inspectedProcess) {
        openKillModal(inspectedProcess.pid, inspectedProcess.name);
      }
    });
  }
}

function openInspector(processItem) {
  inspectedProcess = processItem;
  selectedPid = processItem.pid;
  selectedPpid = processItem.ppid;
  selectedName = processItem.name;
  selectedTitle = processItem.display_title || '';
  selectedPort = processItem.debug_port || null;

  renderInspectorData();

  const drawer = document.getElementById('inspector-drawer');
  const backdrop = document.getElementById('inspector-backdrop');
  if (drawer) drawer.classList.add('open');
  if (backdrop) backdrop.classList.add('open');
}

function closeInspector() {
  inspectedProcess = null;
  const drawer = document.getElementById('inspector-drawer');
  const backdrop = document.getElementById('inspector-backdrop');
  if (drawer) drawer.classList.remove('open');
  if (backdrop) backdrop.classList.remove('open');
}

function updateInspectorIfOpen() {
  if (!inspectedProcess || !currentSnapshot) return;
  // Refresh inspectedProcess with latest snapshot data
  const updated = currentSnapshot.processes.find(p => p.pid === inspectedProcess.pid);
  if (updated) {
    inspectedProcess = updated;
    renderInspectorData();
  }
}

function renderInspectorData() {
  if (!inspectedProcess) return;
  const p = inspectedProcess;
  const id = resolveAppIdentity(p);
  const totalHostRam = (currentSnapshot && currentSnapshot.mem_stats.total_physical_bytes) || (32 * 1024 * 1024 * 1024);

  // Group metrics including children
  const groupWs = p.working_set_bytes + (p.children || []).reduce((acc, c) => acc + c.working_set_bytes, 0);
  const groupPriv = p.private_bytes + (p.children || []).reduce((acc, c) => acc + c.private_bytes, 0);
  const groupCpu = p.cpu_percent + (p.children || []).reduce((acc, c) => acc + c.cpu_percent, 0);
  const ramPct = ((groupWs / totalHostRam) * 100).toFixed(1);

  // Header
  document.getElementById('insp-icon-wrap').innerHTML = id.icon;
  document.getElementById('insp-app-name').textContent = id.name;
  document.getElementById('insp-app-cat').textContent = id.category;

  // KPI Grid
  document.getElementById('insp-ws-val').textContent = formatBytes(groupWs);
  document.getElementById('insp-priv-val').textContent = formatBytes(groupPriv);
  document.getElementById('insp-ram-pct').textContent = `${ramPct}%`;
  document.getElementById('insp-cpu-val').textContent = `${groupCpu.toFixed(1)}%`;

  // Diagnostics Props
  document.getElementById('insp-pid-val').textContent = p.pid;
  document.getElementById('insp-ppid-val').textContent = p.ppid;
  document.getElementById('insp-exe-val').textContent = p.name;
  document.getElementById('insp-title-val').textContent = p.display_title || '—';

  const portRow = document.getElementById('insp-port-row');
  if (p.debug_port) {
    portRow.style.display = 'flex';
    document.getElementById('insp-port-val').textContent = p.debug_port;
  } else {
    portRow.style.display = 'none';
  }

  // Children / Tabs
  const childrenSection = document.getElementById('insp-children-section');
  const childrenList = document.getElementById('insp-children-list');
  const childrenTitle = document.getElementById('insp-children-title');
  const children = p.children || [];

  childrenTitle.textContent = p.is_chromium ? `Grouped Tabs & Workers (${children.length})` : `Child Processes (${children.length})`;

  if (children.length === 0) {
    childrenList.innerHTML = `<div style="font-size: 12px; color: var(--text-muted); padding: 8px 0;">No active child threads or tabs</div>`;
  } else {
    childrenList.innerHTML = children.map(c => `
      <div class="inspector-child-item">
        <div style="min-width:0; flex:1;">
          <div style="font-weight:600; color:var(--text-primary); white-space:nowrap; overflow:hidden; text-overflow:ellipsis;">
            ${escapeHtml(c.display_title || c.name)}
          </div>
          <div style="font-size:11px; color:var(--text-muted);">
            PID ${c.pid} • ${escapeHtml(c.proc_type_label)} • ${formatBytes(c.working_set_bytes)}
          </div>
        </div>
        <div style="display:flex; gap:4px;">
          <button class="mini-action-btn" title="Focus" onclick="focusTab(${c.pid}, ${p.pid}, '${escapeHtml(c.display_title)}', ${p.debug_port || 'null'})">🎯</button>
          <button class="mini-action-btn" title="Sleep / Trim" onclick="trimWorkingSet(${c.pid}, '${escapeHtml(c.name)}')">💤</button>
          <button class="mini-action-btn" style="color:var(--accent-rose);" title="Close" onclick="openKillModal(${c.pid}, '${escapeHtml(c.name)}')">✕</button>
        </div>
      </div>
    `).join('');
  }
}

// Bring window or browser tab to front
function focusTab(pid, ppid, title, port) {
  showToast(`Locating window: ${title || selectedName || 'Process'}...`);
  fetch('/api/focus', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ pid, ppid: ppid || 0, title: title || '', port: port || null })
  })
  .then(res => res.json())
  .then(data => {
    if (data.success) {
      showToast(`🎯 Brought to foreground: ${title || selectedName}`);
    } else {
      showToast(`Notice: ${data.error}`);
    }
  })
  .catch(err => showToast(`Focus error: ${err}`));
}

// Modal handling
function setupModal() {
  const modal = document.getElementById('kill-modal');
  const confirmBtn = document.getElementById('modal-btn-confirm');
  const cancelBtn = document.getElementById('modal-btn-cancel');

  confirmBtn.addEventListener('click', () => {
    if (selectedPid) {
      terminateProcess(selectedPid, selectedName);
      modal.classList.add('hidden');
    }
  });

  cancelBtn.addEventListener('click', () => {
    modal.classList.add('hidden');
  });

  modal.addEventListener('click', (e) => {
    if (e.target === modal) modal.classList.add('hidden');
  });
}

function openKillModal(pid, name) {
  selectedPid = pid;
  selectedName = name;
  document.getElementById('modal-proc-name').textContent = name || 'Application';
  document.getElementById('modal-proc-pid').textContent = pid;
  document.getElementById('kill-modal').classList.remove('hidden');
}

function terminateProcess(pid, name) {
  fetch('/api/kill', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ pid })
  })
  .then(res => res.json())
  .then(data => {
    if (data.success) {
      showToast(`Closed ${name || 'PID ' + pid}`);
      if (inspectedProcess && inspectedProcess.pid === pid) {
        closeInspector();
      }
    } else {
      showToast(`Error: ${data.error || 'Failed to close process'}`);
    }
  })
  .catch(err => showToast(`Request error: ${err}`));
}

// Real-Time Server-Sent Events (SSE)
function connectEventSource() {
  const es = new EventSource('/api/stream');

  es.onmessage = (event) => {
    try {
      const snapshot = JSON.parse(event.data);
      onSnapshotReceived(snapshot);
    } catch (e) {
      console.error('SSE parse error:', e);
    }
  };

  es.onerror = () => {
    setTimeout(connectEventSource, 2000);
  };
}

function onSnapshotReceived(snapshot) {
  currentSnapshot = snapshot;

  // Apply Exocortex Cognitive State theme if available
  if (snapshot.exocortex && snapshot.exocortex.css_class) {
    document.body.className = document.body.className.replace(/\bcognitive-\S+/g, '').trim();
    document.body.classList.add(snapshot.exocortex.css_class);
  }

  // Record history
  const usedGb = snapshot.mem_stats.used_physical_bytes / (1024 * 1024 * 1024);
  const totalGb = snapshot.mem_stats.total_physical_bytes / (1024 * 1024 * 1024);
  const percent = totalGb > 0 ? (usedGb / totalGb) * 100 : 0;

  memoryHistory.push({
    time: Date.now(),
    inUseGb: usedGb,
    totalGb: totalGb,
    percent: percent
  });

  if (memoryHistory.length > MAX_HISTORY) {
    memoryHistory.shift();
  }

  // Update top banners & radial activity gauges
  updateTopBanners(snapshot);

  // Update primary hub view if currently on processes
  if (activeView === 'processes') {
    renderActiveHubView();
    updateInspectorIfOpen();
  }

  // Update performance view if active
  updatePerformanceStats(snapshot);
  if (activeView === 'performance') {
    drawCharts();
  }

  // Update treemap view if active
  if (activeView === 'treemap') {
    renderTreemap();
  }

  // Update analytics view if active
  if (activeView === 'analytics') {
    renderAnalytics();
  }

  // Update copilot view if active, and always update sidebar badge
  updateCopilotBadge(snapshot);
  if (activeView === 'copilot') {
    renderCopilot();
  }
}

// Health Dashboard & Radial Activity Gauges
function updateTopBanners(snapshot) {
  const mem = snapshot.mem_stats;
  const adminBadge = document.getElementById('admin-badge');
  if (adminBadge) {
    if (mem.is_admin) {
      adminBadge.textContent = '[ADMIN]';
      adminBadge.className = 'badge badge-admin';
    } else {
      adminBadge.textContent = '[STANDARD USER]';
      adminBadge.className = 'badge badge-user';
    }
  }

  const usedGb = (mem.used_physical_bytes / (1024 * 1024 * 1024)).toFixed(1);
  const totalGb = (mem.total_physical_bytes / (1024 * 1024 * 1024)).toFixed(1);
  const availGb = (mem.available_physical_bytes / (1024 * 1024 * 1024)).toFixed(1);
  const pct = mem.memory_load_percent;
  const commitUsed = formatBytes(mem.committed_bytes);
  const commitTot = formatBytes(mem.total_page_file_bytes);

  // Status Banner Headline & Subtitle
  const statusDot = document.getElementById('hub-status-dot');
  const statusText = document.getElementById('hub-status-text');
  const statusSub = document.getElementById('hub-status-sub');

  if (statusDot && statusText && statusSub) {
    statusDot.className = 'health-pulse-dot';
    if (pct < 65) {
      statusText.textContent = 'System is Running Smoothly';
      statusSub.textContent = `All applications are responsive • ${availGb} GB headroom available`;
    } else if (pct < 85) {
      statusDot.classList.add('amber');
      statusText.textContent = 'Moderate Memory Workload Active';
      statusSub.textContent = `System is utilizing ${pct}% RAM • ${availGb} GB available`;
    } else {
      statusDot.classList.add('rose');
      statusText.textContent = 'Memory Pressure High — 1-Click Optimize Available';
      statusSub.textContent = `High RAM utilization (${pct}%) • Run Magic Clean to compact memory`;
    }
  }

  // Gauge 1: System Pressure
  const pressureVal = document.getElementById('gauge-pressure-val');
  const pressureFill = document.getElementById('gauge-pressure-fill');
  const pressureStatus = document.getElementById('gauge-pressure-status');
  const pressureSub = document.getElementById('gauge-pressure-sub');

  if (pressureVal) pressureVal.textContent = `${pct}%`;
  if (pressureFill) {
    const C = 276.46; // 2 * PI * 44
    const offset = C - (Math.min(100, pct) / 100) * C;
    pressureFill.style.strokeDashoffset = offset;
    if (pct < 65) {
      pressureFill.setAttribute('stroke', 'url(#pressure-grad)');
    } else if (pct < 85) {
      pressureFill.setAttribute('stroke', '#F59E0B');
    } else {
      pressureFill.setAttribute('stroke', '#F43F5E');
    }
  }
  if (pressureStatus) {
    pressureStatus.className = 'gauge-status-badge';
    if (pct < 65) {
      pressureStatus.textContent = '● Optimal';
    } else if (pct < 85) {
      pressureStatus.classList.add('amber');
      pressureStatus.textContent = '● Elevated';
    } else {
      pressureStatus.classList.add('rose');
      pressureStatus.textContent = '● Heavy Load';
    }
  }
  if (pressureSub) pressureSub.textContent = `Committed: ${commitUsed} / ${commitTot}`;

  // Gauge 2: Active Workload
  const workloadVal = document.getElementById('gauge-workload-val');
  const workloadFill = document.getElementById('gauge-workload-fill');
  const workloadSub = document.getElementById('gauge-workload-sub');

  if (workloadVal) workloadVal.textContent = usedGb;
  if (workloadFill) {
    const C = 276.46;
    const memRatio = mem.total_physical_bytes > 0 ? (mem.used_physical_bytes / mem.total_physical_bytes) : 0;
    const offset = C - Math.min(1, memRatio) * C;
    workloadFill.style.strokeDashoffset = offset;
  }
  if (workloadSub) workloadSub.textContent = `${usedGb} of ${totalGb} GB Used • ${availGb} GB Free`;

  // Gauge 3: Background Apps & Tabs
  const roots = snapshot.processes || [];
  let totalTabs = 0;
  roots.forEach(r => {
    if (r.children) totalTabs += r.children.length;
  });

  const appsVal = document.getElementById('gauge-apps-val');
  const appsFill = document.getElementById('gauge-apps-fill');
  const appsSub = document.getElementById('gauge-apps-sub');

  if (appsVal) appsVal.textContent = roots.length;
  if (appsFill) {
    const C = 276.46;
    const appRatio = Math.min(1, roots.length / 40);
    appsFill.style.strokeDashoffset = C - (appRatio * C);
  }
  if (appsSub) appsSub.textContent = `${roots.length} Apps • ${totalTabs} Tabs`;
}

// Main Hub Renderer (Switches between Cards Grid & Compact Table)
function renderActiveHubView() {
  if (viewMode === 'cards') {
    renderAppsGrid();
  } else {
    renderProcessTable();
  }
}

// Render iOS App Cards Grid
function renderAppsGrid() {
  if (!currentSnapshot || !currentSnapshot.processes) return;
  const grid = document.getElementById('apps-grid');
  if (!grid) return;

  const totalHostRam = currentSnapshot.mem_stats.total_physical_bytes || (32 * 1024 * 1024 * 1024);
  const roots = [...currentSnapshot.processes];

  // Calculate highest group memory for relative bar scaling
  let maxMemory = 1;
  roots.forEach(r => {
    const m = r.private_bytes + (r.children || []).reduce((acc, c) => acc + c.private_bytes, 0);
    if (m > maxMemory) maxMemory = m;
  });

  // Calculate counts for filter pills
  let countAll = 0;
  let countBrowsers = 0;
  let countHeavy = 0;
  let countBackground = 0;

  roots.forEach(r => {
    const id = resolveAppIdentity(r);
    const m = r.private_bytes + (r.children || []).reduce((acc, c) => acc + c.private_bytes, 0);
    countAll++;
    if (r.is_chromium || id.category === 'Web Browser') countBrowsers++;
    if (m > 500 * 1024 * 1024) countHeavy++;
    if (id.category === 'Background Service' || r.proc_type_label === 'Service') countBackground++;
  });

  document.getElementById('count-all').textContent = countAll;
  document.getElementById('count-browsers').textContent = countBrowsers;
  document.getElementById('count-heavy').textContent = countHeavy;
  document.getElementById('count-background').textContent = countBackground;

  // Filter items
  const filtered = roots.filter(r => {
    const id = resolveAppIdentity(r);
    const m = r.private_bytes + (r.children || []).reduce((acc, c) => acc + c.private_bytes, 0);

    // Category filter
    if (activeCategory === 'browsers' && !(r.is_chromium || id.category === 'Web Browser')) return false;
    if (activeCategory === 'heavy' && m <= 500 * 1024 * 1024) return false;
    if (activeCategory === 'background' && !(id.category === 'Background Service' || r.proc_type_label === 'Service')) return false;

    // Search filter
    if (searchQuery) {
      const matchName = id.name.toLowerCase().includes(searchQuery) || (r.name && r.name.toLowerCase().includes(searchQuery));
      const matchTitle = r.display_title && r.display_title.toLowerCase().includes(searchQuery);
      const matchPid = r.pid.toString().includes(searchQuery);
      const matchChildren = (r.children || []).some(c => 
        (c.display_title && c.display_title.toLowerCase().includes(searchQuery)) ||
        (c.name && c.name.toLowerCase().includes(searchQuery))
      );
      if (!matchName && !matchTitle && !matchPid && !matchChildren) return false;
    }

    return true;
  });

  // Sort filtered items
  sortItems(filtered, currentSort.col, currentSort.desc);

  if (filtered.length === 0) {
    grid.innerHTML = `<div class="loading-cell" style="grid-column: 1 / -1;">No matching applications found.</div>`;
    return;
  }

  let html = '';

  for (const r of filtered) {
    const id = resolveAppIdentity(r);
    const children = r.children || [];
    const hasChildren = children.length > 0;
    const groupMem = r.private_bytes + children.reduce((acc, c) => acc + c.private_bytes, 0);
    const groupCpu = r.cpu_percent + children.reduce((acc, c) => acc + c.cpu_percent, 0);
    const ramSharePct = ((groupMem / totalHostRam) * 100).toFixed(1);
    const relBarWidth = Math.min(100, Math.max(5, (groupMem / maxMemory) * 100));

    // Progress bar color based on memory magnitude
    let barColorClass = '';
    if (groupMem > 1024 * 1024 * 1024) barColorClass = 'rose';
    else if (groupMem > 500 * 1024 * 1024) barColorClass = 'amber';

    const isSelected = selectedPid === r.pid;
    const isExpanded = expandedPids.has(r.pid);

    // Browser tabs badge markup
    let tabsBadgeHtml = '';
    if (hasChildren) {
      const tabRam = children.reduce((acc, c) => acc + c.working_set_bytes, 0);
      tabsBadgeHtml = `
        <div class="browser-tabs-badge ${isExpanded ? 'expanded' : ''}" data-tabs-toggle="${r.pid}">
          <div class="tabs-badge-left">
            <span class="tabs-badge-icon">🌐</span>
            <span>${children.length} Active Tabs • ${formatBytes(tabRam)}</span>
          </div>
          <span class="tabs-chevron">▼</span>
        </div>
        <div class="card-tabs-accordion ${isExpanded ? 'open' : ''}" id="tabs-acc-${r.pid}">
          ${children.map(c => `
            <div class="tab-accordion-item">
              <span class="tab-title-text" title="${escapeHtml(c.display_title || c.name)}">
                ${escapeHtml(c.display_title || c.name)}
              </span>
              <div class="tab-actions-mini">
                <span style="font-size:11px; color:var(--text-muted); margin-right:4px;">${formatBytes(c.working_set_bytes)}</span>
                <button class="mini-action-btn" title="Bring tab to front" onclick="event.stopPropagation(); focusTab(${c.pid}, ${r.pid}, '${escapeHtml(c.display_title)}', ${r.debug_port || 'null'})">🎯</button>
                <button class="mini-action-btn" title="Sleep / Trim Tab" onclick="event.stopPropagation(); trimWorkingSet(${c.pid}, '${escapeHtml(c.display_title || c.name)}')">💤</button>
              </div>
            </div>
          `).join('')}
        </div>
      `;
    }

    html += `
      <div class="app-card ${isSelected ? 'selected' : ''}" data-pid="${r.pid}">
        <div class="app-card-header">
          <div class="app-card-identity">
            <div class="app-brand-icon">
              ${id.icon}
            </div>
            <div class="app-name-wrap">
              <div class="app-name-row">
                <span class="app-name" title="${escapeHtml(id.name)}">${escapeHtml(id.name)}</span>
                <span class="app-category-chip">${id.category}</span>
              </div>
              <span class="app-subtitle-detail" title="${escapeHtml(r.display_title || r.name)}">
                ${escapeHtml(r.display_title || r.name)}
              </span>
            </div>
          </div>
          <span class="app-cpu-badge ${groupCpu > 5 ? 'high' : ''}">${groupCpu.toFixed(1)}% CPU</span>
        </div>

        <div class="app-card-metrics">
          <div class="app-metric-values">
            <span class="app-mem-text">${formatBytes(groupMem)}</span>
            <span class="app-mem-pct">${ramSharePct}% of Total RAM</span>
          </div>
          <div class="app-mem-bar-track">
            <div class="app-mem-bar-fill ${barColorClass}" style="width: ${relBarWidth}%;"></div>
          </div>
        </div>

        ${tabsBadgeHtml}

        <div class="app-card-footer">
          <button class="card-action-btn" title="Bring application to foreground" onclick="event.stopPropagation(); focusTab(${r.pid}, ${r.ppid}, '${escapeHtml(r.display_title)}', ${r.debug_port || 'null'})">
            🎯 Switch To
          </button>
          <button class="card-action-btn" title="Trim Working Set (Sleep)" onclick="event.stopPropagation(); trimWorkingSet(${r.pid}, '${escapeHtml(id.name)}')">
            💤 Sleep
          </button>
          <button class="card-action-btn btn-close" title="Close application" onclick="event.stopPropagation(); openKillModal(${r.pid}, '${escapeHtml(id.name)}')">
            ✕ Close
          </button>
          <button class="card-action-btn btn-inspect" title="View technical diagnostics" onclick="event.stopPropagation(); openInspectorFromPid(${r.pid})">
            ℹ️ Inspect
          </button>
        </div>
      </div>
    `;
  }

  grid.innerHTML = html;

  // Bind card clicks & tab accordion clicks
  grid.querySelectorAll('.app-card').forEach(card => {
    card.addEventListener('click', (e) => {
      // Toggle tabs accordion if clicking the tabs badge
      const toggleEl = e.target.closest('[data-tabs-toggle]');
      if (toggleEl) {
        const pid = parseInt(toggleEl.dataset.tabsToggle, 10);
        if (expandedPids.has(pid)) expandedPids.delete(pid);
        else expandedPids.add(pid);
        renderAppsGrid();
        return;
      }

      // Otherwise, open inspector for this card
      const pid = parseInt(card.dataset.pid, 10);
      openInspectorFromPid(pid);
    });
  });
}

function openInspectorFromPid(pid) {
  if (!currentSnapshot || !currentSnapshot.processes) return;
  const p = currentSnapshot.processes.find(item => item.pid === pid);
  if (p) openInspector(p);
}

// Render Compact Table View (For Dense Power-User Diagnosis)
function renderProcessTable() {
  if (!currentSnapshot || !currentSnapshot.processes) return;
  const tbody = document.getElementById('process-table-body');
  if (!tbody) return;

  const roots = [...currentSnapshot.processes];
  let maxMemory = 1;
  roots.forEach(r => {
    const m = r.private_bytes + (r.children || []).reduce((acc, c) => acc + c.private_bytes, 0);
    if (m > maxMemory) maxMemory = m;
  });

  sortItems(roots, currentSort.col, currentSort.desc);

  let html = '';
  let matchCount = 0;

  for (const root of roots) {
    const id = resolveAppIdentity(root);
    const isExpanded = expandedPids.has(root.pid);
    const hasChildren = root.children && root.children.length > 0;
    const groupMem = root.private_bytes + (root.children || []).reduce((acc, c) => acc + c.private_bytes, 0);
    const groupWs = root.working_set_bytes + (root.children || []).reduce((acc, c) => acc + c.working_set_bytes, 0);
    const groupCpu = root.cpu_percent + (root.children || []).reduce((acc, c) => acc + c.cpu_percent, 0);

    const matchesRoot = itemMatchesQuery(root, searchQuery) || id.name.toLowerCase().includes(searchQuery);
    const matchingChildren = (root.children || []).filter(c => itemMatchesQuery(c, searchQuery));
    const hasMatches = !searchQuery || matchesRoot || matchingChildren.length > 0;

    if (!hasMatches) continue;
    matchCount++;

    const isSelected = selectedPid === root.pid;
    const heatWidth = Math.min(100, Math.max(4, (groupMem / maxMemory) * 100));

    let toggleHtml = hasChildren 
      ? `<span class="tree-toggle" data-toggle="${root.pid}">${isExpanded ? '▼' : '▶'}</span>`
      : `<span class="tree-indent"></span>`;

    const titleSuffix = root.display_title ? ` — <span style="color:var(--accent-cyan)">${escapeHtml(root.display_title)}</span>` : '';
    const childBadge = hasChildren ? ` <span style="color:var(--text-muted)">(${root.children.length})</span>` : '';

    html += `
      <tr class="root-row ${isSelected ? 'selected' : ''}" data-pid="${root.pid}">
        <td class="col-name">
          <div class="tree-node">
            ${toggleHtml}
            <span class="tree-title">${escapeHtml(id.name)}</span>
            ${childBadge}
            ${titleSuffix}
          </div>
        </td>
        <td class="col-pid">${root.pid}</td>
        <td class="col-type">${escapeHtml(id.category)}</td>
        <td class="col-cpu" style="${groupCpu > 5 ? 'color:var(--accent-amber);font-weight:600' : ''}">${groupCpu.toFixed(1)}%</td>
        <td class="col-memory">
          <div class="heat-cell">
            <div class="heat-bar" style="width: ${heatWidth}%;"></div>
            <span>${formatBytes(groupMem)}</span>
          </div>
        </td>
        <td class="col-ws">${formatBytes(groupWs)}</td>
      </tr>
    `;

    if ((isExpanded || searchQuery) && hasChildren) {
      const displayChildren = searchQuery ? matchingChildren : root.children;
      for (const child of displayChildren) {
        const isChildSelected = selectedPid === child.pid;
        const childHeat = Math.min(100, Math.max(3, (child.private_bytes / maxMemory) * 100));
        const childTitle = child.display_title || child.name;

        html += `
          <tr class="child-row ${isChildSelected ? 'selected' : ''}" data-pid="${child.pid}" style="background: rgba(255,255,255,0.02);">
            <td class="col-name" style="padding-left: 36px;">
              <div class="tree-node">
                <span class="tree-indent" style="color:var(--text-muted)">├─</span>
                <span class="tree-title" style="color:var(--text-secondary)">${escapeHtml(childTitle)}</span>
              </div>
            </td>
            <td class="col-pid">${child.pid}</td>
            <td class="col-type" style="color:var(--text-muted)">${escapeHtml(child.proc_type_label)}</td>
            <td class="col-cpu">${child.cpu_percent.toFixed(1)}%</td>
            <td class="col-memory">
              <div class="heat-cell">
                <div class="heat-bar" style="width: ${childHeat}%;"></div>
                <span>${formatBytes(child.private_bytes)}</span>
              </div>
            </td>
            <td class="col-ws">${formatBytes(child.working_set_bytes)}</td>
          </tr>
        `;
      }
    }
  }

  tbody.innerHTML = html || '<tr><td colspan="6" class="loading-cell">No matching processes found.</td></tr>';

  tbody.querySelectorAll('tr[data-pid]').forEach(row => {
    row.addEventListener('click', (e) => {
      if (e.target.classList.contains('tree-toggle')) {
        const pid = parseInt(e.target.dataset.toggle, 10);
        if (expandedPids.has(pid)) expandedPids.delete(pid);
        else expandedPids.add(pid);
        renderProcessTable();
        return;
      }
      const pid = parseInt(row.dataset.pid, 10);
      openInspectorFromPid(pid);
    });
  });
}

function itemMatchesQuery(item, query) {
  if (!query) return true;
  return (item.name && item.name.toLowerCase().includes(query))
    || (item.display_title && item.display_title.toLowerCase().includes(query))
    || (item.proc_type_label && item.proc_type_label.toLowerCase().includes(query))
    || (item.pid && item.pid.toString().includes(query));
}

function sortItems(items, col, desc) {
  items.sort((a, b) => {
    let valA, valB;
    if (col === 'memory') {
      valA = a.private_bytes + (a.children || []).reduce((acc, c) => acc + c.private_bytes, 0);
      valB = b.private_bytes + (b.children || []).reduce((acc, c) => acc + c.private_bytes, 0);
    } else if (col === 'cpu') {
      valA = a.cpu_percent + (a.children || []).reduce((acc, c) => acc + c.cpu_percent, 0);
      valB = b.cpu_percent + (b.children || []).reduce((acc, c) => acc + c.cpu_percent, 0);
    } else if (col === 'pid') {
      valA = a.pid;
      valB = b.pid;
    } else {
      const nameA = resolveAppIdentity(a).name.toLowerCase();
      const nameB = resolveAppIdentity(b).name.toLowerCase();
      valA = nameA;
      valB = nameB;
    }

    if (valA < valB) return desc ? 1 : -1;
    if (valA > valB) return desc ? -1 : 1;
    return 0;
  });
}

// Performance View Metrics & Charts
function updatePerformanceStats(snapshot) {
  const mem = snapshot.mem_stats;
  const inUseGb = (mem.used_physical_bytes / (1024 * 1024 * 1024)).toFixed(1);
  const totalGb = (mem.total_physical_bytes / (1024 * 1024 * 1024)).toFixed(1);
  const availGb = (mem.available_physical_bytes / (1024 * 1024 * 1024)).toFixed(1);
  const commitUsedGb = (mem.committed_bytes / (1024 * 1024 * 1024)).toFixed(1);
  const commitTotGb = (mem.total_page_file_bytes / (1024 * 1024 * 1024)).toFixed(1);
  const cachedGb = (mem.system_cache_bytes / (1024 * 1024 * 1024)).toFixed(1);
  const pagedGb = (mem.paged_pool_bytes / (1024 * 1024 * 1024)).toFixed(1);
  const nonpagedGb = (mem.non_paged_pool_bytes / (1024 * 1024 * 1024)).toFixed(1);

  // Mini sidebar card
  const miniPercent = document.getElementById('mini-mem-percent');
  const miniVal = document.getElementById('mini-mem-val');
  const perfCap = document.getElementById('perf-total-capacity');
  if (miniPercent) miniPercent.textContent = `${mem.memory_load_percent}%`;
  if (miniVal) miniVal.textContent = `${inUseGb} / ${totalGb} GB (${mem.memory_load_percent}%)`;
  if (perfCap) perfCap.textContent = `${totalGb} GB Host Memory`;

  // Grid
  const perfInUse = document.getElementById('perf-in-use');
  const perfAvail = document.getElementById('perf-avail');
  const perfCommit = document.getElementById('perf-commit');
  const perfCached = document.getElementById('perf-cached');
  const perfPaged = document.getElementById('perf-paged');
  const perfNonpaged = document.getElementById('perf-nonpaged');
  const chartMaxLabel = document.getElementById('chart-max-label');

  if (perfInUse) perfInUse.textContent = `${inUseGb} GB`;
  if (perfAvail) perfAvail.textContent = `${availGb} GB`;
  if (perfCommit) perfCommit.textContent = `${commitUsedGb} / ${commitTotGb} GB`;
  if (perfCached) perfCached.textContent = `${cachedGb} GB`;
  if (perfPaged) perfPaged.textContent = `${pagedGb} GB`;
  if (perfNonpaged) perfNonpaged.textContent = `${nonpagedGb} GB`;
  if (chartMaxLabel) chartMaxLabel.textContent = `${totalGb} GB`;

  // Composition bar
  const totalBytes = mem.total_physical_bytes || 1;
  const inUsePct = (mem.used_physical_bytes / totalBytes) * 100;
  const cachedPct = Math.min(100 - inUsePct, (mem.system_cache_bytes / totalBytes) * 100);
  const availPct = Math.max(0, 100 - inUsePct - cachedPct);

  const compInUse = document.getElementById('comp-in-use');
  const compCached = document.getElementById('comp-cached');
  const compAvail = document.getElementById('comp-avail');

  if (compInUse) compInUse.style.width = `${inUsePct}%`;
  if (compCached) compCached.style.width = `${cachedPct}%`;
  if (compAvail) compAvail.style.width = `${availPct}%`;
}

// 60-Second Real-Time Canvas Charts
let mainCanvas, mainCtx, sparkCanvas, sparkCtx;

function initCanvas() {
  mainCanvas = document.getElementById('memory-chart');
  sparkCanvas = document.getElementById('mini-mem-sparkline');
  if (mainCanvas) mainCtx = mainCanvas.getContext('2d');
  if (sparkCanvas) sparkCtx = sparkCanvas.getContext('2d');

  function resize() {
    if (!mainCanvas) return;
    const rect = mainCanvas.getBoundingClientRect();
    mainCanvas.width = rect.width * window.devicePixelRatio;
    mainCanvas.height = rect.height * window.devicePixelRatio;
    drawCharts();
  }

  window.addEventListener('resize', resize);
  setTimeout(resize, 50);
}

function drawCharts() {
  if (!mainCanvas || !mainCtx || memoryHistory.length === 0) return;

  const w = mainCanvas.width;
  const h = mainCanvas.height;

  mainCtx.clearRect(0, 0, w, h);

  // Background Grid Lines
  mainCtx.strokeStyle = 'rgba(255, 255, 255, 0.05)';
  mainCtx.lineWidth = 1;
  for (let i = 1; i <= 4; i++) {
    const y = (h / 5) * i;
    mainCtx.beginPath();
    mainCtx.moveTo(0, y);
    mainCtx.lineTo(w, y);
    mainCtx.stroke();
  }

  // Draw Area Line Chart
  const count = memoryHistory.length;
  if (count > 1) {
    mainCtx.beginPath();
    for (let i = 0; i < count; i++) {
      const x = (i / (MAX_HISTORY - 1)) * w;
      const y = h - ((memoryHistory[i].percent / 100) * h);
      if (i === 0) mainCtx.moveTo(x, y);
      else mainCtx.lineTo(x, y);
    }

    // Stroke line
    mainCtx.strokeStyle = '#38BDF8';
    mainCtx.lineWidth = 2.5 * window.devicePixelRatio;
    mainCtx.lineJoin = 'round';
    mainCtx.stroke();

    // Fill area
    mainCtx.lineTo((count - 1) / (MAX_HISTORY - 1) * w, h);
    mainCtx.lineTo(0, h);
    mainCtx.closePath();

    const gradient = mainCtx.createLinearGradient(0, 0, 0, h);
    gradient.addColorStop(0, 'rgba(56, 189, 248, 0.35)');
    gradient.addColorStop(1, 'rgba(56, 189, 248, 0.0)');
    mainCtx.fillStyle = gradient;
    mainCtx.fill();
  }

  // Sparkline in Sidebar
  if (sparkCanvas && sparkCtx) {
    const sw = sparkCanvas.width;
    const sh = sparkCanvas.height;
    sparkCtx.clearRect(0, 0, sw, sh);

    if (count > 1) {
      sparkCtx.strokeStyle = '#38BDF8';
      sparkCtx.lineWidth = 1.5;
      sparkCtx.beginPath();
      for (let i = 0; i < count; i++) {
        const sx = (i / (MAX_HISTORY - 1)) * sw;
        const sy = sh - ((memoryHistory[i].percent / 100) * sh);
        if (i === 0) sparkCtx.moveTo(sx, sy);
        else sparkCtx.lineTo(sx, sy);
      }
      sparkCtx.stroke();
    }
  }
}

// Helpers
function formatBytes(bytes) {
  if (bytes == null || isNaN(bytes)) return '0 B';
  const KB = 1024;
  const MB = 1024 * KB;
  const GB = 1024 * MB;

  if (bytes >= GB) return (bytes / GB).toFixed(1) + ' GB';
  if (bytes >= MB) return (bytes / MB).toFixed(1) + ' MB';
  if (bytes >= KB) return (bytes / KB).toFixed(0) + ' KB';
  return bytes + ' B';
}

function escapeHtml(str) {
  if (!str) return '';
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

let toastTimer = null;
function showToast(msg) {
  const toast = document.getElementById('toast');
  if (!toast) return;
  toast.innerHTML = msg;
  toast.classList.remove('hidden');
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => toast.classList.add('hidden'), 3800);
}

// Memory Optimization Actions
function trimWorkingSet(pid, name) {
  showToast(`✂️ Trimming working set for ${name || 'PID ' + pid}...`);
  fetch('/api/trim', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ pid })
  })
  .then(res => res.json())
  .then(data => {
    if (data.success) {
      showToast(`✨ Successfully compacted memory for ${name || 'PID ' + pid}`);
    } else {
      showToast(`Notice: ${data.error}`);
    }
  })
  .catch(err => showToast(`Trim error: ${err}`));
}

function exportParquet() {
  showToast('💾 Exporting snapshot to Apache Parquet...');
  fetch('/api/export-parquet', { method: 'POST' })
    .then(res => res.json())
    .then(data => {
      if (data.success) {
        showToast(`✅ Parquet file saved: ${data.path}`);
      } else {
        showToast(`Export notice: ${data.error}`);
      }
    })
    .catch(err => showToast(`Parquet export error: ${err}`));
}

// VIEW 3: Interactive Squarified Treemap
let treemapCurrentCluster = null;
let treemapSelectedNode = null;

function renderTreemap() {
  if (!currentSnapshot || !currentSnapshot.treemap) return;
  const viewport = document.getElementById('treemap-viewport');
  if (!viewport) return;

  const root = currentSnapshot.treemap;
  let targetNodes = root.children;
  let crumbTitle = '🖥️ System Memory';

  if (treemapCurrentCluster) {
    const cluster = root.children.find(c => c.id === treemapCurrentCluster);
    if (cluster && cluster.children && cluster.children.length > 0) {
      targetNodes = cluster.children;
      crumbTitle = `🖥️ System Memory > ${cluster.title}`;
    } else {
      treemapCurrentCluster = null;
    }
  }

  const breadcrumbs = document.getElementById('treemap-breadcrumbs');
  if (breadcrumbs) {
    breadcrumbs.innerHTML = `<span class="crumb" id="crumb-root" style="cursor:pointer">${escapeHtml(crumbTitle)}</span>`;
    const crumbRoot = document.getElementById('crumb-root');
    if (crumbRoot) {
      crumbRoot.addEventListener('click', () => {
        treemapCurrentCluster = null;
        renderTreemap();
      });
    }
  }

  const resetBtn = document.getElementById('btn-treemap-reset');
  if (resetBtn) {
    resetBtn.onclick = () => {
      treemapCurrentCluster = null;
      treemapSelectedNode = null;
      renderTreemap();
    };
  }

  const vw = viewport.clientWidth || 1000;
  const vh = viewport.clientHeight || 600;

  if (vw <= 0 || vh <= 0) return;

  viewport.innerHTML = '';

  for (const node of targetNodes) {
    if (!node.rect) continue;

    const rx = (node.rect.x / 1200.0) * vw;
    const ry = (node.rect.y / 700.0) * vh;
    const rw = (node.rect.width / 1200.0) * vw;
    const rh = (node.rect.height / 700.0) * vh;

    if (rw < 10 || rh < 10) continue;

    const box = document.createElement('div');
    box.className = `treemap-box ${treemapSelectedNode && treemapSelectedNode.id === node.id ? 'selected' : ''}`;
    box.style.left = `${Math.round(rx)}px`;
    box.style.top = `${Math.round(ry)}px`;
    box.style.width = `${Math.max(12, Math.round(rw) - 4)}px`;
    box.style.height = `${Math.max(12, Math.round(rh) - 4)}px`;
    box.style.backgroundColor = node.color || '#334155';
    box.style.opacity = '0.92';

    const titleEl = document.createElement('div');
    titleEl.className = 'treemap-box-title';
    titleEl.textContent = node.title;
    box.appendChild(titleEl);

    if (rh > 36) {
      const subEl = document.createElement('div');
      subEl.className = 'treemap-box-sub';
      subEl.textContent = `${node.formatted_size} • ${node.subtitle}`;
      box.appendChild(subEl);
    }

    box.addEventListener('click', (e) => {
      e.stopPropagation();
      treemapSelectedNode = node;

      const inspTitle = document.getElementById('insp-title');
      const inspSub = document.getElementById('insp-sub');
      if (inspTitle) inspTitle.textContent = node.title;
      if (inspSub) inspSub.textContent = `${node.formatted_size} | Category: ${node.category} | ${node.subtitle}`;

      const trimBtn = document.getElementById('insp-btn-trim');
      const focusBtn = document.getElementById('insp-btn-focus');
      const killBtn = document.getElementById('insp-btn-kill');

      if (node.pid) {
        if (trimBtn) {
          trimBtn.disabled = false;
          trimBtn.onclick = () => trimWorkingSet(node.pid, node.title);
        }
        if (focusBtn) {
          focusBtn.disabled = false;
          focusBtn.onclick = () => focusTab(node.pid, 0, node.title, null);
        }
        if (killBtn) {
          killBtn.disabled = false;
          killBtn.onclick = () => openKillModal(node.pid, node.title);
        }
      }

      if (node.category === 'browser_cluster' && node.children && node.children.length > 0) {
        treemapCurrentCluster = node.id;
        renderTreemap();
        return;
      }

      viewport.querySelectorAll('.treemap-box').forEach(b => b.classList.remove('selected'));
      box.classList.add('selected');
    });

    viewport.appendChild(box);
  }
}

// VIEW 4: Cognitive AI & Behavioral Analytics
function renderAnalytics() {
  if (!currentSnapshot) return;

  const oom = currentSnapshot.oom_forecast;
  if (oom) {
    const badge = document.getElementById('oom-badge');
    const val = document.getElementById('oom-forecast-val');
    const sub = document.getElementById('oom-forecast-sub');

    if (badge) {
      badge.textContent = oom.risk_level;
      badge.className = 'card-badge';
      if (oom.risk_level === 'CRITICAL') badge.classList.add('badge-danger');
      else if (oom.risk_level === 'WARNING') badge.classList.add('badge-warn');
    }

    if (val) {
      if (oom.estimated_seconds_to_oom != null) {
        const mins = (oom.estimated_seconds_to_oom / 60).toFixed(0);
        val.textContent = `⚠️ ~${mins} mins to Commit Ceiling`;
      } else {
        val.textContent = `Stable — Commit Load: ${oom.current_commit_pct.toFixed(1)}%`;
      }
    }

    if (sub) {
      sub.textContent = `Host commit velocity: ${oom.velocity_mb_min >= 0 ? '+' : ''}${oom.velocity_mb_min.toFixed(1)} MB/min | Limit: ${formatBytes(currentSnapshot.mem_stats.total_page_file_bytes)}`;
    }
  }

  const recGrid = document.getElementById('recommendations-grid');
  const recBadge = document.getElementById('rec-count-badge');
  const recs = currentSnapshot.recommendations || [];

  if (recBadge) recBadge.textContent = `${recs.length} Suggestions`;

  if (recGrid) {
    if (recs.length === 0) {
      recGrid.innerHTML = '<div class="rec-empty-card">✨ System memory usage is optimal. No anomalous leaks or stale background tabs detected.</div>';
    } else {
      let recHtml = '';
      for (const rec of recs) {
        const uClass = rec.urgency.toLowerCase();
        recHtml += `
          <div class="rec-card rec-${uClass}">
            <div class="rec-header">
              <div class="rec-title">${escapeHtml(rec.title)}</div>
              <span class="rec-urgency ${rec.urgency}">${rec.urgency}</span>
            </div>
            <div class="rec-desc">${escapeHtml(rec.description)}</div>
            <div class="rec-footer">
              <div class="rec-reclaim">Potential reclaim: ${formatBytes(rec.potential_reclaim_bytes)}</div>
              <div class="rec-actions">
                ${rec.target_pid > 0 ? `
                  <button class="fluent-btn btn-trim-action" data-pid="${rec.target_pid}" data-name="${escapeHtml(rec.target_name)}">✂️ Trim RAM</button>
                  <button class="fluent-btn btn-focus-action" data-pid="${rec.target_pid}" data-name="${escapeHtml(rec.target_name)}">🎯 Focus</button>
                  <button class="fluent-btn btn-danger btn-kill-action" data-pid="${rec.target_pid}" data-name="${escapeHtml(rec.target_name)}">✕</button>
                ` : ''}
              </div>
            </div>
          </div>
        `;
      }
      recGrid.innerHTML = recHtml;

      recGrid.querySelectorAll('.btn-trim-action').forEach(btn => {
        btn.addEventListener('click', () => {
          const pid = parseInt(btn.dataset.pid, 10);
          trimWorkingSet(pid, btn.dataset.name);
        });
      });

      recGrid.querySelectorAll('.btn-focus-action').forEach(btn => {
        btn.addEventListener('click', () => {
          const pid = parseInt(btn.dataset.pid, 10);
          focusTab(pid, 0, btn.dataset.name, null);
        });
      });

      recGrid.querySelectorAll('.btn-kill-action').forEach(btn => {
        btn.addEventListener('click', () => {
          const pid = parseInt(btn.dataset.pid, 10);
          openKillModal(pid, btn.dataset.name);
        });
      });
    }
  }

  const anomTbody = document.getElementById('anomalies-table-body');
  const anomalies = currentSnapshot.anomalies || [];
  const profiles = currentSnapshot.profiles || [];
  const profileMap = new Map();
  for (const p of profiles) {
    profileMap.set(p.pid, p);
  }

  if (anomTbody) {
    if (anomalies.length === 0) {
      anomTbody.innerHTML = '<tr><td colspan="7" class="loading-cell">No anomalies detected. All processes within normal statistical bounds.</td></tr>';
    } else {
      const sorted = [...anomalies].sort((a, b) => b.leak_velocity_mb_min - a.leak_velocity_mb_min);
      let anomHtml = '';

      for (const a of sorted.slice(0, 30)) {
        const prof = profileMap.get(a.pid);
        const cluster = prof ? prof.cluster : 'StandardApp';
        const isLeaking = a.leak_velocity_mb_min > 10.0;

        anomHtml += `
          <tr>
            <td><strong>${escapeHtml(a.name)}</strong></td>
            <td>${a.pid}</td>
            <td>${formatBytes(a.current_bytes)}</td>
            <td>
              <span class="${isLeaking ? 'badge-leak' : ''}">
                ${a.leak_velocity_mb_min >= 0 ? '+' : ''}${a.leak_velocity_mb_min.toFixed(1)} MB/min
              </span>
            </td>
            <td>${a.z_score >= 0 ? '+' : ''}${a.z_score.toFixed(2)}</td>
            <td><span class="badge-cluster">${cluster}</span></td>
            <td>
              <button class="fluent-btn btn-trim-action" data-pid="${a.pid}" data-name="${escapeHtml(a.name)}" style="padding:2px 8px; font-size:11px;">✂️ Trim</button>
            </td>
          </tr>
        `;
      }

      anomTbody.innerHTML = anomHtml;

      anomTbody.querySelectorAll('.btn-trim-action').forEach(btn => {
        btn.addEventListener('click', () => {
          const pid = parseInt(btn.dataset.pid, 10);
          trimWorkingSet(pid, btn.dataset.name);
        });
      });
    }
  }
}

// -------------------------------------------------------------
// FOCUS COPILOT & EXECUTIVE ASSISTANT IMPLEMENTATION
// -------------------------------------------------------------
function updateCopilotBadge(snapshot) {
  const copilot = snapshot.copilot;
  const badge = document.getElementById('copilot-badge');
  if (!badge) return;

  if (copilot && copilot.assessment) {
    const alerts = (copilot.assessment.forgotten_loops || []).length;
    if (alerts > 0) {
      badge.textContent = alerts;
      badge.classList.remove('hidden');
    } else {
      badge.classList.add('hidden');
    }
  } else {
    badge.classList.add('hidden');
  }
}

function parkCurrentContext() {
  showToast('📦 Parking current context & freezing tabs...');
  fetch('/api/copilot/park', { method: 'POST' })
    .then(res => res.json())
    .then(data => {
      if (data.success && data.session) {
        const s = data.session;
        const mb = (s.total_memory_bytes / (1024 * 1024)).toFixed(1);
        showToast(`🎉 Parked ${s.tab_count} tabs! Reclaimed ${mb} MB RAM.`);
        if (activeView === 'copilot') {
          loadStashedSessions();
        }
      } else {
        showToast(`Notice: ${data.error || 'Failed to park context'}`);
      }
    })
    .catch(err => showToast(`Park error: ${err}`));
}

function renderCopilot() {
  if (!currentSnapshot || !currentSnapshot.copilot) return;

  const copilot = currentSnapshot.copilot;
  const assessment = copilot.assessment || {};
  const status = copilot.status || {};
  const exocortex = currentSnapshot.exocortex || {};

  // Zen Mode Banner
  const zenBanner = document.getElementById('zen-mode-banner');
  const zenSub = document.getElementById('zen-mode-subtitle');
  if (zenBanner && zenSub) {
    if (exocortex.is_zen_mode && exocortex.flow_session) {
      const mins = Math.floor(exocortex.flow_session.duration_secs / 60);
      zenSub.textContent = `Alerts suppressed. ${mins}m elapsed.`;
      zenBanner.style.display = 'flex';
    } else {
      zenBanner.style.display = 'none';
    }
  }

  // Evening Closure Banner
  // We'll update this below in loadExecutiveBriefing or fetch it directly.
  
  // 1. KPI: Context Drift Index
  const driftVal = document.getElementById('copilot-drift-val');
  const driftBadge = document.getElementById('copilot-drift-badge');
  const switchRate = document.getElementById('copilot-switch-rate');

  if (driftVal) {
    driftVal.textContent = (assessment.context_drift_index || 0).toFixed(2);
  }
  if (driftBadge) {
    const sev = assessment.drift_severity || 'Calm';
    driftBadge.textContent = sev;
    if (sev === 'Calm' || sev === 'Focused Flow') {
      driftBadge.className = 'badge badge-success';
    } else if (sev === 'Elevated Drift') {
      driftBadge.className = 'badge badge-warning';
    } else {
      driftBadge.className = 'badge badge-danger';
    }
  }
  if (switchRate) {
    switchRate.textContent = `${(assessment.switches_per_min || 0).toFixed(1)} context switches / min`;
  }

  // 2. KPI: Cognitive Working Set
  const tabsVal = document.getElementById('copilot-tabs-val');
  const capLabel = document.getElementById('copilot-cap-label');
  const capFill = document.getElementById('copilot-cap-fill');
  const capSubtext = document.getElementById('copilot-cap-subtext');

  const tabCount = assessment.active_tab_count || 0;
  const tabCap = assessment.working_memory_cap || 12;

  if (tabsVal) tabsVal.textContent = tabCount;
  if (capLabel) capLabel.textContent = `/ ${tabCap} Soft Cap`;
  if (capFill) {
    const pct = Math.min(100, Math.round((tabCount / tabCap) * 100));
    capFill.style.width = `${pct}%`;
    if (pct >= 85) {
      capFill.classList.add('cap-warning');
    } else {
      capFill.classList.remove('cap-warning');
    }
  }
  if (capSubtext) {
    if (tabCount >= tabCap) {
      capSubtext.textContent = '⚠️ Working memory ceiling reached. Park older tasks!';
      capSubtext.style.color = '#ef4444';
    } else if (tabCount >= tabCap - 2) {
      capSubtext.textContent = 'Nearing capacity. Consider wrapping up open loops.';
      capSubtext.style.color = '#f59e0b';
    } else {
      capSubtext.textContent = 'Operating within comfortable mental bandwidth.';
      capSubtext.style.color = 'var(--text-secondary)';
    }
  }

  // 3. KPI: Celebration metrics
  const reclaimedVal = document.getElementById('copilot-reclaimed-val');
  const stashedCount = document.getElementById('copilot-stashed-count');

  if (reclaimedVal) {
    reclaimedVal.textContent = `${(status.total_memory_reclaimed_mb || 0).toFixed(1)} MB`;
  }
  if (stashedCount) {
    stashedCount.textContent = `${status.total_tabs_stashed || 0} tasks cleanly parked & finished`;
  }

  // 4. Section 1: Forgotten Loops
  const loopsGrid = document.getElementById('forgotten-loops-grid');
  const countBadge = document.getElementById('forgotten-count-badge');
  const loops = assessment.forgotten_loops || [];

  if (countBadge) {
    countBadge.textContent = `${loops.length} Open Loops`;
  }

  if (loopsGrid) {
    if (loops.length === 0) {
      loopsGrid.innerHTML = '<div class="rec-empty-card">All current tabs and tasks are actively maintained. High focus flow state! 🚀</div>';
    } else {
      let html = '';
      for (const loop of loops) {
        html += `
          <div class="loop-card">
            <div class="loop-header">
              <div>
                <div class="loop-title" title="${escapeHtml(loop.title)}">${escapeHtml(loop.title)}</div>
                <div class="loop-meta">
                  <span>${escapeHtml(loop.browser_family)}</span> •
                  <span>PID ${loop.pid}</span> •
                  <span>${loop.idle_duration_mins}m idle</span> •
                  <span>${loop.memory_mb.toFixed(0)} MB</span>
                </div>
              </div>
              <span class="badge badge-warning">Score: ${loop.cognitive_cost_score.toFixed(1)}</span>
            </div>
            <div class="loop-prompt">
              💬 "${escapeHtml(loop.suggested_prompt)}"
            </div>
            <div class="loop-actions">
              <button class="fluent-btn btn-primary btn-loop-focus" data-pid="${loop.pid}" data-title="${escapeHtml(loop.title)}">🎯 Focus</button>
              <button class="fluent-btn btn-loop-stash" data-pid="${loop.pid}" data-title="${escapeHtml(loop.title)}">📦 Stash</button>
              <button class="fluent-btn btn-loop-snooze" data-pid="${loop.pid}" data-title="${escapeHtml(loop.title)}">⏳ Snooze 30m</button>
            </div>
          </div>
        `;
      }
      loopsGrid.innerHTML = html;

      loopsGrid.querySelectorAll('.btn-loop-focus').forEach(btn => {
        btn.addEventListener('click', () => {
          executeCopilotAction('focus', parseInt(btn.dataset.pid, 10), btn.dataset.title);
        });
      });
      loopsGrid.querySelectorAll('.btn-loop-stash').forEach(btn => {
        btn.addEventListener('click', () => {
          executeCopilotAction('stash', parseInt(btn.dataset.pid, 10), btn.dataset.title);
        });
      });
      loopsGrid.querySelectorAll('.btn-loop-snooze').forEach(btn => {
        btn.addEventListener('click', () => {
          executeCopilotAction('snooze', parseInt(btn.dataset.pid, 10), btn.dataset.title);
        });
      });
    }
  }
}

function executeCopilotAction(action, pid, title) {
  fetch('/api/copilot/action', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ action, pid, title })
  })
  .then(res => res.json())
  .then(data => {
    if (data.success) {
      showToast(`✨ ${data.message || 'Action executed'}`);
      if (action === 'stash') {
        renderCopilot();
        loadStashedSessions();
      }
    } else {
      showToast(`Notice: ${data.error}`);
    }
  })
  .catch(err => showToast(`Action error: ${err}`));
}

function loadStashedSessions() {
  const container = document.getElementById('stashes-list');
  if (!container) return;

  fetch('/api/copilot/stashes')
    .then(res => res.json())
    .then(data => {
      if (!data.success || !data.sessions || data.sessions.length === 0) {
        container.innerHTML = '<div class="rec-empty-card">No stashed sessions yet. Press Ctrl+Shift+F anytime to park your open tabs!</div>';
        return;
      }

      let html = '';
      for (const s of data.sessions) {
        const dateStr = new Date(s.timestamp_ms).toLocaleString();
        const mb = (s.total_memory_bytes / (1024 * 1024)).toFixed(1);
        const statusBadge = s.restored ? '<span class="badge badge-success">Restored</span>' : '<span class="badge badge-warning">Archived</span>';

        html += `
          <div class="stash-card">
            <div class="stash-info">
              <div class="stash-name">
                ${escapeHtml(s.session_name)} ${statusBadge}
              </div>
              <div class="stash-sub">
                Saved on ${dateStr} • ${s.tab_count} tabs preserved • Reclaimed ${mb} MB
              </div>
            </div>
            <div class="stash-actions">
              <button class="fluent-btn btn-primary btn-restore-stash" data-id="${s.id}">
                🔄 Restore Session
              </button>
            </div>
          </div>
        `;
      }
      container.innerHTML = html;

      container.querySelectorAll('.btn-restore-stash').forEach(btn => {
        btn.addEventListener('click', () => {
          restoreStashedSession(parseInt(btn.dataset.id, 10));
        });
      });
    })
    .catch(() => {
      container.innerHTML = '<div class="rec-empty-card">Unable to load stashed sessions.</div>';
    });
}

function restoreStashedSession(id) {
  showToast(`Reopening tabs for session #${id}...`);
  fetch('/api/copilot/restore', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ id })
  })
  .then(res => res.json())
  .then(data => {
    if (data.success) {
      showToast(`✨ Restored session #${id} (${data.restored_count} URLs reopened)!`);
      loadStashedSessions();
    } else {
      showToast(`Notice: ${data.error}`);
    }
  })
  .catch(err => showToast(`Restore error: ${err}`));
}

function loadResumeAnchor() {
  fetch('/api/copilot/anchor')
    .then(res => res.json())
    .then(data => {
      if (!data.success || !data.anchor) return;
      const a = data.anchor;
      const s1 = document.getElementById('anchor-summary-1');
      const s2 = document.getElementById('anchor-summary-2');
      const launchBtn = document.getElementById('btn-anchor-launch');

      if (s1) s1.textContent = a.summary_line1;
      if (s2) s2.textContent = a.summary_line2;

      if (launchBtn) {
        if (a.ide_launch_url) {
          launchBtn.href = a.ide_launch_url;
          launchBtn.style.display = 'inline-flex';
        } else {
          launchBtn.style.display = 'none';
        }
      }
    })
    .catch(() => {});
}

function loadClipboardVault() {
  const container = document.getElementById('clipboard-vault-list');
  if (!container) return;

  fetch('/api/copilot/clipboard')
    .then(res => res.json())
    .then(data => {
      if (!data.success || !data.events || data.events.length === 0) {
        container.innerHTML = '<div class="rec-empty-card">No clipboard history recorded yet. Copy any code, error, or command to see it here!</div>';
        return;
      }

      let html = '';
      for (const ev of data.events) {
        const timeStr = new Date(ev.timestamp_ms).toLocaleTimeString();
        const redactedBadge = ev.was_redacted ? '<span class="badge-redacted">Secret Protected</span>' : '';
        const catBadge = `<span class="badge badge-info">${escapeHtml(ev.category)}</span>`;

        html += `
          <div class="clip-card">
            <div class="clip-main">
              <div class="clip-text" title="${escapeHtml(ev.content)}">
                ${escapeHtml(ev.content)}
              </div>
              <div class="clip-meta">
                ${catBadge}
                ${redactedBadge}
                <span>${timeStr}</span> •
                <span>${ev.char_count} chars</span>
              </div>
            </div>
            <button class="fluent-btn btn-copy-clip" data-text="${escapeHtml(ev.content)}" style="padding:4px 10px; font-size:11px;">📋 Copy</button>
          </div>
        `;
      }
      container.innerHTML = html;

      container.querySelectorAll('.btn-copy-clip').forEach(btn => {
        btn.addEventListener('click', () => {
          navigator.clipboard.writeText(btn.dataset.text).then(() => {
            showToast('📋 Copied snippet to clipboard!');
          });
        });
      });
    })
    .catch(() => {
      container.innerHTML = '<div class="rec-empty-card">Unable to load clipboard vault.</div>';
    });
}

function stashRabbitHole() {
  showToast('🐇 Sweeping divergent tabs into Curiosity Drawer...');
  fetch('/api/copilot/stash-rabbithole', { method: 'POST' })
    .then(res => res.json())
    .then(data => {
      if (data.success) {
        if (data.stashed_count > 0) {
          showToast(`🎉 Swept ${data.stashed_count} tangent tabs into ${data.drawer_name}!`);
        } else {
          showToast('No tangent/divergent tabs detected. Clean working state! ✨');
        }
        loadResumeAnchor();
        loadStashedSessions();
      } else {
        showToast(`Notice: ${data.error || 'Failed to stash rabbit hole'}`);
      }
    })
    .catch(err => showToast(`Stash error: ${err}`));
}

function loadExecutiveBriefing() {
  fetch('/api/knowledge/briefing')
    .then(res => res.json())
    .then(brief => {
      const hl = document.getElementById('briefing-headline');
      const sub = document.getElementById('briefing-summary');
      const arch = document.getElementById('briefing-archetype');
      const peak = document.getElementById('briefing-peak');
      const cap = document.getElementById('briefing-cap');

      if (hl) hl.textContent = brief.headline;
      if (sub) sub.textContent = brief.yesterday_summary;
      if (arch) arch.textContent = brief.primary_archetype;
      if (peak) peak.textContent = brief.peak_focus_window;
      if (cap) cap.textContent = `${brief.dynamic_working_memory_cap} tabs (personalized)`;
    })
    .catch(() => {});
}

function loadEveningClosure() {
  fetch('/api/knowledge/closure')
    .then(res => res.json())
    .then(closure => {
      const banner = document.getElementById('closure-banner');
      if (!closure) {
        if (banner) banner.style.display = 'none';
        return;
      }
      if (banner) banner.style.display = 'flex';
      
      const hl = document.getElementById('closure-headline');
      const badge = document.getElementById('closure-badge');
      const flow = document.getElementById('closure-flow');
      const reclaimed = document.getElementById('closure-reclaimed');

      if (hl) hl.textContent = closure.headline || 'End of day summary';
      if (badge) badge.textContent = closure.archetype || 'Balanced';
      if (flow) flow.textContent = closure.flow_sessions ? `${closure.flow_sessions} sessions` : '0';
      if (reclaimed) reclaimed.textContent = closure.total_memory_reclaimed_mb ? `${closure.total_memory_reclaimed_mb.toFixed(1)} MB` : '0 MB';
    })
    .catch(() => {
      const banner = document.getElementById('closure-banner');
      if (banner) banner.style.display = 'none';
    });
}

function loadTabAnalytics() {
  fetch('/api/knowledge/tab-analytics')
    .then(res => res.json())
    .then(data => {
      const gRatio = document.getElementById('tab-graveyard-ratio');
      const medShelf = document.getElementById('tab-median-shelf');
      const decayTau = document.getElementById('tab-decay-tau');
      const mQuick = document.getElementById('mort-quick');
      const mActive = document.getElementById('mort-active');
      const mAnchor = document.getElementById('mort-anchor');

      if (gRatio) gRatio.textContent = `${Math.round(data.graveyard_ratio * 100)}%`;
      if (medShelf) medShelf.textContent = `${data.median_shelf_life_mins} m`;
      if (decayTau) decayTau.textContent = `${data.context_decay_threshold_mins} m`;

      if (mQuick) mQuick.textContent = `${data.mortality_curve.ephemeral_quick_lookups_pct}%`;
      if (mActive) mActive.textContent = `${data.mortality_curve.active_working_tasks_pct}%`;
      if (mAnchor) mAnchor.textContent = `${data.mortality_curve.persistent_anchor_tabs_pct}%`;

      const anchorList = document.getElementById('anchor-domains-list');
      if (anchorList && data.anchor_domains && data.anchor_domains.length > 0) {
        anchorList.innerHTML = data.anchor_domains.slice(0, 8).map(d =>
          `<span class="domain-pill" title="${d.observation_count} tabs, avg ${d.avg_dwell_mins}m dwell">⚓ ${d.domain}</span>`
        ).join('');
      }

      const ephemList = document.getElementById('ephemeral-domains-list');
      if (ephemList && data.ephemeral_domains && data.ephemeral_domains.length > 0) {
        ephemList.innerHTML = data.ephemeral_domains.slice(0, 8).map(d =>
          `<span class="domain-pill" title="${d.observation_count} tabs, avg ${d.avg_shelf_life_mins}m shelf">⚡ ${d.domain}</span>`
        ).join('');
      }
    })
    .catch(() => {});
}

function loadExplorerWorkspaces() {
  fetch('/api/knowledge/explorer-analytics')
    .then(res => res.json())
    .then(data => {
      const container = document.getElementById('explorer-workspaces-list');
      if (!container) return;

      const active = data.active_open || [];
      if (active.length === 0) {
        container.innerHTML = '<div class="rec-empty-card">No active File Explorer windows or tabs detected.</div>';
        return;
      }

      container.innerHTML = active.map(f => {
        const dwellMins = Math.round(f.active_dwell_secs / 60);
        const forgottenBadge = f.is_forgotten_loop
          ? '<span class="badge badge-warning">Idle &gt;90m</span>'
          : '<span class="badge badge-success">Active</span>';

        return `
          <div class="explorer-card">
            <div class="explorer-card-left">
              <span class="explorer-icon">📁</span>
              <div class="explorer-info">
                <div class="explorer-title">
                  <span>${f.project_name}</span>
                  ${forgottenBadge}
                </div>
                <div class="explorer-path" title="${f.folder_path}">${f.folder_path}</div>
              </div>
            </div>
            <div class="explorer-card-right">
              <span class="explorer-dwell-badge">⏱️ ${dwellMins}m focused</span>
            </div>
          </div>
        `;
      }).join('');
    })
    .catch(() => {});
}

document.addEventListener('DOMContentLoaded', () => {
  const btnRefreshTabs = document.getElementById('btn-refresh-tab-analytics');
  if (btnRefreshTabs) {
    btnRefreshTabs.addEventListener('click', () => {
      loadTabAnalytics();
      showToast('🔄 Tab lifecycle & mortality analytics refreshed');
    });
  }

  const btnRefreshExplorer = document.getElementById('btn-refresh-explorer');
  if (btnRefreshExplorer) {
    btnRefreshExplorer.addEventListener('click', () => {
      loadExplorerWorkspaces();
      showToast('🔄 File Explorer workspaces refreshed');
    });
  }
});


