CREATE TABLE IF NOT EXISTS explorer_window_groups (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    original_hwnd INTEGER NOT NULL,
    show_cmd INTEGER NOT NULL,
    rect_left INTEGER NOT NULL,
    rect_top INTEGER NOT NULL,
    rect_right INTEGER NOT NULL,
    rect_bottom INTEGER NOT NULL,
    captured_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS explorer_tabs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    window_group_id INTEGER NOT NULL REFERENCES explorer_window_groups(id) ON DELETE CASCADE,
    tab_index INTEGER NOT NULL,
    title TEXT NOT NULL,
    folder_path TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT 0
);
