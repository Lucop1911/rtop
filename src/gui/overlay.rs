use crate::{App, InputMode};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn draw_input_overlay(f: &mut Frame, app: &App) {
    match app.input_mode {
        InputMode::SelectFilter => {
            let area = centered_rect(60, 20, f.area());

            f.render_widget(Clear, area);

            let block = Block::default()
                .title("Select a filter (0. Reset filters / 1. User / 2. Status / 3. CPU% / 4. Memory)")
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Black));

            let text = vec![
                Line::from(""),
                Line::from(vec![
                    Span::raw("Enter numer (0 - 4): "),
                    Span::styled(&app.input_buffer, Style::default().fg(Color::Yellow)),
                ]),
                Line::from(""),
                Line::from("Press Enter to confirm, Esc to cancel"),
            ];

            let paragraph = Paragraph::new(text)
                .block(block)
                .alignment(ratatui::layout::Alignment::Center)
                .style(Style::default().bg(Color::Black));

            f.render_widget(paragraph, area);
        }

        InputMode::UpdateInterval => {
            let area = centered_rect(60, 20, f.area());

            f.render_widget(Clear, area);

            let block = Block::default()
                .title("Set Update Interval (ms)")
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Black));

            let text = vec![
                Line::from(""),
                Line::from(vec![
                    Span::raw("Enter interval (100-6000 ms): "),
                    Span::styled(&app.input_buffer, Style::default().fg(Color::Yellow)),
                ]),
                Line::from(""),
                Line::from("Press Enter to confirm, Esc to cancel"),
            ];

            let paragraph = Paragraph::new(text)
                .block(block)
                .alignment(ratatui::layout::Alignment::Center)
                .style(Style::default().bg(Color::Black));

            f.render_widget(paragraph, area);
        }
        InputMode::ConfirmKill => {
            let area = centered_rect(60, 20, f.area());

            f.render_widget(Clear, area);

            let block = Block::default()
                .title("⚠ Confirm Kill Critical Process")
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Black));

            let text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "This appears to be a critical system process!",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from("Are you sure you want to kill this process?"),
                Line::from(""),
                Line::from("Press Y to confirm, N or Esc to cancel"),
            ];

            let paragraph = Paragraph::new(text)
                .block(block)
                .alignment(ratatui::layout::Alignment::Center)
                .style(Style::default().bg(Color::Black));

            f.render_widget(paragraph, area);
        }
        InputMode::UserFilter => {
            let area = centered_rect(60, 25, f.area());

            f.render_widget(Clear, area);

            let block = Block::default()
                .title("Filter by User ID")
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Black));

            let text = vec![
                Line::from(""),
                Line::from(vec![
                    Span::raw("Enter User ID: "),
                    Span::styled(&app.input_buffer, Style::default().fg(Color::Yellow)),
                ]),
                Line::from(""),
                Line::from("Shows processes matching the specified user ID"),
                Line::from("Leave empty to clear filter"),
                Line::from(""),
                Line::from("Press Enter to confirm, Esc to cancel"),
            ];

            let paragraph = Paragraph::new(text)
                .block(block)
                .alignment(ratatui::layout::Alignment::Center)
                .style(Style::default().bg(Color::Black));

            f.render_widget(paragraph, area);
        }
        InputMode::StatusFilter => {
            let area = centered_rect(60, 30, f.area());

            f.render_widget(Clear, area);

            let block = Block::default()
                .title("Filter by Status")
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Black));

            let text = vec![
                Line::from(""),
                Line::from(vec![
                    Span::raw("Enter Status: "),
                    Span::styled(&app.input_buffer, Style::default().fg(Color::Yellow)),
                ]),
                Line::from(""),
                Line::from("Shows processes matching the specified status"),
                Line::from(""),
                Line::from(Span::styled(
                    "Common statuses:",
                    Style::default().fg(Color::Cyan),
                )),
                Line::from("  • Running, Sleeping, Stopped, Zombie"),
                Line::from(""),
                Line::from("Leave empty to clear filter"),
                Line::from(""),
                Line::from("Press Enter to confirm, Esc to cancel"),
            ];

            let paragraph = Paragraph::new(text)
                .block(block)
                .alignment(ratatui::layout::Alignment::Center)
                .style(Style::default().bg(Color::Black));

            f.render_widget(paragraph, area);
        }
        InputMode::CpuThreshold => {
            let area = centered_rect(60, 25, f.area());

            f.render_widget(Clear, area);

            let block = Block::default()
                .title("Filter by CPU Threshold")
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Black));

            let text = vec![
                Line::from(""),
                Line::from(vec![
                    Span::raw("Enter minimum CPU% (0-100): "),
                    Span::styled(&app.input_buffer, Style::default().fg(Color::Yellow)),
                ]),
                Line::from(""),
                Line::from("Shows only processes using >= specified CPU%"),
                Line::from("Leave empty to clear filter"),
                Line::from(""),
                Line::from("Press Enter to confirm, Esc to cancel"),
            ];

            let paragraph = Paragraph::new(text)
                .block(block)
                .alignment(ratatui::layout::Alignment::Center)
                .style(Style::default().bg(Color::Black));

            f.render_widget(paragraph, area);
        }
        InputMode::MemoryThreshold => {
            let area = centered_rect(60, 25, f.area());

            f.render_widget(Clear, area);

            let block = Block::default()
                .title("Filter by Memory Threshold")
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Black));

            let text = vec![
                Line::from(""),
                Line::from(vec![
                    Span::raw("Enter minimum Memory (MB): "),
                    Span::styled(&app.input_buffer, Style::default().fg(Color::Yellow)),
                ]),
                Line::from(""),
                Line::from("Shows only processes using >= specified MB"),
                Line::from("Leave empty to clear filter"),
                Line::from(""),
                Line::from("Press Enter to confirm, Esc to cancel"),
            ];

            let paragraph = Paragraph::new(text)
                .block(block)
                .alignment(ratatui::layout::Alignment::Center)
                .style(Style::default().bg(Color::Black));

            f.render_widget(paragraph, area);
        }
        InputMode::Error => {
            let area = centered_rect(60, 20, f.area());
            f.render_widget(Clear, area);

            let block = Block::default()
                .title("Errors occurred")
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Black));

            let mut lines: Vec<Line> = app
                .errors
                .iter()
                .map(|(label, message)| {
                    Line::from(vec![
                        Span::styled(format!("[{}] ", label), Style::default().fg(Color::Red)),
                        Span::raw(message.clone()),
                    ])
                })
                .collect();

            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Press Enter to confirm, Esc to cancel",
                Style::default().fg(Color::Gray),
            )));

            let paragraph = Paragraph::new(lines)
                .block(block)
                .alignment(ratatui::layout::Alignment::Left)
                .style(Style::default().bg(Color::Black));

            f.render_widget(paragraph, area);
        }

        InputMode::None => {}
    }
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
