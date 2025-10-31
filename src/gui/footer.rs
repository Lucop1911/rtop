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
        let filters = get_active_filters(app);
        
        vec![Line::from(vec![
            ratatui::text::Span::raw("?: Help | 1: Processes | 2: Stats | /: Search | i: Interval | k: Kill | p/n/c/m: Sort by PID/ Name/ CPU%/ Mem | "),
            ratatui::text::Span::raw("+/-: Speed ("),
            ratatui::text::Span::styled(
                format!("{}ms", update_ms),
                Style::default().fg(Color::Yellow)
            ),
            ratatui::text::Span::raw(")"),
            if !filters.is_empty() {
                ratatui::text::Span::styled(
                    format!(" | Filters: {}", filters),
                    Style::default().fg(Color::Magenta)
                )
            } else {
                ratatui::text::Span::raw("")
            },
            ratatui::text::Span::raw(" | q: Exit"),
        ])]
    };

    let footer = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL))
        .alignment(ratatui::layout::Alignment::Center)
        .style(Style::default().fg(Color::Cyan));

    f.render_widget(footer, area);
}

fn get_active_filters(app: &App) -> String {
    let mut filters = Vec::new();
    
    if app.user_filter.is_some() {
        filters.push("User");
    }
    if app.status_filter.is_some() {
        filters.push("Status");
    }
    if app.cpu_threshold.is_some() {
        filters.push("CPU");
    }
    if app.memory_threshold.is_some() {
        filters.push("Memory");
    }
    
    filters.join(", ")
}