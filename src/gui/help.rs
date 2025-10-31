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

    // Keybindings
    let bindings = vec![
        ("Esc / h / ?", "Go back to processes"),
        ("q / Esc", "Quit application"),
        ("Enter/ Space", "Open selected process"),
        ("k", "Kill or delete process"),
        ("↓", "Move selection down"),
        ("↑", "Move selection up"),
        ("PageUP", "Navigate up"),
        ("PageDOWN", "Navigate down"),
        ("t", "Move to top"),
        ("b", "Move to bottom"),
        ("p", "Sort per process ID"),
        ("n", "Sort per process name"),
        ("c", "sort per CPU usage"),
        ("m", "Sort per Memory usage"),
        ("f - /", "search"),
        ("+", "Increase refresh speed"),
        ("-", "Decrease refresh speed"),
        ("R", "Refresh"),
    ];

    let rows: Vec<Row> = bindings
        .iter()
        .map(|(key, desc)| {
            Row::new(vec![
                Span::styled(*key, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(*desc),
            ])
        })
        .collect();
    
    //Tabella
    let table = Table::new(
        rows,
        [
            Constraint::Length(20),
            Constraint::Percentage(80),
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
