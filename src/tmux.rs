use std::process::Command;
use crate::models::{Session, Window, Pane};

/// Executes a tmux command with the given arguments.
/// Returns the stdout as a trimmed String if successful, or None if it fails.
pub fn run_tmux(args: &[&str]) -> Option<String> {
    // We rely on 'tmux' being in the system PATH, which is standard on Linux/macOS.
    let output = Command::new("tmux").args(args).output().ok()?;
    
    if !output.status.success() { 
        return None; 
    }
    
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

// --- READ OPERATIONS ---

/// Fetches all active sessions.
/// Uses a custom format string to parse output reliably.
pub fn get_sessions() -> Vec<Session> {
    let raw = match run_tmux(&["list-sessions", "-F", "#{session_id}|#{session_name}|#{session_windows}|#{session_created_string}"]) {
        Some(s) => s,
        None => return Vec::new(),
    };
    
    raw.lines().filter(|l| !l.is_empty()).map(|line| {
        let parts: Vec<&str> = line.split('|').collect();
        Session {
            id: parts.get(0).unwrap_or(&"").to_string(),
            name: parts.get(1).unwrap_or(&"").to_string(),
            count: parts.get(2).unwrap_or(&"0").to_string(),
            created: parts.get(3).unwrap_or(&"").to_string(),
        }
    }).collect()
}

/// Fetches windows for a specific session ID.
pub fn get_windows(session_id: &str) -> Vec<Window> {
    let raw = match run_tmux(&["list-windows", "-t", session_id, "-F", "#{window_id}|#{window_name}|#{window_active}|#{window_layout}"]) {
        Some(s) => s,
        None => return Vec::new(),
    };

    raw.lines().filter(|l| !l.is_empty()).map(|line| {
        let parts: Vec<&str> = line.split('|').collect();
        Window {
            id: parts.get(0).unwrap_or(&"").to_string(),
            name: parts.get(1).unwrap_or(&"").to_string(),
            active: parts.get(2).unwrap_or(&"0") == &"1",
            layout: parts.get(3).unwrap_or(&"").to_string(),
        }
    }).collect()
}

/// Fetches panes for a specific window ID.
pub fn get_panes(window_id: &str) -> Vec<Pane> {
    let raw = match run_tmux(&["list-panes", "-t", window_id, "-F", "#{pane_id}|#{pane_width}|#{pane_height}|#{pane_current_path}|#{pane_current_command}|#{pane_active}"]) {
        Some(s) => s,
        None => return Vec::new(),
    };

    raw.lines().filter(|l| !l.is_empty()).map(|line| {
        let parts: Vec<&str> = line.split('|').collect();
        Pane {
            id: parts.get(0).unwrap_or(&"").to_string(),
            width: parts.get(1).unwrap_or(&"").to_string(),
            height: parts.get(2).unwrap_or(&"").to_string(),
            current_path: parts.get(3).unwrap_or(&"").to_string(),
            current_command: parts.get(4).unwrap_or(&"").to_string(),
            active: parts.get(5).unwrap_or(&"0") == &"1",
        }
    }).collect()
}

// --- WRITE OPERATIONS ---

pub fn create_session(name: &str) {
    run_tmux(&["new-session", "-d", "-s", name]);
}

pub fn rename_session(old_name: &str, new_name: &str) {
    run_tmux(&["rename-session", "-t", old_name, new_name]);
}

pub fn kill_session(name: &str) {
    run_tmux(&["kill-session", "-t", name]);
}

pub fn create_window(session_id: &str, name: &str) {
    run_tmux(&["new-window", "-t", session_id, "-n", name]);
}

pub fn rename_window(window_id: &str, new_name: &str) {
    run_tmux(&["rename-window", "-t", window_id, new_name]);
}

pub fn kill_window(window_id: &str) {
    run_tmux(&["kill-window", "-t", window_id]);
}

/// Sets the active window for a session. 
/// Used to ensure the user lands on the correct window when attaching.
pub fn select_window(window_id: &str) {
    run_tmux(&["select-window", "-t", window_id]);
}

pub fn create_pane(window_id: &str) {
    run_tmux(&["split-window", "-t", window_id]);
}

pub fn kill_pane(pane_id: &str) {
    run_tmux(&["kill-pane", "-t", pane_id]);
}

/// Sets the active pane for a window.
/// Used to ensure the cursor is in the correct pane when attaching.
pub fn select_pane(pane_id: &str) {
    run_tmux(&["select-pane", "-t", pane_id]);
}