use std::collections::HashSet;
use windows::core::VARIANT;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, TreeScope_Descendants, UIA_ControlTypePropertyId,
    UIA_DocumentControlTypeId, UIA_TabItemControlTypeId,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
    IsWindowVisible,
};

struct WindowSearchCtx {
    allowed_pids: HashSet<u32>,
    hwnds: Vec<HWND>,
    window_titles: Vec<String>,
}

unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctx = &mut *(lparam.0 as *mut WindowSearchCtx);

    if !IsWindowVisible(hwnd).as_bool() {
        return BOOL(1);
    }

    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));

    if ctx.allowed_pids.contains(&pid) {
        let mut class_buf = [0u16; 256];
        let len = GetClassNameW(hwnd, &mut class_buf);
        let class_name = String::from_utf16_lossy(&class_buf[..len as usize]);

        // Match Chromium main windows
        if class_name.contains("Chrome_WidgetWin_1") {
            ctx.hwnds.push(hwnd);

            // Read the top-level window title immediately as an instant, infallible fallback
            let length = GetWindowTextLengthW(hwnd);
            if length > 0 {
                let mut buf = vec![0u16; (length + 1) as usize];
                let read_len = GetWindowTextW(hwnd, &mut buf);
                if read_len > 0 {
                    let raw = String::from_utf16_lossy(&buf[..read_len as usize]);
                    let cleaned = clean_tab_name(&raw);
                    if !cleaned.is_empty() && !ctx.window_titles.contains(&cleaned) {
                        ctx.window_titles.push(cleaned);
                    }
                }
            }
        }
    }

    BOOL(1)
}

/// Discovers all open tab titles for a Chromium browser family using Windows UI Automation
/// combined with Win32 window title fallback.
pub fn get_browser_tab_titles_via_uia(pids: &[u32]) -> Vec<String> {
    if pids.is_empty() {
        return Vec::new();
    }

    let allowed_pids: HashSet<u32> = pids.iter().cloned().collect();

    // 1. Find all top-level windows for this browser process group
    let mut search_ctx = WindowSearchCtx {
        allowed_pids,
        hwnds: Vec::new(),
        window_titles: Vec::new(),
    };

    unsafe {
        let _ = EnumWindows(
            Some(enum_windows_proc),
            LPARAM(&mut search_ctx as *mut _ as isize),
        );
    }

    let mut titles: Vec<String> = search_ctx.window_titles;
    let mut seen: HashSet<String> = titles.iter().cloned().collect();

    if search_ctx.hwnds.is_empty() {
        return titles;
    }

    // 2. Query Windows UI Automation for background tabs in the Tab Strip
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        if let Ok(uia) = CoCreateInstance::<_, IUIAutomation>(&CUIAutomation, None, CLSCTX_INPROC_SERVER) {
            for hwnd in search_ctx.hwnds {
                if let Ok(root_elem) = uia.ElementFromHandle(hwnd) {
                    // Strategy A: Find all TabItem controls in the Tab Strip
                    let tab_var = VARIANT::from(UIA_TabItemControlTypeId.0);
                    if let Ok(tab_cond) = uia.CreatePropertyCondition(UIA_ControlTypePropertyId, &tab_var) {
                        if let Ok(elements) = root_elem.FindAll(TreeScope_Descendants, &tab_cond) {
                            if let Ok(count) = elements.Length() {
                                for i in 0..count {
                                    if let Ok(elem) = elements.GetElement(i) {
                                        if let Ok(name_bstr) = elem.CurrentName() {
                                            let raw_name = name_bstr.to_string();
                                            let cleaned = clean_tab_name(&raw_name);
                                            if !cleaned.is_empty() && seen.insert(cleaned.clone()) {
                                                titles.push(cleaned);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Strategy B: Find Document controls if tab items are not exposed
                    if titles.is_empty() {
                        let doc_var = VARIANT::from(UIA_DocumentControlTypeId.0);
                        if let Ok(doc_cond) = uia.CreatePropertyCondition(UIA_ControlTypePropertyId, &doc_var) {
                            if let Ok(elements) = root_elem.FindAll(TreeScope_Descendants, &doc_cond) {
                                if let Ok(count) = elements.Length() {
                                    for i in 0..count {
                                        if let Ok(elem) = elements.GetElement(i) {
                                            if let Ok(name_bstr) = elem.CurrentName() {
                                                let raw_name = name_bstr.to_string();
                                                let cleaned = clean_tab_name(&raw_name);
                                                if !cleaned.is_empty() && seen.insert(cleaned.clone()) {
                                                    titles.push(cleaned);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        CoUninitialize();
    }

    titles
}

/// Cleans raw accessibility tab labels (e.g. removes "Close Tab", "New Tab", and suffixes).
pub fn clean_tab_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty()
        || trimmed == "New Tab"
        || trimmed == "Close"
        || trimmed == "Close Tab"
        || trimmed == "Tab Search"
        || trimmed == "Add New Tab"
        || trimmed == "Google Chrome"
        || trimmed == "Microsoft Edge"
    {
        return String::new();
    }

    // Strip browser suffixes
    let mut cleaned = trimmed;
    let suffixes = [
        " - Google Chrome",
        " - Microsoft Edge",
        " - Personal - Microsoft​ Edge",
        " - Work - Microsoft​ Edge",
        " - Brave",
        " - Arc",
        " - Opera",
        " - Vivaldi",
    ];

    for s in suffixes {
        if let Some(stripped) = cleaned.strip_suffix(s) {
            cleaned = stripped.trim();
        }
    }

    cleaned.to_string()
}
