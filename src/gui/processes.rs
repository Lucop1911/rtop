use chrono::{DateTime, TimeZone, Utc};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Row, Table},
};

use crate::{App, SortColumn, gui::overlay::draw_input_overlay};

pub fn draw_processes(f: &mut Frame, app: &mut App, area: Rect) {
    let min_width_needed = 10 + 10 + 20 + 12 + 15; // line# + PID + Name(min) + CPU + Memory

    let (table_percent, detail_percent) = if area.width < min_width_needed + 30 {
        if area.width < min_width_needed {
            (100, 0)
        } else {
            let table_width = min_width_needed;
            let table_pct = ((table_width as f32 / area.width as f32) * 100.0) as u16;
            (table_pct.min(100), 100u16.saturating_sub(table_pct))
        }
    } else {
        (70, 30)
    };

    let chunks = if detail_percent == 0 {
        vec![area]
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(table_percent),
                Constraint::Percentage(detail_percent),
            ])
            .split(area)
            .to_vec()
    };

    app.table_area = chunks[0];

    let flat = app.flatten_processes().clone();
    let visible_rows = chunks[0].height.saturating_sub(4) as usize;

    let start = app.viewport_offset;
    let end = (start + visible_rows).min(flat.len());
    let visible_processes = &flat[start..end];

    let max_line_num = flat.len();
    let line_num_width = max_line_num.to_string().len().max(3) as u16;

    let available_width = chunks[0].width.saturating_sub(4);

    let pid_width = 10u16;
    let cpu_width = 12u16;
    let mem_width = 15u16;
    let fixed_total = line_num_width + 1 + pid_width + cpu_width + mem_width;

    let name_width = if available_width > fixed_total {
        available_width.saturating_sub(fixed_total).max(10)
    } else {
        10
    };

    let rows: Vec<Row> = visible_processes
        .iter()
        .enumerate()
        .map(|(i, (depth, _))| {
            let actual_idx = start + i;
            let node = app.get_process_at_flat_index(actual_idx).unwrap();

            let indent = "  ".repeat(*depth);
            let expand_indicator = if !node.children.is_empty() {
                if node.expanded { "▼ " } else { "▶ " }
            } else {
                "  "
            };
            let name_raw = format!("{}{}{}", indent, expand_indicator, node.info.name);

            let max_name_len = name_width.saturating_sub(3) as usize;
            let name = if name_raw.len() > max_name_len {
                format!("{}...", &name_raw[..max_name_len.saturating_sub(3)])
            } else {
                name_raw
            };

            let is_selected = Some(actual_idx) == app.table_state.selected();
            let style = if is_selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let line_num = format!(
                "{:>width$}",
                actual_idx + 1,
                width = line_num_width as usize
            );

            Row::new(vec![
                line_num,
                format!("{}", node.info.pid.as_u32()),
                name,
                format!("{:.1}%", node.info.cpu_usage),
                format!("{:.2} MB", node.info.memory as f64 / 1024.0 / 1024.0),
            ])
            .style(style)
        })
        .collect();

    let pid_header = get_header_with_indicator("PID", SortColumn::Pid, app);
    let name_header = get_header_with_indicator("Name", SortColumn::Name, app);
    let cpu_header = get_header_with_indicator("CPU%", SortColumn::Cpu, app);
    let mem_header = get_header_with_indicator("Memory", SortColumn::Memory, app);

    let header = Row::new(vec![
        "#",
        &pid_header,
        &name_header,
        &cpu_header,
        &mem_header,
    ])
    .style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );

    let title = if app.user_filter.is_some()
        || app.status_filter.is_some()
        || app.cpu_threshold.is_some()
        || app.memory_threshold.is_some()
    {
        format!(
            "Processes ({}/{}) [FILTERED]",
            flat.len(),
            app.system.processes().len()
        )
    } else {
        format!(
            "Processes ({}/{})",
            flat.len(),
            app.system.processes().len()
        )
    };

    let table = Table::new(
        rows,
        [
            Constraint::Length(line_num_width + 1),
            Constraint::Length(pid_width),
            Constraint::Length(name_width),
            Constraint::Length(cpu_width),
            Constraint::Length(mem_width),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .style(Style::default()),
    )
    .style(Style::default());

    app.header_area = Rect {
        x: chunks[0].x,
        y: chunks[0].y,
        width: chunks[0].width,
        height: 3,
    };

    f.render_widget(table, chunks[0]);

    if chunks.len() > 1 && detail_percent > 0 {
        draw_detail_panel(f, app, chunks[1]);
    }

    draw_input_overlay(f, app);
}

fn get_header_with_indicator(name: &str, column: SortColumn, app: &App) -> String {
    if app.sort_column == column {
        let arrow = if app.reverse_sort { "↓" } else { "↑" };
        format!("{} {}", name, arrow)
    } else {
        name.to_string()
    }
}

fn draw_detail_panel(f: &mut Frame, app: &App, area: Rect) {
    f.render_widget(Clear, area);

    let selected_node = app
        .table_state
        .selected()
        .and_then(|idx| app.get_process_at_flat_index(idx));

    let content = if let Some(node) = selected_node {
        let process = app.system.process(node.info.pid);

        let mut lines = vec![
            Line::from(vec![
                Span::styled("PID: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{}", node.info.pid.as_u32())),
            ]),
            Line::from(vec![
                Span::styled("Name: ", Style::default().fg(Color::Cyan)),
                Span::raw(&node.info.name),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("CPU Usage: ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    format!("{:.2}%", node.info.cpu_usage),
                    Style::default().fg(if node.info.cpu_usage > 50.0 {
                        Color::Red
                    } else {
                        Color::Green
                    }),
                ),
            ]),
            Line::from(vec![
                Span::styled("Memory: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!(
                    "{:.2} MB",
                    node.info.memory as f64 / 1024.0 / 1024.0
                )),
            ]),
        ];

        if let Some((read, write)) = app.calculate_process_io() {
            lines.push(Line::from(vec![Span::styled(
                "Process I/O:",
                Style::default().fg(Color::Cyan),
            )]));
            lines.push(Line::from(vec![Span::raw(format!(
                "  Read: {:.2} MB",
                read as f64 / 1024.0 / 1024.0
            ))]));
            lines.push(Line::from(vec![Span::raw(format!(
                "  Write: {:.2} MB",
                write as f64 / 1024.0 / 1024.0
            ))]));
        } else {
            lines.push(Line::from(vec![
                Span::styled("Process I/O: ", Style::default().fg(Color::Cyan)),
                Span::styled("N/A", Style::default().fg(Color::White)),
            ]));
        }
        
        if let Some(proc) = process {
            lines.push(Line::from(vec![
                Span::styled("Virtual Memory: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!(
                    "{:.2} MB",
                    proc.virtual_memory() as f64 / 1024.0 / 1024.0
                )),
            ]));

        
            lines.push(Line::from(""));

            if let Some(parent_pid) = proc.parent() {
                lines.push(Line::from(vec![
                    Span::styled("Parent PID: ", Style::default().fg(Color::Cyan)),
                    Span::raw(format!("{}", parent_pid.as_u32())),
                ]));
            }

            if let Some(parent_pid) = proc.parent() {
                if let Some(parent_proc) = app.system.process(parent_pid) {
                    lines.push(Line::from(vec![
                        Span::styled("Parent process: ", Style::default().fg(Color::Cyan)),
                        Span::raw(format!(
                            "{}",
                            parent_proc.name().to_string_lossy().to_string()
                        )),
                    ]));
                } else {
                    lines.push(Line::from(vec![
                        Span::styled("Parent process: ", Style::default().fg(Color::Cyan)),
                        Span::styled("Unknown", Style::default().fg(Color::Cyan)),
                    ]));
                }
            } else {
                lines.push(Line::from(vec![
                    Span::styled("Parent process: ", Style::default().fg(Color::Cyan)),
                    Span::styled("None", Style::default().fg(Color::Cyan)),
                ]));
            }

            lines.push(Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{:?}", proc.status())),
            ]));

            if let Some(uid) = node.info.user_id {
                lines.push(Line::from(vec![
                    Span::styled("User ID: ", Style::default().fg(Color::Cyan)),
                    Span::raw(format!("{}", uid)),
                ]));
            }

            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Run Time: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{}s", proc.run_time())),
            ]));

            let datetime: DateTime<Utc> = Utc
                .timestamp_opt(proc.start_time() as i64, 0)
                .single()
                .expect("Invalid timestamp");

            lines.push(Line::from(vec![
                Span::styled("Start Time: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{}", datetime)),
            ]));

            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Children: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{}", node.children.len())),
            ]));

            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Command:",
                Style::default().fg(Color::Yellow),
            )));
            let cmd_parts: Vec<String> = proc
                .cmd()
                .iter()
                .map(|s| s.to_string_lossy().to_string())
                .collect();
            let cmd = cmd_parts.join(" ");
            let max_width = (area.width.saturating_sub(4)) as usize;
            if cmd.len() > max_width {
                let truncated = format!("{}...", &cmd[..max_width.saturating_sub(3)]);
                lines.push(Line::from(truncated));
            } else {
                lines.push(Line::from(cmd));
            }
        }
        lines
    } else {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "No process selected",
                Style::default().fg(Color::DarkGray),
            )),
        ]
    };

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Process Details")
                .style(Style::default().fg(Color::White)),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(paragraph, area);
}
