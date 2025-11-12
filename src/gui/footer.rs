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
        let filters = get_active_filters_detailed(app);
        
        vec![Line::from(vec![
            ratatui::text::Span::raw("?: Help | 1: Processes | 2: Stats | /: Search | i: Interval | k: Kill | "),
            ratatui::text::Span::raw("p/n/c/m: Sort | "),
            ratatui::text::Span::raw("+/-: Speed ("),
                if app.refresh == true {
                    ratatui::text::Span::styled(    
                        format!("{}ms", update_ms),
                        Style::default().fg(Color::Yellow)
                    )
                } else {
                    ratatui::text::Span::styled(    
                        format!("{}ms - STOPPED", update_ms),
                        Style::default().fg(Color::Yellow)
                    )
                },
            ratatui::text::Span::raw(")"),
            if !filters.is_empty() {
                ratatui::text::Span::styled(
                    format!(" | Active: {}", filters),
                    Style::default().fg(Color::Magenta)
                )
            } else {
                ratatui::text::Span::raw("| w: Select filter")
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

fn get_active_filters_detailed(app: &App) -> String {
    let mut filters = Vec::new();
    
    if let Some(ref user) = app.user_filter {
        filters.push(format!("User:{}", user));
    }
    if let Some(ref status) = app.status_filter {
        filters.push(format!("Status:{}", status));
    }
    if let Some(threshold) = app.cpu_threshold {
        filters.push(format!("CPU≥{:.1}%", threshold));
    }
    if let Some(threshold) = app.memory_threshold {
        filters.push(format!("Mem≥{}MB", threshold / 1024 / 1024));
    }
    
    filters.join(", ")
}