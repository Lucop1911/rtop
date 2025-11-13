use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph, Row, Table},
};

use crate::{App, gui::overlay::draw_input_overlay};

pub fn draw_help(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(area);

    let title_block = Block::default().borders(Borders::ALL).title(Span::styled(
        " Help / Cheatsheet ",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    ));

    let title_paragraph = Paragraph::new("List of available keybindings")
        .block(title_block)
        .alignment(Alignment::Center)
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
        ("r", "Resume process (SIGCONT)"),
        ("Ctrl+r", "Force refresh"),
        ("", ""),
        ("Sorting", ""),
        ("p", "Sort by PID"),
        ("n", "Sort by name"),
        ("c", "Sort by CPU usage"),
        ("m", "Sort by memory usage"),
        ("", ""),
        ("Search & Filter", ""),
        ("/ or Ctrl+F", "Search processes"),
        ("w", "Select the filtering mode"),
        ("l", "Clear all filters"),
        ("", ""),
        ("View & Settings", ""),
        ("1 or F1", "Process view"),
        ("2 or F2", "System stats view"),
        ("3 or F3 or h or ?", "Help screen"),
        ("i", "Set custom update interval"),
        ("+ / -", "Increase/decrease update speed"),
        ("z", "Toggle auto refresh"),
        ("", ""),
        ("General", ""),
        ("q or Esc", "Quit (saves preferences)"),
        ("Ctrl+C", "Force quit (saves preferences)"),
    ];

    let mid = (bindings.len() + 1) / 2;
    let (left_bindings, right_bindings) = bindings.split_at(mid);

    fn make_rows<'a>(left: &'a [(&'a str, &'a str)], right: &'a [(&'a str, &'a str)]) -> Vec<Row<'a>> {
        let max_len = left.len().max(right.len());
        let mut rows = Vec::with_capacity(max_len);

        for i in 0..max_len {
            let (lk, ld) = left.get(i).copied().unwrap_or(("", ""));
            let (rk, rd) = right.get(i).copied().unwrap_or(("", ""));

            rows.push(Row::new(vec![
                Span::styled(
                    lk,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(ld),
                Span::raw(" "),
                Span::styled(
                    rk,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(rd),
            ]));
        }

        rows
    }

    let rows = make_rows(left_bindings, right_bindings);

    let table = Table::new(
        rows,
        [
            Constraint::Length(20),
            Constraint::Length(35),
            Constraint::Length(10),
            Constraint::Length(20),
            Constraint::Min(25),
        ],
    )
    .header(Row::new(vec![
        Span::styled(
            "Key",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "Action",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " ",
            Style::default()
        ),
        Span::styled(
            "Key",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "Action",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Keybindings ")
            .title_alignment(Alignment::Center),
    )
    .style(Style::default().fg(Color::White));

    f.render_widget(table, chunks[1]);

    draw_input_overlay(f, app);
}