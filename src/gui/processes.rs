use chrono::{TimeZone, Utc};
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

    let visible_rows = chunks[0].height.saturating_sub(4) as usize;

    let start = app.viewport_offset;
    let end = (start + visible_rows).min(app.display.len());
    let visible_processes = &app.display[start..end];

    let max_line_num = app.display.len();
    let available_width = chunks[0].width.saturating_sub(4);
    let columns = crate::helpers::columns::TableColumns::new(available_width, max_line_num);
    let name_width = columns.name_width;
    let line_num_width = columns.line_num_width;

    let rows: Vec<Row> = visible_processes
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let actual_idx = start + i;
            let info = &app.processes[row.proc_idx];

            // Flat name, truncated to the column width like htop.
            let max_name_len = name_width as usize;
            let name = if info.name.len() > max_name_len {
                format!("{}...", &info.name[..max_name_len - 3])
            } else {
                info.name.clone()
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
                format!("{}", info.pid),
                name,
                format!("{:.1}%", info.cpu_usage),
                format!("{:.2} MB", info.memory as f64 / 1024.0 / 1024.0),
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
        || !app.search_query.is_empty()
    {
        format!(
            "Processes ({}/{}) [FILTERED]",
            app.display.len(),
            app.processes.len()
        )
    } else {
        format!("Processes ({}/{})", app.display.len(), app.processes.len())
    };

    // column_spacing(0): column x-ranges must match TableColumns::edges
    // exactly so header clicks hit-test the column that is drawn there.
    // Visual gaps come from the cell padding in the constraint widths.
    let table = Table::new(rows, columns.constraints())
        .column_spacing(0)
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

/// Formats the process owner as `name (uid)` from the uid cached in
/// `ProcessInfo` (resolved on selection change / refresh — the draw
/// path never reads `/proc`).
fn resolve_user(app: &App, info: &crate::ProcessInfo) -> String {
    match info.user_id {
        Some(uid) => match app.user_names.get(&uid) {
            Some(name) => format!("{} ({})", name, uid),
            None => uid.to_string(),
        },
        None => "Unknown".to_string(),
    }
}

fn draw_detail_panel(f: &mut Frame, app: &App, area: Rect) {
    f.render_widget(Clear, area);

    let selected_row = app
        .table_state
        .selected()
        .and_then(|idx| app.display.get(idx));

    let content = if let Some(row) = selected_row {
        let info = &app.processes[row.proc_idx];

        let mut lines = vec![
            Line::from(vec![
                Span::styled("PID: ", Style::default().fg(Color::Cyan)),
                Span::styled(format!("{}", info.pid), Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("Name: ", Style::default().fg(Color::Cyan)),
                Span::styled(&info.name, Style::default().fg(Color::White)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("CPU Usage: ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    format!("{:.2}%", info.cpu_usage),
                    Style::default().fg(if info.cpu_usage > 50.0 {
                        Color::Red
                    } else {
                        Color::Green
                    }),
                ),
            ]),
            Line::from(vec![
                Span::styled("Memory: ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    format!("{:.2} MB", info.memory as f64 / 1024.0 / 1024.0),
                    Style::default().fg(Color::White),
                ),
            ]),
        ];

        if let Some((read, write)) = app.detail_io {
            lines.push(Line::from(vec![Span::styled(
                "Process I/O:",
                Style::default().fg(Color::Cyan),
            )]));
            lines.push(Line::from(vec![Span::styled(
                format!("  Read: {:.2} MB", read as f64 / 1024.0 / 1024.0),
                Style::default().fg(Color::White),
            )]));
            lines.push(Line::from(vec![Span::styled(
                format!("  Write: {:.2} MB", write as f64 / 1024.0 / 1024.0),
                Style::default().fg(Color::White),
            )]));
        } else {
            lines.push(Line::from(vec![
                Span::styled("Process I/O: ", Style::default().fg(Color::Cyan)),
                Span::styled("N/A", Style::default().fg(Color::White)),
            ]));
        }

        lines.push(Line::from(vec![
            Span::styled("Virtual Memory: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("{:.2} MB", info.vsize as f64 / 1024.0 / 1024.0),
                Style::default().fg(Color::White),
            ),
        ]));

        lines.push(Line::from(""));

        if info.ppid == 0 {
            lines.push(Line::from(vec![
                Span::styled("Parent process: ", Style::default().fg(Color::Cyan)),
                Span::styled("None", Style::default().fg(Color::White)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled("Parent PID: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{}", info.ppid)),
            ]));
            match app.pid_index.get(&info.ppid) {
                Some(&parent_idx) => lines.push(Line::from(vec![
                    Span::styled("Parent process: ", Style::default().fg(Color::Cyan)),
                    Span::styled(
                        app.processes[parent_idx].name.clone(),
                        Style::default().fg(Color::White),
                    ),
                ])),
                None => lines.push(Line::from(vec![
                    Span::styled("Parent process: ", Style::default().fg(Color::Cyan)),
                    Span::styled("Unknown", Style::default().fg(Color::White)),
                ])),
            }
        }

        lines.push(Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Cyan)),
            Span::styled(&info.status, Style::default().fg(Color::White)),
        ]));

        lines.push(Line::from(vec![
            Span::styled("User: ", Style::default().fg(Color::Cyan)),
            Span::styled(resolve_user(app, info), Style::default().fg(Color::White)),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Threads: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("{}", info.num_threads),
                Style::default().fg(Color::White),
            ),
        ]));

        lines.push(Line::from(""));

        let now = Utc::now().timestamp();
        let run_time = now.saturating_sub(info.start_time as i64).max(0);
        lines.push(Line::from(vec![
            Span::styled("Run Time: ", Style::default().fg(Color::Cyan)),
            Span::styled(format!("{}s", run_time), Style::default().fg(Color::White)),
        ]));

        if let Some(datetime) = Utc.timestamp_opt(info.start_time as i64, 0).single() {
            lines.push(Line::from(vec![
                Span::styled("Start Time: ", Style::default().fg(Color::Cyan)),
                Span::styled(format!("{}", datetime), Style::default().fg(Color::White)),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Command:",
            Style::default().fg(Color::Cyan),
        )));
        let cmd = &app.detail_cmdline;
        let max_width = (area.width.saturating_sub(4)) as usize;
        if cmd.len() > max_width {
            let truncated = format!("{}...", &cmd[..max_width.saturating_sub(3)]);
            lines.push(Line::from(Span::styled(
                truncated,
                Style::default().fg(Color::White),
            )));
        } else if cmd.is_empty() {
            lines.push(Line::from(Span::styled(
                "N/A",
                Style::default().fg(Color::White),
            )))
        } else {
            lines.push(Line::from(Span::styled(
                cmd.clone(),
                Style::default().fg(Color::White),
            )));
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
