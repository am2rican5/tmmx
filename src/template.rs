use std::fs;
use std::path::PathBuf;

use color_eyre::eyre::{Result, eyre};
use serde::{Deserialize, Serialize};

use crate::tmux;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SplitType {
    Full,
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaneTemplate {
    pub cwd: String,
    pub split: SplitType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowTemplate {
    pub name: String,
    pub cwd: String,
    pub panes: Vec<PaneTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTemplate {
    pub template: TemplateMeta,
    pub windows: Vec<WindowTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateMeta {
    pub name: String,
    #[serde(default)]
    pub description: String,
}

pub fn template_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("tmx")
        .join("templates")
}

pub fn load_all_templates() -> Vec<SessionTemplate> {
    let dir = template_dir();
    if !dir.exists() {
        return Vec::new();
    }
    let mut templates = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "toml") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(t) = toml::from_str::<SessionTemplate>(&content) {
                        templates.push(t);
                    }
                }
            }
        }
    }
    templates.sort_by(|a, b| a.template.name.cmp(&b.template.name));
    templates
}

pub fn save_template(template: &SessionTemplate) -> Result<()> {
    let dir = template_dir();
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.toml", template.template.name));
    let content = toml::to_string_pretty(template)?;
    fs::write(path, content)?;
    Ok(())
}

pub fn delete_template(name: &str) -> Result<()> {
    let path = template_dir().join(format!("{}.toml", name));
    if path.exists() {
        fs::remove_file(path)?;
        Ok(())
    } else {
        Err(eyre!("Template '{}' not found", name))
    }
}

pub fn template_exists(name: &str) -> bool {
    template_dir().join(format!("{}.toml", name)).exists()
}

pub fn capture_session_as_template(session_name: &str) -> Result<SessionTemplate> {
    let windows = tmux::list_windows(session_name)?;
    let mut window_templates = Vec::new();

    for win in &windows {
        let panes = tmux::list_panes(session_name, win.index)?;
        let mut pane_templates = Vec::new();

        for (i, pane) in panes.iter().enumerate() {
            let split = if i == 0 {
                SplitType::Full
            } else if i % 2 == 1 {
                SplitType::Horizontal
            } else {
                SplitType::Vertical
            };
            pane_templates.push(PaneTemplate {
                cwd: pane.cwd.clone(),
                split,
            });
        }

        let cwd = panes.first().map(|p| p.cwd.clone()).unwrap_or_default();
        window_templates.push(WindowTemplate {
            name: win.name.clone(),
            cwd,
            panes: pane_templates,
        });
    }

    Ok(SessionTemplate {
        template: TemplateMeta {
            name: session_name.to_string(),
            description: String::new(),
        },
        windows: window_templates,
    })
}

pub fn launch_template(template: &SessionTemplate, session_name: &str) -> Result<()> {
    if template.windows.is_empty() {
        return Err(eyre!("Template has no windows"));
    }

    let first_win = &template.windows[0];
    // Create session with first window
    tmux::new_session_with_cwd(session_name, &first_win.cwd)?;
    tmux::rename_window(session_name, 0, &first_win.name)?;

    // Create additional panes in first window
    for pane in first_win.panes.iter().skip(1) {
        match pane.split {
            SplitType::Horizontal => {
                tmux::split_window_in_dir(session_name, 0, "-v", &pane.cwd)?;
            }
            SplitType::Vertical => {
                tmux::split_window_in_dir(session_name, 0, "-h", &pane.cwd)?;
            }
            SplitType::Full => {}
        }
    }

    // Create remaining windows
    for (wi, win) in template.windows.iter().enumerate().skip(1) {
        tmux::new_window_with_cwd(session_name, &win.name, &win.cwd)?;
        let win_idx = wi as u32;

        for pane in win.panes.iter().skip(1) {
            match pane.split {
                SplitType::Horizontal => {
                    tmux::split_window_in_dir(session_name, win_idx, "-v", &pane.cwd)?;
                }
                SplitType::Vertical => {
                    tmux::split_window_in_dir(session_name, win_idx, "-h", &pane.cwd)?;
                }
                SplitType::Full => {}
            }
        }
    }

    Ok(())
}
