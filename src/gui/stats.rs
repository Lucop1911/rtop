use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
};

use crate::{App, helpers::utils::{calculate_avg_cpu, generate_sparkline}, helpers::memory, helpers::network};

pub fn draw_stats(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(16),  // CPU + per core infos
            Constraint::Length(8),  // Memory con history
            Constraint::Min(5),     // Network
        ])
        .split(area);

    draw_cpu_section(f, app, chunks[0]);
    draw_memory_section(f, app, chunks[1]);
    draw_network_section(f, app, chunks[2]);
}

fn draw_cpu_section(f: &mut Frame, app: &App, area: Rect) {
    let avg_cpu: f32 = calculate_avg_cpu(app);

    let cpu_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("CPU Usage (Overall)"),
        )
        .gauge_style(Style::default().fg(Color::Cyan).bg(Color::Black))
        .percent(avg_cpu as u16);

    // Split area vertically: overall + per-core
    let cpu_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(area);

    f.render_widget(cpu_gauge, cpu_chunks[0]);

    // Split per-core area into two columns
    let per_core_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(cpu_chunks[1]);

    let cpus = app.system.cpus();
    let half = (cpus.len() + 1) / 2;

    // Helper closure to generate column lines
    let build_core_lines = |slice: &[sysinfo::Cpu]| {
        let mut lines = Vec::new();
        for (i, cpu) in slice.iter().enumerate() {
            let global_idx = app.system.cpus().iter().position(|c| std::ptr::eq(c, cpu)).unwrap_or(i);
            let usage = cpu.cpu_usage();

            // Fetch sparkline safely
            let history = app.cpu_history.get(global_idx).map(|h| &h[..]).unwrap_or(&[]);
            let sparkline = if !history.is_empty() {
                generate_sparkline(history)
            } else {
                String::from("▁".repeat(20))
            };

            let color = if usage > 80.0 {
                Color::Red
            } else if usage > 50.0 {
                Color::Yellow
            } else {
                Color::Green
            };

            lines.push(Line::from(vec![
                Span::styled(format!("CPU{:2}: ", global_idx), Style::default().fg(Color::Cyan)),
                Span::styled(format!("{:5.1}%", usage), Style::default().fg(color)),
                Span::raw("  "),
                Span::styled(sparkline, Style::default().fg(Color::Blue)),
            ]));

            lines.push(Line::from(Span::raw(" ")));
        }
        lines
    };

    // Left and right columns
    let left_lines = build_core_lines(&cpus[..half]);
    let right_lines = build_core_lines(&cpus[half..]);

    let left_widget = Paragraph::new(left_lines)
        .block(Block::default().borders(Borders::ALL).title("Per-Core Usage (1/2)"))
        .alignment(Alignment::Left);

    let right_widget = Paragraph::new(right_lines)
        .block(Block::default().borders(Borders::ALL).title("Per-Core Usage (2/2)"))
        .alignment(Alignment::Left);

    f.render_widget(left_widget, per_core_cols[0]);
    f.render_widget(right_widget, per_core_cols[1]);
}


fn draw_memory_section(f: &mut Frame, app: &App, area: Rect) {
    let (used_mem, total_mem, mem_percent) = memory::calculate_memory(app);

    // Utilizzo memoria
    let mem_sparkline = generate_sparkline(
        &app.memory_history.iter().map(|&x| x as f32).collect::<Vec<f32>>()
    );

    let mem_gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(format!(
            "Memory: {:.2} GB / {:.2} GB ({:.1}%)",
            used_mem,
            total_mem,
            (used_mem / total_mem) * 100.0
        )))
        .gauge_style(Style::default().fg(Color::Green))
        .percent(mem_percent);

    let mem_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(area);

    f.render_widget(mem_gauge, mem_chunks[0]);

    // Memory history
    let history_text = vec![
        Line::from(vec![
            Span::styled("History (60s): ", Style::default().fg(Color::Cyan)),
            Span::styled(mem_sparkline, Style::default().fg(Color::Green)),
        ]),
    ];

    let history = Paragraph::new(history_text)
        .block(Block::default().borders(Borders::ALL).title("Memory Trend"))
        .alignment(Alignment::Left);

    f.render_widget(history, mem_chunks[1]);
}

fn draw_network_section(f: &mut Frame, app: &App, area: Rect) {
    let (total_rx, total_tx) = network::calculate_network_totals(app);

    let net_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(area);

    // Network history sparkline
    let rx_history: Vec<f32> = app.network_history
        .iter()
        .map(|(rx, _)| *rx as f32 / 1024.0 / 1024.0)
        .collect();
    let tx_history: Vec<f32> = app.network_history
        .iter()
        .map(|(_, tx)| *tx as f32 / 1024.0 / 1024.0)
        .collect();

    let rx_sparkline = generate_sparkline(&rx_history);
    let tx_sparkline = generate_sparkline(&tx_history);

    let summary = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("↓ Received: ", Style::default().fg(Color::Green)),
            Span::styled(rx_sparkline, Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("↑ Sent:     ", Style::default().fg(Color::Blue)),
            Span::styled(tx_sparkline, Style::default().fg(Color::Blue)),
        ]),
    ])
    .block(Block::default().borders(Borders::ALL).title(format!(
        "Network History (Total: ↓ {:.2} MB / ↑ {:.2} MB)",
        total_rx as f64 / 1024.0 / 1024.0,
        total_tx as f64 / 1024.0 / 1024.0
    )))
    .alignment(Alignment::Left);

    f.render_widget(summary, net_chunks[0]);

    // Dettagli per interfaccia
    let net_info: Vec<Line> = network::per_interface_info(app)
        .iter()
        .map(|(name, rx, tx)| {
            Line::from(vec![
                Span::styled(format!("{:12}: ", name), Style::default().fg(Color::Cyan)),
                Span::styled(format!("↓ {:8.2} MB", rx), Style::default().fg(Color::Green)),
                Span::raw(" / "),
                Span::styled(format!("↑ {:8.2} MB", tx), Style::default().fg(Color::Blue)),
            ])
        })
        .collect();

    let interfaces = Paragraph::new(net_info)
        .block(Block::default().borders(Borders::ALL).title("Per-Interface Stats"))
        .alignment(Alignment::Left);

    f.render_widget(interfaces, net_chunks[1]);
}