use crate::models::{Pane, Session, Window};
use anyhow::{anyhow, bail, Context, Result};
use std::process::Command;

const FIELD_SEPARATOR: char = '\u{1f}';
const SESSION_FORMAT: &str =
    "#{session_id}\u{1f}#{session_name}\u{1f}#{session_windows}\u{1f}#{session_created_string}";
const WINDOW_FORMAT: &str =
    "#{window_id}\u{1f}#{window_name}\u{1f}#{window_active}\u{1f}#{window_layout}";
const PANE_FORMAT: &str = "#{pane_id}\u{1f}#{pane_width}\u{1f}#{pane_height}\u{1f}#{pane_current_path}\u{1f}#{pane_current_command}\u{1f}#{pane_active}";

pub fn run_tmux(args: &[&str]) -> Result<String> {
    let output = Command::new("tmux")
        .args(args)
        .output()
        .with_context(|| format!("failed to start tmux with args: {}", args.join(" ")))?;

    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        Err(anyhow!(
            "tmux command `{}` failed with status {}",
            args.join(" "),
            output.status
        ))
    } else {
        Err(anyhow!(stderr))
    }
}

fn run_tmux_unit(args: &[&str]) -> Result<()> {
    run_tmux(args).map(|_| ())
}

pub fn get_sessions() -> Result<Vec<Session>> {
    match run_tmux(&["list-sessions", "-F", SESSION_FORMAT]) {
        Ok(raw) => parse_sessions(&raw),
        Err(err) if is_no_server_error(&err.to_string()) => Ok(Vec::new()),
        Err(err) => Err(err.context("could not list tmux sessions")),
    }
}

pub fn get_windows(session_id: &str) -> Result<Vec<Window>> {
    let raw = run_tmux(&["list-windows", "-t", session_id, "-F", WINDOW_FORMAT])
        .with_context(|| format!("could not list windows for session `{session_id}`"))?;

    parse_windows(&raw)
}

pub fn get_panes(window_id: &str) -> Result<Vec<Pane>> {
    let raw = run_tmux(&["list-panes", "-t", window_id, "-F", PANE_FORMAT])
        .with_context(|| format!("could not list panes for window `{window_id}`"))?;

    parse_panes(&raw)
}

pub fn create_session(name: &str) -> Result<()> {
    run_tmux_unit(&["new-session", "-d", "-s", name])
        .with_context(|| format!("could not create session `{name}`"))
}

pub fn rename_session(old_name: &str, new_name: &str) -> Result<()> {
    run_tmux_unit(&["rename-session", "-t", old_name, new_name])
        .with_context(|| format!("could not rename session `{old_name}` to `{new_name}`"))
}

pub fn kill_session(name: &str) -> Result<()> {
    run_tmux_unit(&["kill-session", "-t", name])
        .with_context(|| format!("could not delete session `{name}`"))
}

pub fn create_window(session_id: &str, name: &str) -> Result<()> {
    run_tmux_unit(&["new-window", "-t", session_id, "-n", name])
        .with_context(|| format!("could not create window `{name}`"))
}

pub fn rename_window(window_id: &str, new_name: &str) -> Result<()> {
    run_tmux_unit(&["rename-window", "-t", window_id, new_name])
        .with_context(|| format!("could not rename window `{window_id}` to `{new_name}`"))
}

pub fn kill_window(window_id: &str) -> Result<()> {
    run_tmux_unit(&["kill-window", "-t", window_id])
        .with_context(|| format!("could not delete window `{window_id}`"))
}

pub fn select_window(window_id: &str) -> Result<()> {
    run_tmux_unit(&["select-window", "-t", window_id])
        .with_context(|| format!("could not select window `{window_id}`"))
}

pub fn create_pane(pane_id: &str) -> Result<()> {
    run_tmux_unit(&["split-window", "-t", pane_id])
        .with_context(|| format!("could not split pane `{pane_id}`"))
}

pub fn kill_pane(pane_id: &str) -> Result<()> {
    run_tmux_unit(&["kill-pane", "-t", pane_id])
        .with_context(|| format!("could not delete pane `{pane_id}`"))
}

pub fn select_pane(pane_id: &str) -> Result<()> {
    run_tmux_unit(&["select-pane", "-t", pane_id])
        .with_context(|| format!("could not select pane `{pane_id}`"))
}

fn parse_sessions(raw: &str) -> Result<Vec<Session>> {
    raw.lines()
        .filter(|line| !line.trim().is_empty())
        .map(parse_session_line)
        .collect()
}

fn parse_windows(raw: &str) -> Result<Vec<Window>> {
    raw.lines()
        .filter(|line| !line.trim().is_empty())
        .map(parse_window_line)
        .collect()
}

fn parse_panes(raw: &str) -> Result<Vec<Pane>> {
    raw.lines()
        .filter(|line| !line.trim().is_empty())
        .map(parse_pane_line)
        .collect()
}

fn parse_session_line(line: &str) -> Result<Session> {
    let parts = split_fields(line, 4, "session")?;

    Ok(Session {
        id: parts[0].to_string(),
        name: parts[1].to_string(),
        window_count: parse_usize(parts[2], "session window count")?,
        created: parts[3].to_string(),
    })
}

fn parse_window_line(line: &str) -> Result<Window> {
    let parts = split_fields(line, 4, "window")?;

    Ok(Window {
        id: parts[0].to_string(),
        name: parts[1].to_string(),
        active: parse_flag(parts[2], "window active")?,
        layout: parts[3].to_string(),
    })
}

fn parse_pane_line(line: &str) -> Result<Pane> {
    let parts = split_fields(line, 6, "pane")?;

    Ok(Pane {
        id: parts[0].to_string(),
        width: parse_u16(parts[1], "pane width")?,
        height: parse_u16(parts[2], "pane height")?,
        current_path: parts[3].to_string(),
        current_command: parts[4].to_string(),
        active: parse_flag(parts[5], "pane active")?,
    })
}

fn split_fields<'a>(line: &'a str, expected: usize, item_kind: &str) -> Result<Vec<&'a str>> {
    let parts: Vec<&str> = line.split(FIELD_SEPARATOR).collect();
    if parts.len() != expected {
        bail!(
            "invalid {item_kind} output: expected {expected} fields but got {}",
            parts.len()
        );
    }
    Ok(parts)
}

fn parse_flag(value: &str, field_name: &str) -> Result<bool> {
    match value {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => bail!("invalid {field_name} flag `{value}`"),
    }
}

fn parse_usize(value: &str, field_name: &str) -> Result<usize> {
    value
        .parse()
        .with_context(|| format!("invalid {field_name} `{value}`"))
}

fn parse_u16(value: &str, field_name: &str) -> Result<u16> {
    value
        .parse()
        .with_context(|| format!("invalid {field_name} `{value}`"))
}

fn is_no_server_error(message: &str) -> bool {
    let normalized = message.to_ascii_lowercase();
    normalized.contains("no server running")
        || normalized.contains("failed to connect to server")
        || normalized.contains("connection refused")
        || (normalized.contains("error connecting to")
            && normalized.contains("no such file or directory"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sessions_with_strong_types() {
        let raw = "%0\u{1f}dev\u{1f}3\u{1f}Sun Apr 19 12:00:00 2026";

        let sessions = parse_sessions(raw).expect("sessions should parse");

        assert_eq!(
            sessions,
            vec![Session {
                id: "%0".to_string(),
                name: "dev".to_string(),
                window_count: 3,
                created: "Sun Apr 19 12:00:00 2026".to_string(),
            }]
        );
    }

    #[test]
    fn parses_windows_and_panes() {
        let windows = parse_windows("@1\u{1f}editor\u{1f}1\u{1f}main-vertical")
            .expect("windows should parse");
        let panes = parse_panes("%1\u{1f}120\u{1f}30\u{1f}/tmp\u{1f}zsh\u{1f}0")
            .expect("panes should parse");

        assert_eq!(
            windows,
            vec![Window {
                id: "@1".to_string(),
                name: "editor".to_string(),
                active: true,
                layout: "main-vertical".to_string(),
            }]
        );
        assert_eq!(
            panes,
            vec![Pane {
                id: "%1".to_string(),
                width: 120,
                height: 30,
                current_path: "/tmp".to_string(),
                current_command: "zsh".to_string(),
                active: false,
            }]
        );
    }

    #[test]
    fn rejects_malformed_tmux_output() {
        let err = parse_session_line("%0\u{1f}dev").expect_err("line should be rejected");
        assert!(err.to_string().contains("expected 4 fields"));
    }

    #[test]
    fn detects_no_server_messages() {
        assert!(is_no_server_error(
            "no server running on /tmp/tmux-501/default"
        ));
        assert!(is_no_server_error("failed to connect to server"));
        assert!(is_no_server_error(
            "error connecting to /tmp/tmux-501/default (No such file or directory)"
        ));
        assert!(!is_no_server_error("permission denied"));
    }
}
