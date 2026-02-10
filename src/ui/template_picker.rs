use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, Paragraph};

use crate::app::App;

pub fn draw_template_picker(frame: &mut Frame, app: &mut App) {
    let area = centered_rect(60, 70, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Templates (Enter:launch  d:delete  Esc:close) ")
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.templates.is_empty() {
        let msg = Paragraph::new(Line::from(Span::styled(
            "(no templates — press S on a session to save one)",
            Style::default().fg(Color::DarkGray),
        )));
        frame.render_widget(msg, inner);
        return;
    }

    // Split inner area: template list (left 40%) + preview (right 60%)
    let split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(inner);

    // Template list
    let items: Vec<Line> = app
        .templates
        .iter()
        .map(|t| {
            if t.template.description.is_empty() {
                Line::from(t.template.name.clone())
            } else {
                Line::from(vec![
                    Span::raw(&t.template.name),
                    Span::styled(
                        format!(" — {}", t.template.description),
                        Style::default().fg(Color::DarkGray),
                    ),
                ])
            }
        })
        .collect();

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, split[0], &mut app.template_state);

    // Preview: show structure of selected template
    let preview_block = Block::default()
        .borders(Borders::LEFT)
        .border_style(Style::default().fg(Color::DarkGray));

    let preview_inner = preview_block.inner(split[1]);
    frame.render_widget(preview_block, split[1]);

    if let Some(t) = app.selected_template() {
        let mut lines = vec![
            Line::from(Span::styled(
                format!("Template: {}", t.template.name),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];

        for (i, win) in t.windows.iter().enumerate() {
            lines.push(Line::from(Span::styled(
                format!("  Window {}: {}", i, win.name),
                Style::default().fg(Color::Yellow),
            )));
            lines.push(Line::from(format!("    cwd: {}", win.cwd)));
            lines.push(Line::from(format!("    panes: {}", win.panes.len())));
            for (pi, pane) in win.panes.iter().enumerate() {
                lines.push(Line::from(format!(
                    "      [{}] {:?} — {}",
                    pi, pane.split, pane.cwd
                )));
            }
        }

        let preview = Paragraph::new(lines);
        frame.render_widget(preview, preview_inner);
    }
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
