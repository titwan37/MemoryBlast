use std::mem::size_of;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ,
};

#[repr(C)]
struct UnicodeString {
    length: u16,
    maximum_length: u16,
    buffer: *mut u16,
}

#[link(name = "ntdll")]
extern "system" {
    fn NtQueryInformationProcess(
        process_handle: HANDLE,
        process_information_class: u32,
        process_information: *mut std::ffi::c_void,
        process_information_length: u32,
        return_length: *mut u32,
    ) -> i32;
}

const PROCESS_COMMAND_LINE_INFORMATION: u32 = 60;

/// Attempts to retrieve the full command-line of a process given its PID.
pub fn get_process_command_line(pid: u32) -> Option<String> {
    // Try opening process with query information and vm read
    let desired_access = PROCESS_QUERY_INFORMATION | PROCESS_VM_READ;
    let handle = unsafe { OpenProcess(desired_access, false, pid) }
        .or_else(|_| unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ, false, pid) })
        .or_else(|_| unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) })
        .ok()?;

    let cmdline = query_command_line_from_handle(handle);
    unsafe { let _ = CloseHandle(handle); };
    cmdline
}

fn query_command_line_from_handle(handle: HANDLE) -> Option<String> {
    let mut return_length = 0u32;
    // Initial probe to get needed buffer size
    let _ = unsafe {
        NtQueryInformationProcess(
            handle,
            PROCESS_COMMAND_LINE_INFORMATION,
            std::ptr::null_mut(),
            0,
            &mut return_length,
        )
    };

    if return_length == 0 {
        return_length = 4096;
    }

    let mut buffer: Vec<u8> = vec![0u8; return_length as usize + 256];
    let status = unsafe {
        NtQueryInformationProcess(
            handle,
            PROCESS_COMMAND_LINE_INFORMATION,
            buffer.as_mut_ptr() as *mut std::ffi::c_void,
            buffer.len() as u32,
            &mut return_length,
        )
    };

    if status < 0 || return_length < size_of::<UnicodeString>() as u32 {
        return None;
    }

    unsafe {
        let unicode_str = &*(buffer.as_ptr() as *const UnicodeString);
        let char_count = (unicode_str.length / 2) as usize;
        if char_count == 0 {
            return None;
        }

        // On Windows 8.1 / 10 / 11, the characters are located immediately after the UNICODE_STRING header
        // or unicode_str.buffer points to them within the allocated buffer.
        let chars_ptr = if !unicode_str.buffer.is_null()
            && (unicode_str.buffer as usize) >= (buffer.as_ptr() as usize)
            && (unicode_str.buffer as usize) < (buffer.as_ptr() as usize + buffer.len())
        {
            unicode_str.buffer
        } else {
            buffer.as_ptr().add(size_of::<UnicodeString>()) as *const u16
        };

        let slice = std::slice::from_raw_parts(chars_ptr, char_count);
        Some(String::from_utf16_lossy(slice))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChromiumProcessType {
    BrowserCore,
    Renderer,
    Gpu,
    Utility(String),
    Crashpad,
    Extension(String),
    Other(String),
}

impl ChromiumProcessType {
    pub fn display_name(&self) -> &str {
        match self {
            ChromiumProcessType::BrowserCore => "Browser Core",
            ChromiumProcessType::Renderer => "Renderer / Tab",
            ChromiumProcessType::Gpu => "GPU Process",
            ChromiumProcessType::Utility(sub) => sub.as_str(),
            ChromiumProcessType::Crashpad => "Crashpad Handler",
            ChromiumProcessType::Extension(_) => "Extension",
            ChromiumProcessType::Other(s) => s.as_str(),
        }
    }
}

/// Parses Chromium command line to determine process type and debugging port if configured.
pub fn parse_chromium_command_line(cmdline: &str) -> (ChromiumProcessType, Option<u16>) {
    let mut proc_type = ChromiumProcessType::BrowserCore;
    let mut debug_port = None;

    // Search for --type=
    for arg in cmdline.split_whitespace() {
        if let Some(stripped) = arg.strip_prefix("--type=") {
            let val = stripped.trim_matches('"');
            match val {
                "renderer" => {
                    proc_type = ChromiumProcessType::Renderer;
                }
                "gpu-process" => {
                    proc_type = ChromiumProcessType::Gpu;
                }
                "utility" => {
                    proc_type = ChromiumProcessType::Utility("Utility Service".to_string());
                }
                "crashpad-handler" => {
                    proc_type = ChromiumProcessType::Crashpad;
                }
                other => {
                    proc_type = ChromiumProcessType::Other(format!("Type: {other}"));
                }
            }
        }

        // Sub-types for utility
        if let Some(sub) = arg.strip_prefix("--utility-sub-type=") {
            let sub_clean = sub.trim_matches('"');
            let friendly = if sub_clean.contains("network.mojom.NetworkService") {
                "Network Service"
            } else if sub_clean.contains("audio.mojom.AudioService") {
                "Audio Service"
            } else if sub_clean.contains("storage.mojom.StorageService") {
                "Storage Service"
            } else if sub_clean.contains("video_capture") {
                "Video Capture Service"
            } else {
                sub_clean
            };
            proc_type = ChromiumProcessType::Utility(friendly.to_string());
        }

        // Extension process detection
        if arg.contains("--extension-process") {
            proc_type = ChromiumProcessType::Extension("Web Extension".to_string());
        }

        // Remote debugging port detection (e.g. --remote-debugging-port=9222)
        if let Some(port_str) = arg.strip_prefix("--remote-debugging-port=") {
            let clean = port_str.trim_matches('"');
            if let Ok(port) = clean.parse::<u16>() {
                debug_port = Some(port);
            }
        } else if arg == "--remote-debugging-port" {
            // Default CDP port is 9222
            debug_port = Some(9222);
        }
    }

    (proc_type, debug_port)
}
