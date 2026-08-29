use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::mem::{size_of, zeroed};
use std::time::Instant;
use windows::Win32::Foundation::{CloseHandle, FILETIME};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::ProcessStatus::{
    K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX,
};
use windows::Win32::System::SystemInformation::{GetSystemInfo, SYSTEM_INFO};
use windows::Win32::System::Threading::{
    GetProcessTimes, OpenProcess, TerminateProcess, PROCESS_QUERY_INFORMATION,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE, PROCESS_VM_READ,
};

use crate::chromium::{
    get_chromium_browser_name, get_process_command_line,
    parse_chromium_command_line, ChromiumProcessType,
};

#[derive(Debug, Clone, Serialize)]
pub struct ProcessItem {
    pub pid: u32,
    pub ppid: u32,
    pub name: String,
    pub display_title: String,
    pub proc_type_label: String,
    pub is_chromium: bool,
    pub browser_family: Option<String>,
    pub private_bytes: u64,
    pub working_set_bytes: u64,
    pub cpu_percent: f32,
    pub memory_percent: f32,
    pub children: Vec<ProcessItem>,
    pub debug_port: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortField {
    PrivateMemory,
    WorkingSet,
    Cpu,
    Pid,
    Name,
}

impl SortField {
    pub fn next(&self) -> Self {
        match self {
            SortField::PrivateMemory => SortField::WorkingSet,
            SortField::WorkingSet => SortField::Cpu,
            SortField::Cpu => SortField::Pid,
            SortField::Pid => SortField::Name,
            SortField::Name => SortField::PrivateMemory,
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            SortField::PrivateMemory => "Private RAM (Desc)",
            SortField::WorkingSet => "Working Set (Desc)",
            SortField::Cpu => "CPU % (Desc)",
            SortField::Pid => "PID (Asc)",
            SortField::Name => "Name (Asc)",
        }
    }
}

struct CpuSample {
    kernel_time: u64,
    user_time: u64,
    sample_time: Instant,
}

/// Persistent scanner cache for CPU diffs and metadata caching
pub struct ProcessScanner {
    cpu_history: HashMap<u32, CpuSample>,
    cmdline_cache: HashMap<u32, (ChromiumProcessType, Option<u16>)>,
    num_cpus: u32,
}

impl ProcessScanner {
    pub fn new() -> Self {
        let num_cpus = unsafe {
            let mut info: SYSTEM_INFO = zeroed();
            GetSystemInfo(&mut info);
            info.dwNumberOfProcessors.max(1)
        };

        Self {
            cpu_history: HashMap::new(),
            cmdline_cache: HashMap::new(),
            num_cpus,
        }
    }

    /// Takes a full snapshot of processes, calculating memory and CPU delta usage.
    pub fn scan_processes(
        &mut self,
        total_system_memory: u64,
        window_titles: &HashMap<u32, Vec<String>>,
    ) -> Vec<ProcessItem> {
        let snapshot = match unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) } {
            Ok(handle) if !handle.is_invalid() => handle,
            _ => return Vec::new(),
        };

        let mut entries = Vec::new();
        let mut entry = PROCESSENTRY32W {
            dwSize: size_of::<PROCESSENTRY32W>() as u32,
            ..unsafe { zeroed() }
        };

        if unsafe { Process32FirstW(snapshot, &mut entry) }.is_ok() {
            loop {
                let name = extract_exe_name(&entry.szExeFile);
                entries.push((entry.th32ProcessID, entry.th32ParentProcessID, name));
                if unsafe { Process32NextW(snapshot, &mut entry) }.is_err() {
                    break;
                }
            }
        }
        unsafe { let _ = CloseHandle(snapshot); };

        let now = Instant::now();
        let current_pids: HashSet<u32> = entries.iter().map(|(pid, _, _)| *pid).collect();
        // Prune stale dead processes from history
        self.cpu_history.retain(|pid, _| current_pids.contains(pid));
        self.cmdline_cache.retain(|pid, _| current_pids.contains(pid));

        let mut raw_items = Vec::with_capacity(entries.len());

        for (pid, ppid, name) in entries {
            if pid == 0 {
                continue; // Skip System Idle Process
            }

            let browser_family = get_chromium_browser_name(&name).map(|s| s.to_string());
            let is_chromium = browser_family.is_some();

            // Query memory counters
            let (private_bytes, working_set_bytes, cpu_percent) =
                self.query_process_stats(pid, now);

            let memory_percent = if total_system_memory > 0 {
                (private_bytes as f32 / total_system_memory as f32) * 100.0
            } else {
                0.0
            };

            // Inspect Chromium details if applicable
            let mut proc_type_label = if is_chromium {
                "Chromium".to_string()
            } else {
                "Application".to_string()
            };
            let mut debug_port = None;
            let mut display_title = String::new();

            if is_chromium {
                let (proc_type, port) = if let Some(cached) = self.cmdline_cache.get(&pid) {
                    cached.clone()
                } else if let Some(cmd) = get_process_command_line(pid) {
                    let parsed = parse_chromium_command_line(&cmd);
                    self.cmdline_cache.insert(pid, parsed.clone());
                    parsed
                } else {
                    // Sandboxed Chromium renderers block PROCESS_VM_READ.
                    // Any child process of a browser is a Renderer/Tab, never BrowserCore.
                    (ChromiumProcessType::Renderer, None)
                };

                proc_type_label = proc_type.display_name().to_string();
                debug_port = port;

                // Match window titles if available
                if let Some(titles) = window_titles.get(&pid) {
                    if let Some(first) = titles.first() {
                        display_title = first.clone();
                    }
                }
            } else if let Some(titles) = window_titles.get(&pid) {
                if let Some(first) = titles.first() {
                    display_title = first.clone();
                }
            }

            raw_items.push(ProcessItem {
                pid,
                ppid,
                name,
                display_title,
                proc_type_label,
                is_chromium,
                browser_family,
                private_bytes,
                working_set_bytes,
                cpu_percent,
                memory_percent,
                children: Vec::new(),
                debug_port,
            });
        }

        // Group Chromium processes into tree structure
        group_chromium_trees(raw_items)
    }

    fn query_process_stats(&mut self, pid: u32, now: Instant) -> (u64, u64, f32) {
        let handle = match unsafe {
            OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid)
        } {
            Ok(h) => h,
            Err(_) => {
                match unsafe {
                    OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)
                } {
                    Ok(h) => h,
                    Err(_) => return (0, 0, 0.0),
                }
            }
        };

        // Query memory
        let mut private_bytes = 0u64;
        let mut working_set = 0u64;

        let mut pmc: PROCESS_MEMORY_COUNTERS_EX = unsafe { zeroed() };
        let pmc_size = size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
        if unsafe {
            K32GetProcessMemoryInfo(
                handle,
                &mut pmc as *mut _ as *mut _,
                pmc_size,
            )
        }
        .as_bool()
        {
            private_bytes = pmc.PrivateUsage as u64;
            working_set = pmc.WorkingSetSize as u64;
        }

        // Query CPU times
        let mut creation = FILETIME::default();
        let mut exit = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();

        let mut cpu_percent = 0.0f32;

        if unsafe { GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user) }
            .is_ok()
        {
            let k_time = filetime_to_u64(&kernel);
            let u_time = filetime_to_u64(&user);

            if let Some(prev) = self.cpu_history.get_mut(&pid) {
                let delta_time_ns = now.duration_since(prev.sample_time).as_nanos();
                // 100ns units in FILETIME to nanoseconds: multiply by 100
                let delta_proc_ns = (k_time.saturating_sub(prev.kernel_time)
                    + u_time.saturating_sub(prev.user_time))
                    * 100;

                if delta_time_ns > 0 {
                    let raw_cpu = (delta_proc_ns as f64 / delta_time_ns as f64)
                        / (self.num_cpus as f64)
                        * 100.0;
                    cpu_percent = (raw_cpu as f32).clamp(0.0, 100.0);
                }

                prev.kernel_time = k_time;
                prev.user_time = u_time;
                prev.sample_time = now;
            } else {
                self.cpu_history.insert(
                    pid,
                    CpuSample {
                        kernel_time: k_time,
                        user_time: u_time,
                        sample_time: now,
                    },
                );
            }
        }

        unsafe { let _ = CloseHandle(handle); };

        (private_bytes, working_set, cpu_percent)
    }
}

/// Groups child Chromium processes under their respective parent root browser processes.
fn group_chromium_trees(items: Vec<ProcessItem>) -> Vec<ProcessItem> {
    let mut map: HashMap<u32, ProcessItem> = HashMap::with_capacity(items.len());
    let mut root_pids = Vec::new();
    let mut ppid_map: HashMap<u32, u32> = HashMap::new();

    for item in items {
        ppid_map.insert(item.pid, item.ppid);
        map.insert(item.pid, item.clone());
    }

    // Identify roots vs children
    let mut children_by_parent: HashMap<u32, Vec<u32>> = HashMap::new();

    for (pid, item) in &map {
        if item.is_chromium {
            let parent_pid = item.ppid;
            // If parent is also in map and is the same browser family, treat as child
            let parent_is_same_browser = map
                .get(&parent_pid)
                .map(|p| p.browser_family == item.browser_family)
                .unwrap_or(false);

            if parent_is_same_browser {
                children_by_parent.entry(parent_pid).or_default().push(*pid);
                continue;
            }
        }
        root_pids.push(*pid);
    }

    // Sort roots deterministically by PID
    root_pids.sort_unstable();

    // Attach children recursively
    let mut result = Vec::with_capacity(root_pids.len());
    for root_pid in root_pids {
        if let Some(mut root) = map.remove(&root_pid) {
            // Root of a Chromium tree is always Browser Core
            if root.is_chromium && root.proc_type_label == "Renderer / Tab" {
                root.proc_type_label = "Browser Core".to_string();
            }
            attach_children(&mut root, &mut map, &children_by_parent);
            result.push(root);
        }
    }

    result
}

fn attach_children(
    parent: &mut ProcessItem,
    map: &mut HashMap<u32, ProcessItem>,
    children_by_parent: &HashMap<u32, Vec<u32>>,
) {
    if let Some(child_pids) = children_by_parent.get(&parent.pid) {
        let mut sorted_pids = child_pids.clone();
        sorted_pids.sort_unstable(); // Deterministic stable PID ordering

        for child_pid in sorted_pids {
            if let Some(mut child) = map.remove(&child_pid) {
                attach_children(&mut child, map, children_by_parent);
                parent.children.push(child);
            }
        }
    }
}

fn filetime_to_u64(ft: &FILETIME) -> u64 {
    ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64)
}

fn extract_exe_name(raw: &[u16]) -> String {
    let len = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
    String::from_utf16_lossy(&raw[..len])
}

/// Attempts to terminate a process by PID.
pub fn kill_process(pid: u32) -> Result<(), String> {
    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE, false, pid)
            .map_err(|e| format!("Failed to open process: {e}"))?;

        let res = TerminateProcess(handle, 1);
        let _ = CloseHandle(handle);

        res.map_err(|e| format!("Failed to terminate process: {e}"))
    }
}
