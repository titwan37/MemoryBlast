use std::path::PathBuf;
use std::process::Command;

/// Launches a standalone native Windows desktop application window pointing to the given URL.
pub fn launch_desktop_window(url: &str) -> Result<(), String> {
    // 1. Check for msedge in standard paths
    let edge_paths = [
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
    ];

    for path in edge_paths {
        if PathBuf::from(path).exists() {
            if Command::new(path)
                .arg(format!("--app={url}"))
                .arg("--window-size=1260,820")
                .spawn()
                .is_ok()
            {
                return Ok(());
            }
        }
    }

    // Try `msedge` directly in PATH
    if Command::new("msedge")
        .arg(format!("--app={url}"))
        .arg("--window-size=1260,820")
        .spawn()
        .is_ok()
    {
        return Ok(());
    }

    // 2. Try Google Chrome in standard paths
    let chrome_paths = [
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
    ];

    for path in chrome_paths {
        if PathBuf::from(path).exists() {
            if Command::new(path)
                .arg(format!("--app={url}"))
                .arg("--window-size=1260,820")
                .spawn()
                .is_ok()
            {
                return Ok(());
            }
        }
    }

    // 3. Fallback: default browser
    let _ = Command::new("cmd")
        .args(["/c", "start", url])
        .spawn()
        .map_err(|e| format!("Failed to open browser: {e}"))?;

    Ok(())
}
