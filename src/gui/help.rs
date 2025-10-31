use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph, Row, Table},
};

use crate::App;

pub fn draw_help(f: &mut Frame, _app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
        ])
        .split(area);

    let title_block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            " Help / Cheatsheet ",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ));

    let title_paragraph = Paragraph::new("List of available keybindings")
        .block(title_block)
        .alignment(ratatui::layout::Alignment::Center)
        .style(Style::default().fg(Color::White));

    f.render_widget(title_paragraph, chunks[0]);

    let bindings = vec![
        ("Navigation", ""),
        ("↑/↓", "Move selection up/down"),
        ("PageUp/PageDown", "Navigate by page"),
        ("t", "Jump to top"),
        ("b", "Jump to bottom"),
        ("", ""),
        ("Actions", ""),
        ("Enter/Space", "Expand/collapse process tree"),
        ("k/Del", "Kill process (with confirmation for critical)"),
        ("s", "Suspend process (SIGSTOP)"),
        ("u", "Resume process (SIGCONT)"),
        ("o", "Show open files"),
        ("r", "Force refresh"),
        ("", ""),
        ("Sorting", ""),
        ("p", "Sort by PID"),
        ("n", "Sort by name"),
        ("c", "Sort by CPU usage"),
        ("m", "Sort by memory usage"),
        ("", ""),
        ("Search & Filter", ""),
        ("/ or Ctrl+F", "Search processes"),
        ("w", "Filter by user ID"),
        ("l", "Clear all filters"),
        ("", ""),
        ("View & Settings", ""),
        ("1 or F1", "Process view"),
        ("2 or F2", "System stats view"),
        ("3 or F3 or h or ?", "Help screen"),
        ("i", "Set custom update interval"),
        ("+ / -", "Increase/decrease update speed"),
        ("", ""),
        ("General", ""),
        ("q or Esc", "Quit (saves preferences)"),
        ("Ctrl+C", "Force quit (saves preferences)"),
    ];

    let rows: Vec<Row> = bindings
        .iter()
        .map(|(key, desc)| {
            if key.is_empty() {
                Row::new(vec![
                    Span::raw(""),
                    Span::styled(*desc, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                ])
            } else {
                Row::new(vec![
                    Span::styled(*key, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::raw(*desc),
                ])
            }
        })
        .collect();
    
    let table = Table::new(
        rows,
        [
            Constraint::Length(25),
            Constraint::Percentage(75),
        ],
    )
    .header(
        Row::new(vec![
            Span::styled("Key", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled("Action", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ])
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Keybindings ")
            .title_alignment(ratatui::layout::Alignment::Center)
    )
    .style(Style::default().fg(Color::White));

    f.render_widget(table, chunks[1]);
}