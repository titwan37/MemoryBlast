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

// Column Sorting
function setupSorting() {
  const headers = document.querySelectorAll('th.sortable');
  headers.forEach(th => {
    th.addEventListener('click', () => {
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

// End task and refresh actions
function setupActions() {
  const endTaskBtn = document.getElementById('btn-end-task');
  endTaskBtn.addEventListener('click', () => {
    if (selectedPid) {
      openKillModal(selectedPid, selectedName);
    }
  });

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
