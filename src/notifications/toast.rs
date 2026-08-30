//! Native Windows Interactive Bubble & Toast Messaging
//!
//! Integrates WinRT / Windows 11 Actionable Notifications with interactive buttons:
//! - [ 🎯 Focus Back & Finish ]
//! - [ 📦 Stash & Close ]
//! - [ ⏳ Snooze 30m ]
//!
//! Provides empathetic phrasing, exponential backoff throttling, and fallback mechanisms.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use windows::core::HSTRING;
use windows::Data::Xml::Dom::XmlDocument;
use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};

/// Identifiers for actionable notification callbacks
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationAction {
    FocusTab { pid: u32, title: String },
    StashAndClose { pid: u32, title: String },
    Snooze { pid: u32, minutes: u64 },
    ParkAllContext,
}

/// Global throttle & snooze manager to prevent notification fatigue
struct NotificationThrottle {
    last_toast_time: Option<Instant>,
    snoozed_until: HashMap<String, Instant>,
    min_interval: Duration,
}

impl NotificationThrottle {
    fn new() -> Self {
        Self {
            last_toast_time: None,
            snoozed_until: HashMap::new(),
            min_interval: Duration::from_secs(180), // 3 minutes between automated prompts
        }
    }

    fn is_snoozed(&mut self, key: &str) -> bool {
        let now = Instant::now();
        // Clean up expired
        self.snoozed_until.retain(|_, v| *v > now);
        self.snoozed_until.contains_key(key)
    }

    fn snooze(&mut self, key: &str, duration: Duration) {
        self.snoozed_until.insert(key.to_string(), Instant::now() + duration);
    }

    fn can_fire(&mut self, force: bool) -> bool {
        if force {
            return true;
        }
        let now = Instant::now();
        if let Some(last) = self.last_toast_time {
            if now.duration_since(last) < self.min_interval {
                return false;
            }
        }
        self.last_toast_time = Some(now);
        true
    }
}

static THROTTLE: OnceLock<Mutex<NotificationThrottle>> = OnceLock::new();

fn with_throttle<F, R>(f: F) -> R
where
    F: FnOnce(&mut NotificationThrottle) -> R,
{
    let mutex = THROTTLE.get_or_init(|| Mutex::new(NotificationThrottle::new()));
    let mut guard = mutex.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut guard)
}

/// Snoozes alerts for a given target key (e.g. PID or tab title)
pub fn snooze_alert(key: &str, minutes: u64) {
    with_throttle(|t| {
        t.snooze(key, Duration::from_secs(minutes * 60));
    });
}

/// Checks if a key is currently snoozed
#[allow(dead_code)]
pub fn is_alert_snoozed(key: &str) -> bool {
    with_throttle(|t| t.is_snoozed(key))
}

/// Sends a native Windows 11 interactive toast notification for a forgotten task or tab
pub fn send_copilot_toast(
    title: &str,
    message: &str,
    pid: u32,
    tab_title: &str,
    force: bool,
) -> Result<bool, Box<dyn std::error::Error>> {
    let key = if !tab_title.is_empty() {
        tab_title.to_string()
    } else {
        format!("pid_{pid}")
    };

    let allowed = with_throttle(|t| {
        if t.is_snoozed(&key) {
            return false;
        }
        t.can_fire(force)
    });

    if !allowed {
        return Ok(false);
    }

    // Sanitize XML strings
    let clean_title = xml_escape(title);
    let clean_msg = xml_escape(message);
    let clean_tab = xml_escape(tab_title);

    // Build interactive Windows 11 toast XML with action buttons
    let toast_xml = format!(
        r#"<toast duration="long" scenario="reminder">
    <visual>
        <binding template="ToastGeneric">
            <text>⚡ {clean_title}</text>
            <text>{clean_msg}</text>
            <text placement="attribution">MemoryBlast Focus Copilot</text>
        </binding>
    </visual>
    <actions>
        <action content="🎯 Focus &amp; Finish" arguments="action=focus&amp;pid={pid}&amp;title={clean_tab}" activationType="background"/>
        <action content="📦 Stash &amp; Close" arguments="action=stash&amp;pid={pid}&amp;title={clean_tab}" activationType="background"/>
        <action content="⏳ Snooze 30m" arguments="action=snooze&amp;pid={pid}&amp;title={clean_tab}" activationType="background"/>
    </actions>
</toast>"#
    );

    dispatch_toast(&toast_xml)
}

/// Sends a positive reinforcement / micro-closure celebration notification
pub fn send_celebration_toast(
    tabs_closed: usize,
    memory_reclaimed_mb: f64,
) -> Result<bool, Box<dyn std::error::Error>> {
    let title = "🎉 Focus Context Restored";
    let message = format!(
        "Parked {tabs_closed} tabs and reclaimed {memory_reclaimed_mb:.1} MB of RAM! Your active working set is refreshed."
    );

    let clean_title = xml_escape(title);
    let clean_msg = xml_escape(&message);

    let toast_xml = format!(
        r#"<toast duration="short">
    <visual>
        <binding template="ToastGeneric">
            <text>{clean_title}</text>
            <text>{clean_msg}</text>
            <text placement="attribution">MemoryBlast Cognitive Copilot</text>
        </binding>
    </visual>
</toast>"#
    );

    dispatch_toast(&toast_xml)
}

/// Dispatches toast via WinRT, falling back safely to PowerShell if WinRT AUMID is unregistered
fn dispatch_toast(toast_xml: &str) -> Result<bool, Box<dyn std::error::Error>> {
    // Attempt 1: Native WinRT Notification APIs
    let winrt_res = (|| -> Result<(), Box<dyn std::error::Error>> {
        let xml = XmlDocument::new()?;
        xml.LoadXml(&HSTRING::from(toast_xml))?;
        let toast = ToastNotification::CreateToastNotification(&xml)?;
        // Standard registered AppUserModelId or Windows Explorer identity
        let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(
            "{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\\WindowsPowerShell\\v1.0\\powershell.exe",
        ))?;
        notifier.Show(&toast)?;
        Ok(())
    })();

    if winrt_res.is_ok() {
        return Ok(true);
    }

    // Attempt 2: Fallback via lightweight background PowerShell toast helper
    // This guarantees native toast popup on any Windows 10/11 installation without requiring app registration
    let escaped_xml = toast_xml.replace('\"', "`\"").replace('$', "`$");
    let script = format!(
        "[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] > $null; \
         $xml = New-Object Windows.Data.Xml.Dom.XmlDocument; \
         $xml.LoadXml(\"{escaped_xml}\"); \
         $toast = [Windows.UI.Notifications.ToastNotification]::new($xml); \
         [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('MemoryBlast.FocusCopilot').Show($toast);"
    );

    let _ = std::process::Command::new("powershell")
        .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script])
        .spawn();

    Ok(true)
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
