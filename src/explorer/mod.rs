//! Windows File Explorer Window & Tab Lifecycle Management
//!
//! Exposes routines to resolve open Explorer folder paths, track lifecycles,
//! and consolidate workspace patterns over time.

pub mod watcher;

pub use watcher::{
    clean_address_bar_text, decode_file_uri, extract_project_name, global_explorer_tracker,
    resolve_open_explorer_folders, ExplorerFolderInfo, ExplorerLifecycleTracker,
};
