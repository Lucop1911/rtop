use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
};

use crate::{App, gui::input_overlay::draw_input_overlay, helpers::{memory, network, utils::{calculate_avg_cpu, generate_sparkline, generate_sparkline_with_max}}};

pub fn draw_stats(f: &mut Frame, app: &App, area: Rect) {
    let num_cpus = app.system.cpus().len();
    let rows_per_column = (num_cpus + 1) / 2;
    let cpu_cores_height = (rows_per_column * 2) as u16;
    let cpu_total_height = 3 + 2 + cpu_cores_height;
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(cpu_total_height),  // CPU
            Constraint::Length(7),                 // Memory
            Constraint::Min(5),                    // Networ
        ])
        .split(area);

    draw_cpu_section(f, app, chunks[0]);
    draw_memory_section(f, app, chunks[1]);
    draw_network_section(f, app, chunks[2]);

    draw_input_overlay(f, app);
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

    let cpu_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(area);

    f.render_widget(cpu_gauge, cpu_chunks[0]);

    let per_core_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(cpu_chunks[1]);

    let cpus = app.system.cpus();
    let half = (cpus.len() + 1) / 2;

    let build_core_lines = |slice: &[sysinfo::Cpu]| {
        let mut lines = Vec::new();
        for (i, cpu) in slice.iter().enumerate() {
            let global_idx = app.system.cpus().iter().position(|c| std::ptr::eq(c, cpu)).unwrap_or(i);
            let usage = cpu.cpu_usage();

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

    let mem_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(4),
        ])
        .split(area);

    let mem_gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(format!(
            "Memory: {:.2} GB / {:.2} GB ({:.1}%)",
            used_mem,
            total_mem,
            (used_mem / total_mem) * 100.0
        )))
        .gauge_style(Style::default().fg(Color::Green))
        .percent(mem_percent);

    f.render_widget(mem_gauge, mem_chunks[0]);

    let history_width = mem_chunks[1].width.saturating_sub(4) as usize;
    
    let mem_sparkline = if app.memory_history.is_empty() {
        "▁".repeat(history_width.min(60))
    } else if app.memory_history.len() >= history_width {
        let start_idx = app.memory_history.len() - history_width;
        let sampled: Vec<f32> = app.memory_history[start_idx..]
            .iter()
            .map(|&x| x as f32)
            .collect();
        generate_sparkline_with_max(&sampled, total_mem as f32)
    } else {
        let mem_data: Vec<f32> = app.memory_history.iter().map(|&x| x as f32).collect();
        generate_sparkline_with_max(&mem_data, total_mem as f32)
    };

    let history_text = vec![
        Line::from(vec![
            Span::styled("History: ", Style::default().fg(Color::Cyan)),
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
            Constraint::Length(6),
            Constraint::Min(1),
        ])
        .split(area);

    let inner_width = net_chunks[0].width.saturating_sub(4) as usize; // Remove borders
    let label_width = "↓ Received: ".len();
    let sparkline_width = inner_width.saturating_sub(label_width).max(20);

    let sample_network = |history: &[(u64, u64)], extract_fn: fn(&(u64, u64)) -> u64| -> String {
        if history.is_empty() {
            return "▁".repeat(sparkline_width.min(60));
        }
        
        if history.len() >= sparkline_width {
            let start_idx = history.len() - sparkline_width;
            let sampled: Vec<f32> = history[start_idx..]
                .iter()
                .map(|item| extract_fn(item) as f32 / 1024.0 / 1024.0)
                .collect();
            generate_sparkline(&sampled)
        } else {
            let data: Vec<f32> = history
                .iter()
                .map(|item| extract_fn(item) as f32 / 1024.0 / 1024.0)
                .collect();
            generate_sparkline(&data)
        }
    };

    let rx_sparkline = sample_network(&app.network_history, |&(rx, _)| rx);
    let tx_sparkline = sample_network(&app.network_history, |&(_, tx)| tx);

    let summary = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("↓ Received: ", Style::default().fg(Color::Green)),
            Span::styled(rx_sparkline, Style::default().fg(Color::Green)),
        ]),
        Line::from(""),
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