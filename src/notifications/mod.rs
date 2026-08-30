#[allow(unused_imports)]
pub mod badge;
#[allow(unused_imports)]
pub mod toast;

#[allow(unused_imports)]
pub use badge::{global_badge, CopilotBadgeState, CopilotStatus};
#[allow(unused_imports)]
pub use toast::{send_celebration_toast, send_copilot_toast, snooze_alert, NotificationAction};
