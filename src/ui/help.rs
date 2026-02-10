use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::{App, Panel};

pub fn draw_help(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 70, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Help (press ? or Esc to close) ")
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = vec![
        section_header("Global"),
        key_line("q", "Quit"),
        key_line("?", "Toggle help"),
        key_line("R", "Force refresh"),
        key_line("1-4", "Switch panel"),
        key_line("Tab/S-Tab", "Next/prev panel"),
        Line::from(""),
    ];

    let panel_lines = match app.focused {
        Panel::Sessions => vec![
            section_header("Sessions"),
            key_line("j/k ↑/↓", "Navigate"),
            key_line("h/l ←/→", "Switch panel"),
            key_line("n", "New session"),
            key_line("r", "Rename session"),
            key_line("d", "Kill session"),
            key_line("Enter", "Switch to session"),
        ],
        Panel::Windows => vec![
            section_header("Windows"),
            key_line("j/k ↑/↓", "Navigate"),
            key_line("h/l ←/→", "Switch panel"),
            key_line("n", "New window"),
            key_line("r", "Rename window"),
            key_line("d", "Kill window"),
            key_line("Enter", "Switch to window"),
        ],
        Panel::Panes => vec![
            section_header("Panes"),
            key_line("j/k ↑/↓", "Navigate"),
            key_line("h/l ←/→", "Switch panel"),
            key_line("n", "Split vertical"),
            key_line("N", "Split horizontal"),
            key_line("d", "Kill pane"),
            key_line("z", "Toggle zoom"),
            key_line("w", "Break to window"),
            key_line("Enter", "Switch to pane"),
        ],
        Panel::Preview => vec![
            section_header("Preview"),
            key_line("h/l ←/→", "Switch panel"),
        ],
    };

    lines.extend(panel_lines);

    let widget = Paragraph::new(lines);
    frame.render_widget(widget, inner);
}

fn section_header(title: &str) -> Line<'static> {
    Line::from(Span::styled(
        title.to_string(),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ))
}

fn key_line(key: &str, desc: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  {:12}", key),
            Style::default().fg(Color::Yellow),
        ),
        Span::raw(desc.to_string()),
    ])
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}
