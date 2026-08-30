let currentSnapshot = null;
let expandedPids = new Set();
let selectedPid = null;
let selectedPpid = null;
let selectedName = null;
let selectedTitle = null;
let selectedPort = null;
let currentSort = { col: 'memory', desc: true };
let searchQuery = '';
let activeView = 'processes';

// 60-second history buffer (each sample ~500ms -> up to 120 samples)
const memoryHistory = [];
const MAX_HISTORY = 120;

// Initialize on DOM load
document.addEventListener('DOMContentLoaded', () => {
  setupNavigation();
  setupSearch();
  setupSorting();
  setupColumnResizing();
  setupActions();
  setupModal();
  initCanvas();

  // 1. Immediately fetch initial snapshot so the table renders in 5ms
  fetch('/api/snapshot')
    .then(res => res.json())
    .then(snapshot => {
      if (snapshot && snapshot.processes) {
        onSnapshotReceived(snapshot);
      }
    })
    .catch(() => {});

  // 2. Connect to live real-time SSE stream
  connectEventSource();
});

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
    renderProcessTable();
  });

  clearBtn.addEventListener('click', () => {
    input.value = '';
    searchQuery = '';
    clearBtn.classList.add('hidden');
    renderProcessTable();
  });
}

// Column Sorting & Resizing State
let preventSortUntil = 0;

const DEFAULT_COL_WIDTHS = {
  name: 380,
  pid: 80,
  type: 180,
  cpu: 90,
  memory: 170,
  ws: 130
};

const MIN_COL_WIDTHS = {
  name: 140,
  pid: 55,
  type: 90,
  cpu: 65,
  memory: 100,
  ws: 75
};

function setupSorting() {
  const headers = document.querySelectorAll('th.sortable');
  headers.forEach(th => {
    th.addEventListener('click', (e) => {
      // Prevent sorting if clicking directly on a resizer handle or right after dragging
      if (e.target.classList.contains('col-resizer') || Date.now() < preventSortUntil) {
        return;
      }

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

      renderProcessTable();
    });
  });
}

// Column Width Resizing
function setupColumnResizing() {
  const table = document.getElementById('process-table');
  if (!table) return;

  // Restore previously saved column widths from localStorage
  applySavedColumnWidths();

  const resizers = table.querySelectorAll('.col-resizer');
  resizers.forEach(resizer => {
    const th = resizer.parentElement;
    const colKey = th.dataset.col;

    // Double-click on resizer resets column to default width
    resizer.addEventListener('dblclick', (e) => {
      e.preventDefault();
      e.stopPropagation();
      preventSortUntil = Date.now() + 300;
      if (DEFAULT_COL_WIDTHS[colKey]) {
        th.style.width = `${DEFAULT_COL_WIDTHS[colKey]}px`;
        saveColumnWidths();
      }
    });

    resizer.addEventListener('mousedown', (e) => {
      e.preventDefault();
      e.stopPropagation();

      const startX = e.pageX;
      const startWidth = th.offsetWidth;
      const minWidth = MIN_COL_WIDTHS[colKey] || 50;

      resizer.classList.add('resizing');
      document.body.classList.add('is-resizing');

      let hasMoved = false;

      function onMouseMove(moveEvent) {
        hasMoved = true;
        const deltaX = moveEvent.pageX - startX;
        const newWidth = Math.max(minWidth, startWidth + deltaX);
        th.style.width = `${newWidth}px`;
      }

      function onMouseUp() {
        document.removeEventListener('mousemove', onMouseMove);
        document.removeEventListener('mouseup', onMouseUp);

        resizer.classList.remove('resizing');
        document.body.classList.remove('is-resizing');

        if (hasMoved) {
          preventSortUntil = Date.now() + 300;
          saveColumnWidths();
        }
      }

      document.addEventListener('mousemove', onMouseMove);
      document.addEventListener('mouseup', onMouseUp);
    });
  });
}

function applySavedColumnWidths() {
  try {
    const saved = JSON.parse(localStorage.getItem('memoryblast_col_widths'));
    if (saved && typeof saved === 'object') {
      document.querySelectorAll('#process-table thead th[data-col]').forEach(th => {
        const key = th.dataset.col;
        if (saved[key] && typeof saved[key] === 'number') {
          th.style.width = `${saved[key]}px`;
        }
      });
    }
  } catch (_) {}
}

function saveColumnWidths() {
  try {
    const widths = {};
    document.querySelectorAll('#process-table thead th[data-col]').forEach(th => {
      const key = th.dataset.col;
      const w = parseInt(th.style.width, 10) || th.offsetWidth;
      if (key && w) {
        widths[key] = w;
      }
    });
    localStorage.setItem('memoryblast_col_widths', JSON.stringify(widths));
  } catch (_) {}
}

// End task and refresh actions
function setupActions() {
  const endTaskBtn = document.getElementById('btn-end-task');
  endTaskBtn.addEventListener('click', () => {
    if (selectedPid) {
      openKillModal(selectedPid, selectedName);
    }
  });

  const trimBtn = document.getElementById('btn-trim-ram');
  if (trimBtn) {
    trimBtn.addEventListener('click', () => {
      if (selectedPid) {
        trimWorkingSet(selectedPid, selectedName);
      }
    });
  }

  const exportParquetBtn = document.getElementById('btn-export-parquet');
  if (exportParquetBtn) {
    exportParquetBtn.addEventListener('click', () => {
      exportParquet();
    });
  }

  const exportParquetInlineBtn = document.getElementById('btn-export-parquet-inline');
  if (exportParquetInlineBtn) {
    exportParquetInlineBtn.addEventListener('click', () => {
      exportParquet();
    });
  }

  const detectTabsBtn = document.getElementById('btn-detect-tabs');
  if (detectTabsBtn) {
    detectTabsBtn.addEventListener('click', () => {
      fetch('/api/detect-tabs', { method: 'POST' }).catch(() => {});
      showToast('🔍 Retriggering tab detection...');
    });
  }

  const focusBtn = document.getElementById('btn-focus-tab');
  if (focusBtn) {
    focusBtn.addEventListener('click', () => {
      if (selectedPid) {
        focusTab(selectedPid, selectedPpid, selectedTitle, selectedPort);
      }
    });
  }

  const refreshBtn = document.getElementById('btn-refresh');
  refreshBtn.addEventListener('click', () => {
    fetch('/api/refresh', { method: 'POST' }).catch(() => {});
    showToast('Refreshing process metrics...');
  });

  const parkBtn = document.getElementById('btn-park-context');
  if (parkBtn) {
    parkBtn.addEventListener('click', () => parkCurrentContext());
  }

  const heroParkBtn = document.getElementById('btn-copilot-park-hero');
  if (heroParkBtn) {
    heroParkBtn.addEventListener('click', () => parkCurrentContext());
  }

  const refreshStashesBtn = document.getElementById('btn-refresh-stashes');
  if (refreshStashesBtn) {
    refreshStashesBtn.addEventListener('click', () => loadStashedSessions());
  }

  const refreshClipBtn = document.getElementById('btn-refresh-clipboard');
  if (refreshClipBtn) {
    refreshClipBtn.addEventListener('click', () => loadClipboardVault());
  }

  const stashRabbitBtn = document.getElementById('btn-stash-rabbithole');
  if (stashRabbitBtn) {
    stashRabbitBtn.addEventListener('click', () => stashRabbitHole());
  }

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

function focusTab(pid, ppid, title, port) {
  showToast(`Locating and focusing tab...`);
  fetch('/api/focus', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ pid, ppid, title: title || '', port: port || null })
  })
  .then(res => res.json())
  .then(data => {
    if (data.success) {
      showToast(`🎯 Focused: ${title || selectedName}`);
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
  document.getElementById('modal-proc-name').textContent = name || 'Process';
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
      showToast(`Terminated PID ${pid} [${name}]`);
      selectedPid = null;
      selectedName = null;
      document.getElementById('btn-end-task').disabled = true;
    } else {
      showToast(`Error: ${data.error || 'Failed to terminate'}`);
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
    // Retry polling if stream drops
    setTimeout(connectEventSource, 2000);
  };
}

function onSnapshotReceived(snapshot) {
  currentSnapshot = snapshot;

  // Apply Exocortex Cognitive State theme
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

  // Update top banner
  updateTopBanners(snapshot);

  // Update process table if in process view
  if (activeView === 'processes') {
    renderProcessTable();
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

function updateTopBanners(snapshot) {
  const mem = snapshot.mem_stats;
  const adminBadge = document.getElementById('admin-badge');
  if (mem.is_admin) {
    adminBadge.textContent = '[ADMIN]';
    adminBadge.className = 'badge badge-admin';
  } else {
    adminBadge.textContent = '[STANDARD USER]';
    adminBadge.className = 'badge badge-user';
  }

  const used = formatBytes(mem.used_physical_bytes);
  const total = formatBytes(mem.total_physical_bytes);
  const pct = mem.memory_load_percent;
  document.getElementById('banner-ram-used').textContent = `${used} / ${total} (${pct}%)`;

  const commitUsed = formatBytes(mem.committed_bytes);
  const commitTot = formatBytes(mem.total_page_file_bytes);
  document.getElementById('banner-commit').textContent = `${commitUsed} / ${commitTot}`;

  const paged = formatBytes(mem.paged_pool_bytes);
  const nonpaged = formatBytes(mem.non_paged_pool_bytes);
  document.getElementById('banner-pools').textContent = `${paged} / ${nonpaged}`;
}

// Render Table
function renderProcessTable() {
  if (!currentSnapshot) return;

  const tbody = document.getElementById('process-table-body');
  let roots = [...currentSnapshot.processes];

  // Calculate highest memory usage for heatmap normalization
  let maxMemory = 1;
  for (const r of roots) {
    const groupMem = r.private_bytes + (r.children || []).reduce((acc, c) => acc + c.private_bytes, 0);
    if (groupMem > maxMemory) maxMemory = groupMem;
  }

  // Sort
  sortItems(roots, currentSort.col, currentSort.desc);

  let html = '';
  let matchCount = 0;

  for (const root of roots) {
    const isExpanded = expandedPids.has(root.pid);
    const hasChildren = root.children && root.children.length > 0;
    const groupMem = root.private_bytes + (root.children || []).reduce((acc, c) => acc + c.private_bytes, 0);
    const groupWs = root.working_set_bytes + (root.children || []).reduce((acc, c) => acc + c.working_set_bytes, 0);
    const groupCpu = root.cpu_percent + (root.children || []).reduce((acc, c) => acc + c.cpu_percent, 0);

    const matchesRoot = itemMatchesQuery(root, searchQuery);
    const matchingChildren = (root.children || []).filter(c => itemMatchesQuery(c, searchQuery));
    const hasMatches = !searchQuery || matchesRoot || matchingChildren.length > 0;

    if (!hasMatches) continue;
    matchCount++;

    const isSelected = selectedPid === root.pid;
    const heatWidth = Math.min(100, Math.max(4, (groupMem / maxMemory) * 100));

    let toggleHtml = '';
    if (hasChildren) {
      toggleHtml = `<span class="tree-toggle" data-toggle="${root.pid}">${isExpanded ? '▼' : '▶'}</span>`;
    } else {
      toggleHtml = `<span class="tree-indent"></span>`;
    }

    const browserIcon = root.is_chromium ? '🌐 ' : '📦 ';
    const titleSuffix = root.display_title ? ` — <span style="color:#60cdff">${escapeHtml(root.display_title)}</span>` : '';
    const childBadge = hasChildren ? ` <span style="color:#757575">(${root.children.length})</span>` : '';

    html += `
      <tr class="root-row ${isSelected ? 'selected' : ''}" data-pid="${root.pid}" data-ppid="${root.ppid}" data-name="${escapeHtml(root.name)}" data-title="${escapeHtml(root.display_title)}" data-port="${root.debug_port || ''}">
        <td class="col-name">
          <div class="tree-node">
            ${toggleHtml}
            <span class="tree-icon">${browserIcon}</span>
            <span class="tree-title">${escapeHtml(root.name)}</span>
            ${childBadge}
            ${titleSuffix}
          </div>
        </td>
        <td class="col-pid">${root.pid}</td>
        <td class="col-type">${escapeHtml(root.proc_type_label)}</td>
        <td class="col-cpu" style="${groupCpu > 5 ? 'color:#fce100;font-weight:600' : ''}">${groupCpu.toFixed(1)}%</td>
        <td class="col-memory">
          <div class="heat-cell">
            <div class="heat-bar" style="width: ${heatWidth}%;"></div>
            <span>${formatBytes(groupMem)}</span>
          </div>
        </td>
        <td class="col-ws">${formatBytes(groupWs)}</td>
      </tr>
    `;

    // Render children if expanded OR if search is active
    if ((isExpanded || searchQuery) && hasChildren) {
      const displayChildren = searchQuery ? matchingChildren : root.children;
      sortItems(displayChildren, currentSort.col, currentSort.desc);

      for (let i = 0; i < displayChildren.length; i++) {
        const child = displayChildren[i];
        const isLast = i === displayChildren.length - 1;
        const isChildSelected = selectedPid === child.pid;
        const childHeat = Math.min(100, Math.max(3, (child.private_bytes / maxMemory) * 100));

        const icon = getChildIcon(child.proc_type_label);
        const childTitle = child.display_title || child.name;

        html += `
          <tr class="child-row ${isChildSelected ? 'selected' : ''}" data-pid="${child.pid}" data-ppid="${root.pid}" data-name="${escapeHtml(child.name)}" data-title="${escapeHtml(child.display_title)}" data-port="${root.debug_port || ''}">
            <td class="col-name" style="padding-left: 36px;">
              <div class="tree-node">
                <span class="tree-indent" style="color:#555">${isLast ? '└─' : '├─'}</span>
                <span class="tree-icon">${icon}</span>
                <span class="tree-title" style="color:#e0e0e0">${escapeHtml(childTitle)}</span>
              </div>
            </td>
            <td class="col-pid">${child.pid}</td>
            <td class="col-type" style="color:#8c8c8c">${escapeHtml(child.proc_type_label)}</td>
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
  document.getElementById('banner-proc-count').textContent = matchCount;

  // Bind clicks
  tbody.querySelectorAll('tr[data-pid]').forEach(row => {
    row.addEventListener('click', (e) => {
      if (e.target.classList.contains('tree-toggle')) {
        const pid = parseInt(e.target.dataset.toggle, 10);
        if (expandedPids.has(pid)) {
          expandedPids.delete(pid);
        } else {
          expandedPids.add(pid);
        }
        renderProcessTable();
        return;
      }

      tbody.querySelectorAll('tr').forEach(r => r.classList.remove('selected'));
      row.classList.add('selected');
      selectedPid = parseInt(row.dataset.pid, 10);
      selectedPpid = parseInt(row.dataset.ppid, 10) || 0;
      selectedName = row.dataset.name;
      selectedTitle = row.dataset.title || '';
      selectedPort = row.dataset.port ? parseInt(row.dataset.port, 10) : null;

      document.getElementById('btn-end-task').disabled = false;
      const trimBtn = document.getElementById('btn-trim-ram');
      if (trimBtn) trimBtn.disabled = false;
      const focusBtn = document.getElementById('btn-focus-tab');
      if (focusBtn) {
        focusBtn.disabled = false;
      }
    });
  });
}

function getChildIcon(role) {
  if (role.includes('GPU')) return '⚡';
  if (role.includes('Network')) return '🌐';
  if (role.includes('Audio')) return '🔊';
  if (role.includes('Extension')) return '🧩';
  if (role.includes('Crashpad')) return '🛡️';
  return '📄';
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
      valA = a.name.toLowerCase();
      valB = b.name.toLowerCase();
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

  // Left card
  document.getElementById('mini-mem-percent').textContent = `${mem.memory_load_percent}%`;
  document.getElementById('mini-mem-val').textContent = `${inUseGb} / ${totalGb} GB (${mem.memory_load_percent}%)`;
  document.getElementById('perf-total-capacity').textContent = `${totalGb} GB Host Memory`;

  // Grid
  document.getElementById('perf-in-use').textContent = `${inUseGb} GB`;
  document.getElementById('perf-avail').textContent = `${availGb} GB`;
  document.getElementById('perf-commit').textContent = `${commitUsedGb} / ${commitTotGb} GB`;
  document.getElementById('perf-cached').textContent = `${cachedGb} GB`;
  document.getElementById('perf-paged').textContent = `${pagedGb} GB`;
  document.getElementById('perf-nonpaged').textContent = `${nonpagedGb} GB`;
  document.getElementById('chart-max-label').textContent = `${totalGb} GB`;

  // Composition bar
  const totalBytes = mem.total_physical_bytes || 1;
  const inUsePct = (mem.used_physical_bytes / totalBytes) * 100;
  const cachedPct = Math.min(100 - inUsePct, (mem.system_cache_bytes / totalBytes) * 100);
  const availPct = Math.max(0, 100 - inUsePct - cachedPct);

  document.getElementById('comp-in-use').style.width = `${inUsePct}%`;
  document.getElementById('comp-cached').style.width = `${cachedPct}%`;
  document.getElementById('comp-avail').style.width = `${availPct}%`;
}

// 60-Second Real-Time Canvas Charts
let mainCanvas, mainCtx, sparkCanvas, sparkCtx;

function initCanvas() {
  mainCanvas = document.getElementById('memory-chart');
  mainCtx = mainCanvas.getContext('2d');
  sparkCanvas = document.getElementById('mini-mem-sparkline');
  sparkCtx = sparkCanvas.getContext('2d');

  function resize() {
    const rect = mainCanvas.getBoundingClientRect();
    mainCanvas.width = rect.width * window.devicePixelRatio;
    mainCanvas.height = rect.height * window.devicePixelRatio;
    drawCharts();
  }

  window.addEventListener('resize', resize);
  resize();
}

function drawCharts() {
  if (!mainCanvas || !mainCtx || memoryHistory.length === 0) return;

  const w = mainCanvas.width;
  const h = mainCanvas.height;
  mainCtx.clearRect(0, 0, w, h);

  // Background Grid Lines
  mainCtx.strokeStyle = '#282828';
  mainCtx.lineWidth = 1;

  for (let i = 1; i <= 4; i++) {
    const y = (h / 4) * i;
    mainCtx.beginPath();
    mainCtx.moveTo(0, y);
    mainCtx.lineTo(w, y);
    mainCtx.stroke();
  }

  // Draw Area Curve
  const points = [];
  const count = memoryHistory.length;
  for (let i = 0; i < count; i++) {
    const x = (i / (MAX_HISTORY - 1)) * w;
    const pct = memoryHistory[i].percent / 100;
    const y = h - (pct * h);
    points.push({ x, y });
  }

  if (points.length > 1) {
    // Fill gradient
    const grad = mainCtx.createLinearGradient(0, 0, 0, h);
    grad.addColorStop(0, 'rgba(0, 120, 212, 0.45)');
    grad.addColorStop(1, 'rgba(0, 120, 212, 0.02)');

    mainCtx.fillStyle = grad;
    mainCtx.beginPath();
    mainCtx.moveTo(points[0].x, h);
    for (const p of points) {
      mainCtx.lineTo(p.x, p.y);
    }
    mainCtx.lineTo(points[points.length - 1].x, h);
    mainCtx.closePath();
    mainCtx.fill();

    // Stroke line
    mainCtx.strokeStyle = '#60cdff';
    mainCtx.lineWidth = 2 * window.devicePixelRatio;
    mainCtx.beginPath();
    mainCtx.moveTo(points[0].x, points[0].y);
    for (const p of points) {
      mainCtx.lineTo(p.x, p.y);
    }
    mainCtx.stroke();

    // Latest pulse dot
    const lastP = points[points.length - 1];
    mainCtx.fillStyle = '#ffffff';
    mainCtx.beginPath();
    mainCtx.arc(lastP.x, lastP.y, 3.5 * window.devicePixelRatio, 0, Math.PI * 2);
    mainCtx.fill();
  }

  // Mini Sparkline in sidebar
  if (sparkCanvas && sparkCtx) {
    const sw = sparkCanvas.width;
    const sh = sparkCanvas.height;
    sparkCtx.clearRect(0, 0, sw, sh);

    if (points.length > 1) {
      sparkCtx.strokeStyle = '#60cdff';
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
  toast.textContent = msg;
  toast.classList.remove('hidden');
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => toast.classList.add('hidden'), 3500);
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
      showToast(`✨ Successfully trimmed working set for ${name || 'PID ' + pid}`);
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


