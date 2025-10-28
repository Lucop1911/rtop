use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
};

use crate::{App, helpers::cpu::calculate_avg_cpu, helpers::memory, helpers::network};

pub fn draw_stats(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // CPU gauge
            Constraint::Length(5), // Memory gauge
            Constraint::Min(3),    // Network info
        ])
        .split(area);

    // --- Overall CPU Usage ---
    let avg_cpu: f32 = calculate_avg_cpu(app);

    let cpu_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("CPU Usage (Overall)"),
        )
        .gauge_style(Style::default().fg(Color::Cyan).bg(Color::Black))
        .percent(avg_cpu as u16);

    f.render_widget(cpu_gauge, chunks[0]);

    // --- Memory Usage ---
    let (used_mem, total_mem, mem_percent) = memory::calculate_memory(app);

    let mem_gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(format!(
            "Memory: {:.2} GB / {:.2} GB ({:.1}%)",
            used_mem,
            total_mem,
            (used_mem / total_mem) * 100.0
        )))
        .gauge_style(Style::default().fg(Color::Green))
        .percent(mem_percent);

    f.render_widget(mem_gauge, chunks[1]);

    // --- Network Usage ---
    let (total_rx, total_tx) = network::calculate_network_totals(app);

    let net_info: Vec<Line> = network::per_interface_info(app)
        .iter()
        .map(|(name, rx, tx)| {
            Line::from(vec![
                Span::styled(format!("{}: ", name), Style::default().fg(Color::Cyan)),
                Span::styled(format!("↓ {:.2} MB", rx), Style::default().fg(Color::Green)),
                Span::raw(" / "),
                Span::styled(format!("↑ {:.2} MB", tx), Style::default().fg(Color::Blue)),
            ])
        })
        .collect();

    let network = Paragraph::new(net_info)
        .block(Block::default().borders(Borders::ALL).title(format!(
            "Network (Total: ↓ {:.2} MB / ↑ {:.2} MB)",
            total_rx as f64 / 1024.0 / 1024.0,
            total_tx as f64 / 1024.0 / 1024.0
        )))
        .alignment(Alignment::Left);

    f.render_widget(network, chunks[2]);
}
