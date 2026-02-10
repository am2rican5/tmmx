use std::process::Command;

use color_eyre::eyre::{Result, eyre};

use crate::model::{TmuxPane, TmuxSession, TmuxWindow};

const FIELD_SEP: &str = "|||";

fn run_tmux(args: &[&str]) -> Result<String> {
    let output = Command::new("tmux").args(args).output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(eyre!("{}", stderr))
    }
}

fn run_tmux_allow_empty(args: &[&str]) -> Result<String> {
    let output = Command::new("tmux").args(args).output()?;
    if output.status.success() || output.status.code() == Some(1) {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(eyre!("{}", stderr))
    }
}

pub fn is_tmux_running() -> bool {
    Command::new("tmux")
        .args(["list-sessions"])
        .output()
        .map(|o| o.status.success() || o.status.code() == Some(1))
        .unwrap_or(false)
}

pub fn is_inside_tmux() -> bool {
    std::env::var("TMUX")
        .map(|v| !v.is_empty())
        .unwrap_or(false)
}

pub fn list_sessions() -> Result<Vec<TmuxSession>> {
    let format = [
        "#{session_name}",
        "#{session_id}",
        "#{session_windows}",
        "#{session_attached}",
        "#{session_created}",
    ]
    .join(FIELD_SEP);

    let output = run_tmux_allow_empty(&["list-sessions", "-F", &format])?;
    let mut sessions = Vec::new();

    for line in output.lines() {
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split(FIELD_SEP).collect();
        if fields.len() < 5 {
            continue;
        }
        sessions.push(TmuxSession {
            name: fields[0].to_string(),
            id: fields[1].to_string(),
            windows: fields[2].parse().unwrap_or(0),
            attached: fields[3] != "0",
            created: fields[4].parse().unwrap_or(0),
        });
    }

    Ok(sessions)
}

pub fn list_windows(session: &str) -> Result<Vec<TmuxWindow>> {
    let format = [
        "#{window_name}",
        "#{window_index}",
        "#{window_id}",
        "#{window_active}",
        "#{window_panes}",
    ]
    .join(FIELD_SEP);

    let output = run_tmux(&["list-windows", "-t", session, "-F", &format])?;
    let mut windows = Vec::new();

    for line in output.lines() {
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split(FIELD_SEP).collect();
        if fields.len() < 5 {
            continue;
        }
        windows.push(TmuxWindow {
            name: fields[0].to_string(),
            index: fields[1].parse().unwrap_or(0),
            id: fields[2].to_string(),
            active: fields[3] != "0",
            panes: fields[4].parse().unwrap_or(0),
        });
    }

    Ok(windows)
}

pub fn list_panes(session: &str, window_index: u32) -> Result<Vec<TmuxPane>> {
    let target = format!("{}:{}", session, window_index);
    let format = [
        "#{pane_id}",
        "#{pane_index}",
        "#{pane_active}",
        "#{pane_current_command}",
        "#{pane_width}",
        "#{pane_height}",
        "#{pane_current_path}",
    ]
    .join(FIELD_SEP);

    let output = run_tmux(&["list-panes", "-t", &target, "-F", &format])?;
    let mut panes = Vec::new();

    for line in output.lines() {
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split(FIELD_SEP).collect();
        if fields.len() < 7 {
            continue;
        }
        panes.push(TmuxPane {
            id: fields[0].to_string(),
            index: fields[1].parse().unwrap_or(0),
            active: fields[2] != "0",
            command: fields[3].to_string(),
            width: fields[4].parse().unwrap_or(0),
            height: fields[5].parse().unwrap_or(0),
            cwd: fields[6].to_string(),
        });
    }

    Ok(panes)
}

pub fn capture_pane(pane_id: &str) -> Result<String> {
    run_tmux(&["capture-pane", "-t", pane_id, "-p"])
}

pub fn new_session(name: &str) -> Result<()> {
    run_tmux(&["new-session", "-d", "-s", name])?;
    Ok(())
}

pub fn kill_session(name: &str) -> Result<()> {
    run_tmux(&["kill-session", "-t", name])?;
    Ok(())
}

pub fn rename_session(old: &str, new: &str) -> Result<()> {
    run_tmux(&["rename-session", "-t", old, new])?;
    Ok(())
}

pub fn new_window(session: &str, name: Option<&str>) -> Result<()> {
    match name {
        Some(n) if !n.is_empty() => run_tmux(&["new-window", "-t", session, "-n", n])?,
        _ => run_tmux(&["new-window", "-t", session])?,
    };
    Ok(())
}

pub fn kill_window(session: &str, window_index: u32) -> Result<()> {
    let target = format!("{}:{}", session, window_index);
    run_tmux(&["kill-window", "-t", &target])?;
    Ok(())
}

pub fn rename_window(session: &str, window_index: u32, new_name: &str) -> Result<()> {
    let target = format!("{}:{}", session, window_index);
    run_tmux(&["rename-window", "-t", &target, new_name])?;
    Ok(())
}

pub fn split_window_horizontal(session: &str, window_index: u32, pane_id: &str) -> Result<()> {
    let _ = (session, window_index);
    run_tmux(&["split-window", "-v", "-t", pane_id])?;
    Ok(())
}

pub fn split_window_vertical(session: &str, window_index: u32, pane_id: &str) -> Result<()> {
    let _ = (session, window_index);
    run_tmux(&["split-window", "-h", "-t", pane_id])?;
    Ok(())
}

pub fn kill_pane(pane_id: &str) -> Result<()> {
    run_tmux(&["kill-pane", "-t", pane_id])?;
    Ok(())
}

pub fn select_pane(pane_id: &str) -> Result<()> {
    run_tmux(&["select-pane", "-t", pane_id])?;
    Ok(())
}

pub fn resize_pane_zoom(pane_id: &str) -> Result<()> {
    run_tmux(&["resize-pane", "-Z", "-t", pane_id])?;
    Ok(())
}

pub fn break_pane(pane_id: &str) -> Result<()> {
    run_tmux(&["break-pane", "-t", pane_id])?;
    Ok(())
}

pub fn switch_client(target: &str) -> Result<()> {
    run_tmux(&["switch-client", "-t", target])?;
    Ok(())
}

pub fn attach_session(target: &str) -> Result<()> {
    run_tmux(&["attach-session", "-t", target])?;
    Ok(())
}

pub fn select_window(session: &str, window_index: u32) -> Result<()> {
    let target = format!("{}:{}", session, window_index);
    run_tmux(&["select-window", "-t", &target])?;
    Ok(())
}
