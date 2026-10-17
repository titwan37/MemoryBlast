// src/copilot/explorer_session.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use windows::core::{Interface, BSTR, VARIANT};
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, IDispatch, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Accessibility::*;
use windows::Win32::UI::Shell::{IShellWindows, IWebBrowser2, ShellWindows, SWC_EXPLORER};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowPlacement, SetForegroundWindow, SetWindowPlacement, SW_SHOWMAXIMIZED, SW_SHOWNORMAL,
    WINDOWPLACEMENT,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorerTabState {
    pub title: String,
    pub path: String,
    pub is_active: bool,
    pub tab_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowGeometry {
    pub show_cmd: u32,
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorerWindowGroup {
    pub hwnd: isize,
    pub geometry: WindowGeometry,
    pub tabs: Vec<ExplorerTabState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorerSnapshot {
    pub session_id: String,
    pub captured_at_unix: i64,
    pub windows: Vec<ExplorerWindowGroup>,
}

pub struct ExplorerSessionEngine;

impl ExplorerSessionEngine {
    /// Captures all open File Explorer windows, their grouped tabs, active tab state, and screen coordinates.
    pub fn capture_snapshot(session_id: &str) -> anyhow::Result<ExplorerSnapshot> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

            let shell_windows: IShellWindows = CoCreateInstance(&ShellWindows, None, CLSCTX_ALL)?;
            let count = shell_windows.Count()?;

            let mut groups: HashMap<isize, ExplorerWindowGroup> = HashMap::new();

            for i in 0..count {
                let index_var = VARIANT::from(i);
                if let Ok(disp) = shell_windows.Item(&index_var) {
                    if let Ok(browser) = disp.cast::<IWebBrowser2>() {
                        // Verify host is explorer.exe
                        let full_name = browser.FullName()?.to_string();
                        if !full_name.to_lowercase().ends_with("explorer.exe") {
                            continue;
                        }

                        let hwnd_val = browser.HWND()?.0 as isize;
                        let title = browser.LocationName()?.to_string();
                        let raw_url = browser.LocationURL()?.to_string();
                        let path = Self::normalize_shell_url(&raw_url);

                        if path.is_empty() {
                            continue;
                        }

                        let entry = groups.entry(hwnd_val).or_insert_with(|| {
                            let geometry = Self::get_window_geometry(HWND(hwnd_val as _));
                            ExplorerWindowGroup {
                                hwnd: hwnd_val,
                                geometry,
                                tabs: Vec::new(),
                            }
                        });

                        let tab_idx = entry.tabs.len();
                        entry.tabs.push(ExplorerTabState {
                            title,
                            path,
                            is_active: false,
                            tab_index: tab_idx,
                        });
                    }
                }
            }

            // Enrich active tab state via IUIAutomation on each CabinetWClass HWND
            if let Ok(uia) = CoCreateInstance::<_, IUIAutomation>(&CUIAutomation, None, CLSCTX_ALL)
            {
                for group in groups.values_mut() {
                    if let Some(active_title) =
                        Self::get_active_tab_title(&uia, HWND(group.hwnd as _))
                    {
                        for tab in &mut group.tabs {
                            if tab.title == active_title {
                                tab.is_active = true;
                                break;
                            }
                        }
                    }
                    // Default first tab as active if UIA didn't match
                    if !group.tabs.iter().any(|t| t.is_active) && !group.tabs.is_empty() {
                        group.tabs[0].is_active = true;
                    }
                }
            }

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs() as i64;

            Ok(ExplorerSnapshot {
                session_id: session_id.to_string(),
                captured_at_unix: now,
                windows: groups.into_values().collect(),
            })
        }
    }

    /// Queries the WinUI 3 TabView inside CabinetWClass to determine which tab is currently focused
    unsafe fn get_active_tab_title(uia: &IUIAutomation, hwnd: HWND) -> Option<String> {
        let root = uia.ElementFromHandle(hwnd).ok()?;
        let condition = uia
            .CreatePropertyCondition(
                UIA_ControlTypePropertyId,
                &VARIANT::from(UIA_TabItemControlTypeId.0 as i32),
            )
            .ok()?;

        let tab_items = root.FindAll(TreeScope_Descendants, &condition).ok()?;
        let length = tab_items.Length().ok()?;

        for i in 0..length {
            if let Ok(item) = tab_items.GetElement(i) {
                if let Ok(pattern) = item.GetCurrentPatternAs::<IUIAutomationSelectionItemPattern>(
                    UIA_SelectionItemPatternId,
                ) {
                    if pattern.CurrentIsSelected().ok()?.as_bool() {
                        return Some(item.CurrentName().ok()?.to_string());
                    }
                }
            }
        }
        None
    }

    unsafe fn get_window_geometry(hwnd: HWND) -> WindowGeometry {
        let mut wp = WINDOWPLACEMENT {
            length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
            ..Default::default()
        };
        let _ = GetWindowPlacement(hwnd, &mut wp);
        WindowGeometry {
            show_cmd: wp.showCmd,
            left: wp.rcNormalPosition.left,
            top: wp.rcNormalPosition.top,
            right: wp.rcNormalPosition.right,
            bottom: wp.rcNormalPosition.bottom,
        }
    }

    /// Applies saved WINDOWPLACEMENT coordinates to a restored Explorer window
    pub unsafe fn apply_window_geometry(hwnd: HWND, geom: &WindowGeometry) {
        let mut wp = WINDOWPLACEMENT {
            length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
            ..Default::default()
        };
        let _ = GetWindowPlacement(hwnd, &mut wp);
        wp.showCmd = geom.show_cmd;
        wp.rcNormalPosition = RECT {
            left: geom.left,
            top: geom.top,
            right: geom.right,
            bottom: geom.bottom,
        };
        let _ = SetWindowPlacement(hwnd, &wp);
    }

    fn normalize_shell_url(url: &str) -> String {
        if let Some(stripped) = url.strip_prefix("file:///") {
            urlencoding::decode(stripped)
                .unwrap_or_default()
                .replace('/', "\\")
        } else {
            url.to_string()
        }
    }
}
