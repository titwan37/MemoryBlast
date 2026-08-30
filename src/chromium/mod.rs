pub mod cmdline;
pub mod focus;
pub mod lifecycle;
pub mod tab_resolver;
pub mod uia_tabs;

pub use cmdline::{get_process_command_line, parse_chromium_command_line, ChromiumProcessType};
pub use focus::focus_chromium_tab;
#[allow(unused_imports)]
pub use lifecycle::{global_lifecycle_tracker, ChromiumLifecycleTracker, ClosureCause};
pub use tab_resolver::{get_chromium_browser_name, get_window_titles_by_pid, query_cdp_tabs};
pub use uia_tabs::get_browser_tab_titles_via_uia;
