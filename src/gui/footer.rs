use crate::App;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
};

pub fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let text = if app.search_mode {
        vec![Line::from(vec![
            ratatui::text::Span::raw("Search: "),
            ratatui::text::Span::styled(&app.search_query, Style::default().fg(Color::Yellow)),
            ratatui::text::Span::raw(" | ↑↓: Navigate | ESC: Cancel | Enter: Confirm"),
        ])]
    } else {
        let update_ms = app.update_interval.as_millis();
        vec![Line::from(vec![
            ratatui::text::Span::raw("1: Processes | 2: Stats | /: Search | K: Kill | R: Refresh | Space/Enter: Expand | "),
            ratatui::text::Span::raw("↑↓/PgUp/PgDn: Navigate | g/h: Top/Bottom | "),
            ratatui::text::Span::raw("p/n/c/m: Sort | +/-: Speed ("),
            ratatui::text::Span::styled(
                format!("{}ms", update_ms),
                Style::default().fg(Color::Yellow)
            ),
            ratatui::text::Span::raw(") | q: Exit"),
        ])]
    };

    let footer = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL))
        .alignment(ratatui::layout::Alignment::Left)
        .style(Style::default().fg(Color::Cyan));

    f.render_widget(footer, area);
}