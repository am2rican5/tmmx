mod help;
mod panes;
mod preview;
mod prompt;
mod sessions;
mod template_picker;
mod windows;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, InputMode, Panel};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let size = frame.area();

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(size);

    let main_area = outer[0];
    let bottom_area = outer[1];

    // Main panel layout
    if size.width >= 100 {
        draw_wide_layout(frame, app, main_area);
    } else {
        draw_narrow_layout(frame, app, main_area);
    }

    // Status bar
    draw_status_bar(frame, app, bottom_area);

    // Overlays
    if app.mode == InputMode::TextInput {
        prompt::draw_text_input(frame, app);
    }
    if app.mode == InputMode::Confirm {
        prompt::draw_confirm(frame, app);
    }
    if app.mode == InputMode::TemplatePicker {
        template_picker::draw_template_picker(frame, app);
    }
    if app.mode == InputMode::Help {
        help::draw_help(frame, app);
    }
}

fn draw_wide_layout(frame: &mut Frame, app: &mut App, area: Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(30),
            Constraint::Percentage(50),
        ])
        .split(area);

    // Left column: Sessions (top) + Windows (bottom)
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(columns[0]);

    sessions::draw_sessions(frame, app, left[0]);
    windows::draw_windows(frame, app, left[1]);
    panes::draw_panes(frame, app, columns[1]);
    preview::draw_preview(frame, app, columns[2]);
}

fn draw_narrow_layout(frame: &mut Frame, app: &mut App, area: Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(columns[0]);

    sessions::draw_sessions(frame, app, left[0]);
    windows::draw_windows(frame, app, left[1]);

    // Right area shows panes or preview depending on focus
    match app.focused {
        Panel::Preview => preview::draw_preview(frame, app, columns[1]),
        _ => panes::draw_panes(frame, app, columns[1]),
    }
}

pub fn panel_block(title: &str, focused: bool) -> Block<'_> {
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", title))
        .border_style(border_style)
}

fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let widget = if let Some(ref status) = app.status {
        let color = if status.is_error {
            Color::Red
        } else {
            Color::Green
        };
        Paragraph::new(Line::from(Span::styled(
            &status.text,
            Style::default().fg(color),
        )))
    } else {
        Paragraph::new(Line::from(Span::styled(
            " q:quit  ?:help  1-3:panels  n:new  r:rename  d:delete  Enter:switch  R:refresh",
            Style::default().fg(Color::DarkGray),
        )))
    };
    frame.render_widget(widget, area);
}
