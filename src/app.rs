use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::widgets::ListState;

use crate::model::{TmuxPane, TmuxSession, TmuxWindow};
use crate::tmux;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Sessions,
    Windows,
    Panes,
    Preview,
}

impl Panel {
    pub const ALL: [Panel; 4] = [Panel::Sessions, Panel::Windows, Panel::Panes, Panel::Preview];

    pub fn index(self) -> usize {
        match self {
            Panel::Sessions => 0,
            Panel::Windows => 1,
            Panel::Panes => 2,
            Panel::Preview => 3,
        }
    }

    pub fn from_index(i: usize) -> Self {
        Panel::ALL[i % Panel::ALL.len()]
    }

    pub fn label(self) -> &'static str {
        match self {
            Panel::Sessions => "Sessions",
            Panel::Windows => "Windows",
            Panel::Panes => "Panes",
            Panel::Preview => "Preview",
        }
    }

    pub fn next(self) -> Self {
        Self::from_index(self.index() + 1)
    }

    pub fn prev(self) -> Self {
        Self::from_index((self.index() + Panel::ALL.len() - 1) % Panel::ALL.len())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    TextInput,
    Confirm,
    Help,
}

#[derive(Debug, Clone)]
pub enum PendingAction {
    CreateSession,
    RenameSession(String),
    CreateWindow,
    RenameWindow(String, u32),
    KillSession(String),
    KillWindow(String, u32),
    KillPane(String),
}

#[derive(Debug, Clone)]
pub struct StatusMessage {
    pub text: String,
    pub is_error: bool,
    pub created: Instant,
}

pub struct App {
    pub running: bool,
    pub should_suspend: bool,
    pub suspend_target: Option<String>,

    pub focused: Panel,
    pub mode: InputMode,

    pub sessions: Vec<TmuxSession>,
    pub windows: Vec<TmuxWindow>,
    pub panes: Vec<TmuxPane>,
    pub pane_capture: String,

    pub session_state: ListState,
    pub window_state: ListState,
    pub pane_state: ListState,

    pub input_buffer: String,
    pub input_prompt: String,
    pub pending_action: Option<PendingAction>,
    pub confirm_message: String,

    pub status: Option<StatusMessage>,

    pub last_refresh: Instant,
    pub refresh_interval_secs: u64,
}

impl App {
    pub fn new() -> Self {
        let mut app = Self {
            running: true,
            should_suspend: false,
            suspend_target: None,
            focused: Panel::Sessions,
            mode: InputMode::Normal,
            sessions: Vec::new(),
            windows: Vec::new(),
            panes: Vec::new(),
            pane_capture: String::new(),
            session_state: ListState::default(),
            window_state: ListState::default(),
            pane_state: ListState::default(),
            input_buffer: String::new(),
            input_prompt: String::new(),
            pending_action: None,
            confirm_message: String::new(),
            status: None,
            last_refresh: Instant::now(),
            refresh_interval_secs: 2,
        };
        app.refresh_tmux_state();
        app
    }

    pub fn refresh_tmux_state(&mut self) {
        let prev_session = self.selected_session_name();
        let prev_window = self.selected_window_index();
        let prev_pane = self.selected_pane_id();

        self.sessions = tmux::list_sessions().unwrap_or_default();

        if !self.sessions.is_empty() {
            let idx = if let Some(ref name) = prev_session {
                self.sessions
                    .iter()
                    .position(|s| &s.name == name)
                    .unwrap_or(0)
            } else {
                0
            };
            self.session_state.select(Some(idx.min(self.sessions.len() - 1)));
        } else {
            self.session_state.select(None);
        }

        self.refresh_windows(prev_window);
        self.refresh_panes(prev_pane);
        self.refresh_preview();
        self.last_refresh = Instant::now();
    }

    fn refresh_windows(&mut self, prev_index: Option<u32>) {
        if let Some(session) = self.selected_session() {
            self.windows = tmux::list_windows(&session.name).unwrap_or_default();
            if !self.windows.is_empty() {
                let idx = if let Some(pi) = prev_index {
                    self.windows
                        .iter()
                        .position(|w| w.index == pi)
                        .unwrap_or(0)
                } else {
                    0
                };
                self.window_state.select(Some(idx.min(self.windows.len() - 1)));
            } else {
                self.window_state.select(None);
            }
        } else {
            self.windows.clear();
            self.window_state.select(None);
        }
    }

    fn refresh_panes(&mut self, prev_id: Option<String>) {
        if let (Some(session), Some(window)) = (self.selected_session(), self.selected_window()) {
            self.panes = tmux::list_panes(&session.name, window.index).unwrap_or_default();
            if !self.panes.is_empty() {
                let idx = if let Some(ref pid) = prev_id {
                    self.panes.iter().position(|p| &p.id == pid).unwrap_or(0)
                } else {
                    0
                };
                self.pane_state.select(Some(idx.min(self.panes.len() - 1)));
            } else {
                self.pane_state.select(None);
            }
        } else {
            self.panes.clear();
            self.pane_state.select(None);
        }
    }

    fn refresh_preview(&mut self) {
        if let Some(pane) = self.selected_pane() {
            self.pane_capture = tmux::capture_pane(&pane.id).unwrap_or_default();
        } else {
            self.pane_capture.clear();
        }
    }

    pub fn selected_session(&self) -> Option<TmuxSession> {
        self.session_state
            .selected()
            .and_then(|i| self.sessions.get(i).cloned())
    }

    pub fn selected_session_name(&self) -> Option<String> {
        self.selected_session().map(|s| s.name)
    }

    pub fn selected_window(&self) -> Option<TmuxWindow> {
        self.window_state
            .selected()
            .and_then(|i| self.windows.get(i).cloned())
    }

    pub fn selected_window_index(&self) -> Option<u32> {
        self.selected_window().map(|w| w.index)
    }

    pub fn selected_pane(&self) -> Option<TmuxPane> {
        self.pane_state
            .selected()
            .and_then(|i| self.panes.get(i).cloned())
    }

    pub fn selected_pane_id(&self) -> Option<String> {
        self.selected_pane().map(|p| p.id)
    }

    fn set_status(&mut self, text: String, is_error: bool) {
        self.status = Some(StatusMessage {
            text,
            is_error,
            created: Instant::now(),
        });
    }

    fn clear_stale_status(&mut self) {
        if let Some(ref s) = self.status {
            if s.created.elapsed().as_secs() >= 5 {
                self.status = None;
            }
        }
    }

    fn start_text_input(&mut self, prompt: &str, prefill: &str, action: PendingAction) {
        self.mode = InputMode::TextInput;
        self.input_prompt = prompt.to_string();
        self.input_buffer = prefill.to_string();
        self.pending_action = Some(action);
    }

    fn start_confirm(&mut self, message: &str, action: PendingAction) {
        self.mode = InputMode::Confirm;
        self.confirm_message = message.to_string();
        self.pending_action = Some(action);
    }

    fn cancel_input(&mut self) {
        self.mode = InputMode::Normal;
        self.input_buffer.clear();
        self.input_prompt.clear();
        self.confirm_message.clear();
        self.pending_action = None;
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        self.clear_stale_status();

        match self.mode {
            InputMode::Help => self.handle_help_key(key),
            InputMode::Confirm => self.handle_confirm_key(key),
            InputMode::TextInput => self.handle_text_input_key(key),
            InputMode::Normal => self.handle_normal_key(key),
        }
    }

    fn handle_help_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('?') | KeyCode::Esc => self.mode = InputMode::Normal,
            _ => {}
        }
    }

    fn handle_confirm_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Enter => {
                if let Some(action) = self.pending_action.take() {
                    self.execute_action(action);
                }
                self.cancel_input();
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                self.cancel_input();
            }
            _ => {}
        }
    }

    fn handle_text_input_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                let value = self.input_buffer.clone();
                if let Some(action) = self.pending_action.take() {
                    self.execute_text_action(action, &value);
                }
                self.cancel_input();
            }
            KeyCode::Esc => self.cancel_input(),
            KeyCode::Backspace => {
                self.input_buffer.pop();
            }
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
            }
            _ => {}
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) {
        // Global keybindings
        match key.code {
            KeyCode::Char('q') => {
                self.running = false;
                return;
            }
            KeyCode::Char('?') => {
                self.mode = InputMode::Help;
                return;
            }
            KeyCode::Char('R') => {
                self.refresh_tmux_state();
                self.set_status("Refreshed".to_string(), false);
                return;
            }
            KeyCode::Char('1') => {
                self.focused = Panel::Sessions;
                return;
            }
            KeyCode::Char('2') => {
                self.focused = Panel::Windows;
                return;
            }
            KeyCode::Char('3') => {
                self.focused = Panel::Panes;
                return;
            }
            KeyCode::Char('4') => {
                self.focused = Panel::Preview;
                return;
            }
            KeyCode::Tab => {
                self.focused = self.focused.next();
                return;
            }
            KeyCode::BackTab => {
                self.focused = self.focused.prev();
                return;
            }
            _ => {}
        }

        // Panel-specific keybindings
        match self.focused {
            Panel::Sessions => self.handle_sessions_key(key),
            Panel::Windows => self.handle_windows_key(key),
            Panel::Panes => self.handle_panes_key(key),
            Panel::Preview => self.handle_preview_key(key),
        }
    }

    fn handle_sessions_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_selection_down(&Panel::Sessions);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_selection_up(&Panel::Sessions);
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.focused = self.focused.next();
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.focused = self.focused.prev();
            }
            KeyCode::Char('n') => {
                self.start_text_input("New session name: ", "", PendingAction::CreateSession);
            }
            KeyCode::Char('r') => {
                if let Some(session) = self.selected_session() {
                    self.start_text_input(
                        "Rename session: ",
                        &session.name,
                        PendingAction::RenameSession(session.name.clone()),
                    );
                }
            }
            KeyCode::Char('d') => {
                if let Some(session) = self.selected_session() {
                    self.start_confirm(
                        &format!("Kill session '{}'? (y/n)", session.name),
                        PendingAction::KillSession(session.name.clone()),
                    );
                }
            }
            KeyCode::Enter => {
                self.switch_to_selected_session();
            }
            _ => {}
        }
    }

    fn handle_windows_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_selection_down(&Panel::Windows);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_selection_up(&Panel::Windows);
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.focused = self.focused.next();
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.focused = self.focused.prev();
            }
            KeyCode::Char('n') => {
                self.start_text_input("New window name: ", "", PendingAction::CreateWindow);
            }
            KeyCode::Char('r') => {
                if let (Some(session), Some(window)) =
                    (self.selected_session(), self.selected_window())
                {
                    self.start_text_input(
                        "Rename window: ",
                        &window.name,
                        PendingAction::RenameWindow(session.name.clone(), window.index),
                    );
                }
            }
            KeyCode::Char('d') => {
                if let (Some(session), Some(window)) =
                    (self.selected_session(), self.selected_window())
                {
                    self.start_confirm(
                        &format!("Kill window '{}:{}'? (y/n)", session.name, window.name),
                        PendingAction::KillWindow(session.name.clone(), window.index),
                    );
                }
            }
            KeyCode::Enter => {
                self.switch_to_selected_window();
            }
            _ => {}
        }
    }

    fn handle_panes_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_selection_down(&Panel::Panes);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_selection_up(&Panel::Panes);
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.focused = self.focused.next();
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.focused = self.focused.prev();
            }
            KeyCode::Char('n') => {
                self.split_pane_vertical();
            }
            KeyCode::Char('N') => {
                self.split_pane_horizontal();
            }
            KeyCode::Char('d') => {
                if let Some(pane) = self.selected_pane() {
                    self.start_confirm(
                        &format!("Kill pane '{}'? (y/n)", pane.id),
                        PendingAction::KillPane(pane.id.clone()),
                    );
                }
            }
            KeyCode::Enter => {
                self.switch_to_selected_pane();
            }
            KeyCode::Char('z') => {
                self.toggle_zoom_pane();
            }
            KeyCode::Char('w') => {
                self.break_pane_to_window();
            }
            _ => {}
        }
    }

    fn handle_preview_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('l') | KeyCode::Right => {
                self.focused = self.focused.next();
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.focused = self.focused.prev();
            }
            _ => {}
        }
    }

    fn move_selection_down(&mut self, panel: &Panel) {
        let (state, len) = self.state_and_len(panel);
        if len == 0 {
            return;
        }
        let i = state.selected().unwrap_or(0);
        let next = if i >= len - 1 { len - 1 } else { i + 1 };
        state.select(Some(next));
        self.on_selection_changed(panel);
    }

    fn move_selection_up(&mut self, panel: &Panel) {
        let (state, len) = self.state_and_len(panel);
        if len == 0 {
            return;
        }
        let i = state.selected().unwrap_or(0);
        let next = i.saturating_sub(1);
        state.select(Some(next));
        self.on_selection_changed(panel);
    }

    fn state_and_len(&mut self, panel: &Panel) -> (&mut ListState, usize) {
        match panel {
            Panel::Sessions => (&mut self.session_state, self.sessions.len()),
            Panel::Windows => (&mut self.window_state, self.windows.len()),
            Panel::Panes => (&mut self.pane_state, self.panes.len()),
            Panel::Preview => (&mut self.session_state, 0), // Preview has no list
        }
    }

    fn on_selection_changed(&mut self, panel: &Panel) {
        match panel {
            Panel::Sessions => {
                self.refresh_windows(None);
                self.refresh_panes(None);
                self.refresh_preview();
            }
            Panel::Windows => {
                self.refresh_panes(None);
                self.refresh_preview();
            }
            Panel::Panes => {
                self.refresh_preview();
            }
            Panel::Preview => {}
        }
    }

    fn execute_action(&mut self, action: PendingAction) {
        let result = match action {
            PendingAction::KillSession(ref name) => {
                tmux::kill_session(name).map(|_| format!("Session '{}' killed", name))
            }
            PendingAction::KillWindow(ref session, index) => {
                tmux::kill_window(session, index).map(|_| format!("Window {}:{} killed", session, index))
            }
            PendingAction::KillPane(ref id) => {
                tmux::kill_pane(id).map(|_| format!("Pane '{}' killed", id))
            }
            _ => return,
        };
        match result {
            Ok(msg) => {
                self.set_status(msg, false);
                self.refresh_tmux_state();
            }
            Err(e) => self.set_status(e.to_string(), true),
        }
    }

    fn execute_text_action(&mut self, action: PendingAction, value: &str) {
        let result = match action {
            PendingAction::CreateSession => {
                if value.is_empty() {
                    return;
                }
                tmux::new_session(value).map(|_| format!("Session '{}' created", value))
            }
            PendingAction::RenameSession(ref old) => {
                if value.is_empty() {
                    return;
                }
                tmux::rename_session(old, value)
                    .map(|_| format!("Session renamed to '{}'", value))
            }
            PendingAction::CreateWindow => {
                if let Some(session) = self.selected_session() {
                    let name = if value.is_empty() { None } else { Some(value) };
                    tmux::new_window(&session.name, name).map(|_| "Window created".to_string())
                } else {
                    return;
                }
            }
            PendingAction::RenameWindow(ref session, index) => {
                if value.is_empty() {
                    return;
                }
                tmux::rename_window(session, index, value)
                    .map(|_| format!("Window renamed to '{}'", value))
            }
            _ => return,
        };
        match result {
            Ok(msg) => {
                self.set_status(msg, false);
                self.refresh_tmux_state();
            }
            Err(e) => self.set_status(e.to_string(), true),
        }
    }

    fn switch_to_selected_session(&mut self) {
        if let Some(session) = self.selected_session() {
            if tmux::is_inside_tmux() {
                match tmux::switch_client(&session.name) {
                    Ok(_) => self.set_status(format!("Switched to '{}'", session.name), false),
                    Err(e) => self.set_status(e.to_string(), true),
                }
            } else {
                self.should_suspend = true;
                self.suspend_target = Some(session.name.clone());
            }
        }
    }

    fn switch_to_selected_window(&mut self) {
        if let (Some(session), Some(window)) = (self.selected_session(), self.selected_window()) {
            let target = format!("{}:{}", session.name, window.index);
            if tmux::is_inside_tmux() {
                if let Err(e) = tmux::select_window(&session.name, window.index) {
                    self.set_status(e.to_string(), true);
                    return;
                }
                match tmux::switch_client(&target) {
                    Ok(_) => self.set_status(format!("Switched to {}", target), false),
                    Err(e) => self.set_status(e.to_string(), true),
                }
            } else {
                self.should_suspend = true;
                self.suspend_target = Some(target);
            }
        }
    }

    fn switch_to_selected_pane(&mut self) {
        if let (Some(session), Some(window), Some(pane)) =
            (self.selected_session(), self.selected_window(), self.selected_pane())
        {
            let target = format!("{}:{}.{}", session.name, window.index, pane.index);
            if tmux::is_inside_tmux() {
                if let Err(e) = tmux::select_window(&session.name, window.index) {
                    self.set_status(e.to_string(), true);
                    return;
                }
                if let Err(e) = tmux::select_pane(&pane.id) {
                    self.set_status(e.to_string(), true);
                    return;
                }
                match tmux::switch_client(&format!("{}:{}", session.name, window.index)) {
                    Ok(_) => self.set_status(format!("Switched to {}", target), false),
                    Err(e) => self.set_status(e.to_string(), true),
                }
            } else {
                self.should_suspend = true;
                self.suspend_target = Some(target);
            }
        }
    }

    fn split_pane_vertical(&mut self) {
        if let (Some(session), Some(window), Some(pane)) =
            (self.selected_session(), self.selected_window(), self.selected_pane())
        {
            match tmux::split_window_vertical(&session.name, window.index, &pane.id) {
                Ok(_) => {
                    self.set_status("Pane split vertically".to_string(), false);
                    self.refresh_tmux_state();
                }
                Err(e) => self.set_status(e.to_string(), true),
            }
        }
    }

    fn split_pane_horizontal(&mut self) {
        if let (Some(session), Some(window), Some(pane)) =
            (self.selected_session(), self.selected_window(), self.selected_pane())
        {
            match tmux::split_window_horizontal(&session.name, window.index, &pane.id) {
                Ok(_) => {
                    self.set_status("Pane split horizontally".to_string(), false);
                    self.refresh_tmux_state();
                }
                Err(e) => self.set_status(e.to_string(), true),
            }
        }
    }

    fn toggle_zoom_pane(&mut self) {
        if let Some(pane) = self.selected_pane() {
            match tmux::resize_pane_zoom(&pane.id) {
                Ok(_) => self.set_status("Pane zoom toggled".to_string(), false),
                Err(e) => self.set_status(e.to_string(), true),
            }
        }
    }

    fn break_pane_to_window(&mut self) {
        if let Some(pane) = self.selected_pane() {
            match tmux::break_pane(&pane.id) {
                Ok(_) => {
                    self.set_status("Pane broken to new window".to_string(), false);
                    self.refresh_tmux_state();
                }
                Err(e) => self.set_status(e.to_string(), true),
            }
        }
    }

    pub fn tick(&mut self) {
        self.clear_stale_status();
        if self.last_refresh.elapsed().as_secs() >= self.refresh_interval_secs {
            self.refresh_tmux_state();
        }
    }
}
