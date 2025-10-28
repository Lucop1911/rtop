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
            ratatui::text::Span::raw(" | ESC: Cancel | Enter: Confirm"),
        ])]
    } else {
        vec![Line::from(vec![ratatui::text::Span::raw(
            "F1: Processes | F2: Stats | Ctrl+F: Search | Shift+Q: Kill | Enter: Expand | ↑↓: Navigate | Mouse: Click & Scroll | Ctrl+C: Exit",
        )])]
    };

    let footer = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL))
        .alignment(ratatui::layout::Alignment::Center)
        .style(Style::default().fg(Color::Cyan));

    f.render_widget(footer, area);
}
