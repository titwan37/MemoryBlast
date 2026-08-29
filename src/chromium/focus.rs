use std::time::Duration;
use windows::core::VARIANT;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationInvokePattern, IUIAutomationSelectionItemPattern,
    TreeScope_Descendants, UIA_ControlTypePropertyId, UIA_InvokePatternId,
    UIA_SelectionItemPatternId, UIA_TabItemControlTypeId,
};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, EnumWindows, GetClassNameW, GetForegroundWindow,
    GetWindowThreadProcessId, IsWindowVisible, SetForegroundWindow, ShowWindow, SW_RESTORE,
};

#[link(name = "user32")]
extern "system" {
    fn AttachThreadInput(idattach: u32, idattachto: u32, fattach: BOOL) -> BOOL;
    fn SetFocus(hwnd: HWND) -> HWND;
}

struct WindowFindCtx {
    pids: Vec<u32>,
    found_hwnd: Option<HWND>,
}

unsafe extern "system" fn enum_find_browser_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctx = &mut *(lparam.0 as *mut WindowFindCtx);

    if !IsWindowVisible(hwnd).as_bool() {
        return BOOL(1);
    }

    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));

    if ctx.pids.contains(&pid) {
        let mut class_buf = [0u16; 256];
        let len = GetClassNameW(hwnd, &mut class_buf);
        let class_name = String::from_utf16_lossy(&class_buf[..len as usize]);

        if class_name == "Chrome_WidgetWin_1" {
            ctx.found_hwnd = Some(hwnd);
            return BOOL(0); // Stop enumeration, found the main window!
        }
    }

    BOOL(1)
}

/// Locates the browser window for the given PID/PPID, forces it to the foreground,
/// and switches directly to the specified tab using CDP or Windows UI Automation.
pub async fn focus_chromium_tab(
    pid: u32,
    ppid: u32,
    tab_title: &str,
    debug_port: Option<u16>,
) -> Result<(), String> {
    // 1. Locate the top-level browser window HWND and force to foreground immediately
    let hwnd_raw: usize = {
        let mut search_ctx = WindowFindCtx {
            pids: vec![pid, ppid],
            found_hwnd: None,
        };

        unsafe {
            let _ = EnumWindows(
                Some(enum_find_browser_window),
                LPARAM(&mut search_ctx as *mut _ as isize),
            );
        }

        let hwnd = search_ctx
            .found_hwnd
            .ok_or_else(|| format!("Could not find browser window for PID {pid} / PPID {ppid}"))?;

        // Win32: Force browser window to foreground & restore if minimized
        force_foreground_window(hwnd);

        hwnd.0 as usize
    };

    // 2. Strategy A: Try CDP activate if debugging port is present
    if let Some(port) = debug_port {
        if activate_tab_via_cdp(port, tab_title).await.is_ok() {
            return Ok(());
        }
    }

    // 4. Strategy B: Switch tab visually via Windows UI Automation SelectionItemPattern
    let title_owned = tab_title.to_string();

    tokio::task::spawn_blocking(move || {
        let hwnd_reconstructed = HWND(hwnd_raw as *mut std::ffi::c_void);
        select_tab_via_uia(hwnd_reconstructed, &title_owned)
    })
    .await
    .map_err(|e| format!("Task join error: {e}"))?
}

/// Bypasses Windows focus-stealing restrictions using AttachThreadInput and brings window to front.
fn force_foreground_window(hwnd: HWND) {
    unsafe {
        let fg_hwnd = GetForegroundWindow();
        let fg_thread = GetWindowThreadProcessId(fg_hwnd, None);
        let target_thread = GetWindowThreadProcessId(hwnd, None);
        let cur_thread = GetCurrentThreadId();

        // Attach input threads to permit SetForegroundWindow call
        let attached_cur = AttachThreadInput(cur_thread, target_thread, BOOL(1)).as_bool();
        let attached_fg = if fg_thread != 0 && fg_thread != target_thread {
            AttachThreadInput(fg_thread, target_thread, BOOL(1)).as_bool()
        } else {
            false
        };

        // Restore window if minimized
        let _ = ShowWindow(hwnd, SW_RESTORE);
        let _ = BringWindowToTop(hwnd);
        let _ = SetForegroundWindow(hwnd);
        let _ = SetFocus(hwnd);

        // Detach input threads
        if attached_cur {
            let _ = AttachThreadInput(cur_thread, target_thread, BOOL(0));
        }
        if attached_fg {
            let _ = AttachThreadInput(fg_thread, target_thread, BOOL(0));
        }
    }
}

/// Queries Chrome DevTools Protocol /json and sends /json/activate/{id}
async fn activate_tab_via_cdp(port: u16, target_title: &str) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("http://127.0.0.1:{port}/json");
    let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
    let tabs: Vec<serde_json::Value> = resp.json().await.map_err(|e| e.to_string())?;

    let target_clean = target_title.trim().to_lowercase();

    for tab in tabs {
        let title = tab.get("title").and_then(|t| t.as_str()).unwrap_or("");
        if title.to_lowercase().contains(&target_clean) || target_clean.contains(&title.to_lowercase()) {
            if let Some(id) = tab.get("id").and_then(|i| i.as_str()) {
                let activate_url = format!("http://127.0.0.1:{port}/json/activate/{id}");
                let _ = client.get(&activate_url).send().await;
                return Ok(());
            }
        }
    }

    Err("No matching CDP target found".to_string())
}

/// Traverses UIA TabItem controls and invokes SelectionItemPattern::Select()
fn select_tab_via_uia(hwnd: HWND, target_title: &str) -> Result<(), String> {
    if target_title.is_empty() {
        return Ok(());
    }

    let search_clean = target_title.trim().to_lowercase();

    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let uia: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
            .map_err(|e| format!("UIA CoCreateInstance error: {e}"))?;

        let root_elem = uia
            .ElementFromHandle(hwnd)
            .map_err(|e| format!("ElementFromHandle error: {e}"))?;

        let tab_var = VARIANT::from(UIA_TabItemControlTypeId.0);
        let tab_cond = uia
            .CreatePropertyCondition(UIA_ControlTypePropertyId, &tab_var)
            .map_err(|e| format!("CreatePropertyCondition error: {e}"))?;

        let elements = root_elem
            .FindAll(TreeScope_Descendants, &tab_cond)
            .map_err(|e| format!("FindAll error: {e}"))?;

        let count = elements.Length().map_err(|e| format!("Length error: {e}"))?;

        for i in 0..count {
            if let Ok(elem) = elements.GetElement(i) {
                if let Ok(name_bstr) = elem.CurrentName() {
                    let tab_name = name_bstr.to_string().to_lowercase();
                    if tab_name.contains(&search_clean) || search_clean.contains(&tab_name) {
                        // Strategy 1: SelectionItemPattern::Select()
                        if let Ok(sel_pattern) =
                            elem.GetCurrentPatternAs::<IUIAutomationSelectionItemPattern>(
                                UIA_SelectionItemPatternId,
                            )
                        {
                            if sel_pattern.Select().is_ok() {
                                CoUninitialize();
                                return Ok(());
                            }
                        }

                        // Strategy 2: InvokePattern::Invoke() fallback
                        if let Ok(inv_pattern) =
                            elem.GetCurrentPatternAs::<IUIAutomationInvokePattern>(
                                UIA_InvokePatternId,
                            )
                        {
                            if inv_pattern.Invoke().is_ok() {
                                CoUninitialize();
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }

        CoUninitialize();
    }

    Ok(())
}
