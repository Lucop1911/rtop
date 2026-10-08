use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    symbols::Marker,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Dataset, Gauge, GraphType, Paragraph},
};

use crate::{
    App,
    gui::overlay::draw_input_overlay,
    helpers::{gpu::GpuInfo, memory, network, utils::generate_sparkline},
};

pub fn draw_stats(f: &mut Frame, app: &App, area: Rect) {
    let num_cpus = app.cpu_usage.len();
    let rows_per_column = num_cpus.div_ceil(2);
    let cpu_cores_height = (rows_per_column * 2) as u16;
    let cpu_total_height = 3 + 2 + cpu_cores_height;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(cpu_total_height), // CPU
            Constraint::Length(3),                // Memory
            Constraint::Length(6),                // Network (history + per-interface)
            Constraint::Min(5),                   // GPU (gets the space left at the bottom)
        ])
        .split(area);

    draw_cpu_section(f, app, chunks[0]);
    draw_memory_section(f, app, chunks[1]);
    draw_network_section(f, app, chunks[2]);
    draw_gpu_section(f, app, chunks[3]);

    draw_input_overlay(f, app);
}

fn draw_cpu_section(f: &mut Frame, app: &App, area: Rect) {
    let avg_cpu: f32 = app.cpu_total_usage;

    let cpu_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("CPU Usage (Overall)"),
        )
        .gauge_style(Style::default().fg(Color::Cyan))
        .percent(avg_cpu as u16);

    let cpu_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);

    f.render_widget(cpu_gauge, cpu_chunks[0]);

    let per_core_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(cpu_chunks[1]);

    let cpus = &app.cpu_usage;
    let half = cpus.len().div_ceil(2);

    let build_core_lines = |slice: &[f32], offset: usize| {
        let mut lines = Vec::new();
        for (i, usage) in slice.iter().enumerate() {
            let usage = *usage;
            let global_idx = offset + i;

            let history = app
                .cpu_history
                .get(global_idx)
                .map(|h| &h[..])
                .unwrap_or(&[]);
            let sparkline = if !history.is_empty() {
                generate_sparkline(history, 100.0)
            } else {
                "▁".repeat(20)
            };

            let color = if usage > 80.0 {
                Color::Red
            } else if usage > 50.0 {
                Color::Yellow
            } else {
                Color::Green
            };

            lines.push(Line::from(vec![
                Span::styled(
                    format!("CPU{:2}: ", global_idx),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(format!("{:5.1}%", usage), Style::default().fg(color)),
                Span::raw("  "),
                Span::styled(sparkline, Style::default().fg(Color::Blue)),
            ]));

            lines.push(Line::from(Span::raw(" ")));
        }
        lines
    };

    let left_lines = build_core_lines(&cpus[..half], 0);
    let right_lines = build_core_lines(&cpus[half..], half);

    let left_widget = Paragraph::new(left_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Per-Core Usage (1/2)"),
        )
        .alignment(Alignment::Left);

    let right_widget = Paragraph::new(right_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Per-Core Usage (2/2)"),
        )
        .alignment(Alignment::Left);

    f.render_widget(left_widget, per_core_cols[0]);
    f.render_widget(right_widget, per_core_cols[1]);
}

fn draw_memory_section(f: &mut Frame, app: &App, area: Rect) {
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

    f.render_widget(mem_gauge, area);
}

fn draw_network_section(f: &mut Frame, app: &App, area: Rect) {
    let (total_rx, total_tx) = network::calculate_network_totals(app);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    const MIB: f64 = 1024.0 * 1024.0;
    let rx_points: Vec<(f64, f64)> = app
        .network_history
        .iter()
        .enumerate()
        .map(|(i, &(rx, _))| (i as f64, rx as f64 / MIB))
        .collect();
    let tx_points: Vec<(f64, f64)> = app
        .network_history
        .iter()
        .enumerate()
        .map(|(i, &(_, tx))| (i as f64, tx as f64 / MIB))
        .collect();

    // One shared scale for both lines so their heights are comparable.
    let peak = rx_points
        .iter()
        .chain(tx_points.iter())
        .fold(0.0f64, |m, &(_, v)| m.max(v));

    let dim = Style::default().fg(Color::DarkGray);
    let fmt_label = |v: f64| {
        if v >= 100.0 {
            format!("{v:.0}")
        } else if v >= 10.0 {
            format!("{v:.1}")
        } else if v >= 0.1 {
            format!("{v:.2}")
        } else {
            format!("{v:.3}")
        }
    };
    let (y_max, y_labels) = if peak > 0.0 {
        (
            peak,
            vec![
                Span::styled("0", dim),
                Span::styled(fmt_label(peak / 2.0), dim),
                Span::styled(fmt_label(peak), dim),
            ],
        )
    } else {
        (1.0, vec![Span::styled("0", dim), Span::styled("1", dim)])
    };

    // The title doubles as the legend; the y-axis carries the scale in MiB.
    let title = Line::from(vec![
        Span::raw(" Network History  "),
        Span::styled(
            format!("↓ {:.2}", total_rx as f64 / MIB),
            Style::default().fg(Color::Green),
        ),
        Span::raw("  "),
        Span::styled(
            format!("↑ {:.2}", total_tx as f64 / MIB),
            Style::default().fg(Color::Blue),
        ),
        Span::raw(" MiB"),
    ]);

    let x_max = (app.network_history.len().saturating_sub(1)).max(1) as f64;
    let chart = Chart::new(vec![
        Dataset::default()
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Green))
            .data(&rx_points),
        Dataset::default()
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Blue))
            .data(&tx_points),
    ])
    .block(Block::default().borders(Borders::ALL).title(title))
    .x_axis(Axis::default().bounds([0.0, x_max]))
    .y_axis(Axis::default().bounds([0.0, y_max]).labels(y_labels));

    f.render_widget(chart, cols[0]);

    // Per-interface details
    let net_info: Vec<Line> = network::per_interface_info(app)
        .iter()
        .map(|(name, rx, tx)| {
            Line::from(vec![
                Span::styled(format!("{:12}: ", name), Style::default().fg(Color::Cyan)),
                Span::styled(
                    format!("↓ {:8.2} MiB", rx),
                    Style::default().fg(Color::Green),
                ),
                Span::raw(" / "),
                Span::styled(
                    format!("↑ {:8.2} MiB", tx),
                    Style::default().fg(Color::Blue),
                ),
            ])
        })
        .collect();

    let interfaces = Paragraph::new(net_info)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Per-Interface Stats (last interval)"),
        )
        .alignment(Alignment::Left);

    f.render_widget(interfaces, cols[1]);
}

fn draw_gpu_section(f: &mut Frame, app: &App, area: Rect) {
    if app.gpus.is_empty() {
        let none = Paragraph::new("No GPU detected")
            .block(Block::default().borders(Borders::ALL).title("GPU Usage"));
        f.render_widget(none, area);
        return;
    }

    let n = app.gpus.len() as u16;
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![Constraint::Ratio(1, n as u32); n as usize])
        .split(area);

    for (i, gpu) in app.gpus.iter().enumerate() {
        draw_gpu_chart(f, gpu, cols[i]);
    }
}

/// nvtop-style chart: GPU core % and VRAM % share one 0-100% plot.
fn draw_gpu_chart(f: &mut Frame, gpu: &GpuInfo, area: Rect) {
    let gpu_style = Style::default().fg(Color::Blue);
    let mem_style = Style::default().fg(Color::Green);

    let gpu_points: Vec<(f64, f64)> = gpu
        .history
        .iter()
        .enumerate()
        .map(|(i, (g, _))| (i as f64, f64::from(*g)))
        .collect();
    let mem_points: Vec<(f64, f64)> = gpu
        .history
        .iter()
        .enumerate()
        .map(|(i, (_, m))| (i as f64, f64::from(*m)))
        .collect();

    let mem_pct = if gpu.mem_total > 0 {
        gpu.mem_used as f32 / gpu.mem_total as f32 * 100.0
    } else {
        0.0
    };

    // The title doubles as the legend: value colors match the lines.
    // On narrow terminals drop the MiB breakdown, then the name, so the
    // title never gets cut mid-token.
    let gpu_part = format!("GPU {:5.1}%", gpu.usage);
    let mem_short = if gpu.mem_total > 0 {
        format!("MEM {:5.1}%", mem_pct)
    } else {
        "MEM n/a".to_string()
    };
    let mem_long = if gpu.mem_total > 0 {
        format!(
            "{} ({:.0}/{:.0} MiB)",
            mem_short,
            gpu.mem_used as f64 / 1024.0 / 1024.0,
            gpu.mem_total as f64 / 1024.0 / 1024.0
        )
    } else {
        mem_short.clone()
    };

    let inner = area.width.saturating_sub(2) as usize; // inside the borders
    let fits =
        |name_len: usize, mem: &str| 1 + name_len + 2 + gpu_part.len() + 2 + mem.len() <= inner;
    let (name, mem_part) = if fits(gpu.name.len(), &mem_long) {
        (gpu.name.clone(), mem_long)
    } else if fits(gpu.name.len(), &mem_short) {
        (gpu.name.clone(), mem_short)
    } else {
        (String::new(), mem_short)
    };

    let mut title_spans = vec![Span::raw(" ")];
    if !name.is_empty() {
        title_spans.push(Span::styled(name, Style::default().fg(Color::White)));
        title_spans.push(Span::raw("  "));
    }
    title_spans.push(Span::styled(gpu_part, gpu_style));
    title_spans.push(Span::raw("  "));
    title_spans.push(Span::styled(mem_part, mem_style));
    let title = Line::from(title_spans);

    let mut datasets = vec![
        Dataset::default()
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(gpu_style)
            .data(&gpu_points),
    ];
    if gpu.mem_total > 0 {
        datasets.push(
            Dataset::default()
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(mem_style)
                .data(&mem_points),
        );
    }

    let dim = Style::default().fg(Color::DarkGray);
    let chart = Chart::new(datasets)
        .block(Block::default().borders(Borders::ALL).title(title))
        .x_axis(Axis::default().bounds([0.0, (gpu.history.len().saturating_sub(1)).max(1) as f64]))
        .y_axis(Axis::default().bounds([0.0, 100.0]).labels([
            Span::styled("0", dim),
            Span::styled("50", dim),
            Span::styled("100", dim),
        ]));

    f.render_widget(chart, area);
}
