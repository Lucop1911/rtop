use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table, Clear},
};

use crate::{App, SortColumn, InputMode};

pub fn draw_processes(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(70),
            Constraint::Percentage(30),
        ])
        .split(area);

    app.table_area = chunks[0];
    
    let flat = app.flatten_processes().clone();
    let visible_rows = chunks[0].height.saturating_sub(4) as usize;
    
    let start = app.viewport_offset;
    let end = (start + visible_rows).min(flat.len());
    let visible_processes = &flat[start..end];

    let max_line_num = flat.len();
    let line_num_width = max_line_num.to_string().len().max(3) as u16;

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
            let name = format!("{}{}{}", indent, expand_indicator, node.info.name);
            
            let is_selected = Some(actual_idx) == app.table_state.selected();
            let style = if is_selected {
                Style::default().bg(Color::DarkGray).fg(Color::White).add_modifier(Modifier::BOLD)
            } else {
                Style::default().bg(Color::Black).fg(Color::White)
            };

            let line_num = format!("{:>width$}", actual_idx + 1, width = line_num_width as usize);

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

    // Headers con indicatori
    let pid_header = get_header_with_indicator("PID", SortColumn::Pid, app);
    let name_header = get_header_with_indicator("Name", SortColumn::Name, app);
    let cpu_header = get_header_with_indicator("CPU%", SortColumn::Cpu, app);
    let mem_header = get_header_with_indicator("Memory", SortColumn::Memory, app);

    let header = Row::new(vec!["#", &pid_header, &name_header, &cpu_header, &mem_header])
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD).bg(Color::Black));

    let title = if app.user_filter.is_some() || app.status_filter.is_some() || 
                  app.cpu_threshold.is_some() || app.memory_threshold.is_some() {
        format!("Processes ({}/{}) [FILTERED]", flat.len(), app.system.processes().len())
    } else {
        format!("Processes ({}/{})", flat.len(), app.system.processes().len())
    };

    let table = Table::new(
        rows,
        [
            Constraint::Length(line_num_width + 1),
            Constraint::Length(10),
            Constraint::Percentage(50),
            Constraint::Length(12),
            Constraint::Length(15),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .style(Style::default().bg(Color::Black))
    )
    .style(Style::default().bg(Color::Black));

    app.header_area = Rect {
        x: chunks[0].x,
        y: chunks[0].y,
        width: chunks[0].width,
        height: 3,
    };

    f.render_widget(table, chunks[0]);
    draw_detail_panel(f, app, chunks[1]);

    // Draw input overlays
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
    
    let selected_node = app.table_state.selected()
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
                    Style::default().fg(if node.info.cpu_usage > 50.0 { Color::Red } else { Color::Green })
                ),
            ]),
            Line::from(vec![
                Span::styled("Memory: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{:.2} MB", node.info.memory as f64 / 1024.0 / 1024.0)),
            ]),
        ];

        if let Some(proc) = process {
            lines.push(Line::from(""));
            
            if let Some(parent_pid) = proc.parent() {
                lines.push(Line::from(vec![
                    Span::styled("Parent PID: ", Style::default().fg(Color::Cyan)),
                    Span::raw(format!("{}", parent_pid.as_u32())),
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

            lines.push(Line::from(vec![
                Span::styled("Virtual Memory: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{:.2} MB", proc.virtual_memory() as f64 / 1024.0 / 1024.0)),
            ]));

            lines.push(Line::from(""));
            if let Some((read, write)) = app.calculate_process_io() {
                lines.push(Line::from(vec![
                    Span::styled("Process I/O:", Style::default().fg(Color::Cyan)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw(format!("  Read: {:.2} MB", read as f64 / 1024.0 / 1024.0)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw(format!("  Write: {:.2} MB", write as f64 / 1024.0 / 1024.0)),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("Process I/O: ", Style::default().fg(Color::Cyan)),
                    Span::styled("N/A", Style::default().fg(Color::White)),
                ]));
            }

            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Run Time: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{}s", proc.run_time())),
            ]));

            lines.push(Line::from(vec![
                Span::styled("Start Time: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{}", proc.start_time())),
            ]));

            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled("Command:", Style::default().fg(Color::Yellow))));
            let cmd_parts: Vec<String> = proc.cmd()
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

            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Children: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{}", node.children.len())),
            ]));
        }
        lines
    } else {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "No process selected",
                Style::default().fg(Color::DarkGray)
            )),
        ]
    };

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Process Details")
                .style(Style::default().fg(Color::White).bg(Color::Black))
        )
        .style(Style::default().fg(Color::White).bg(Color::Black));

    f.render_widget(paragraph, area);
}

fn draw_input_overlay(f: &mut Frame, app: &App) {
    match app.input_mode {
        InputMode::UpdateInterval => {
            let area = centered_rect(60, 20, f.area());
            
            f.render_widget(Clear, area);
            
            let block = Block::default()
                .title("Set Update Interval (ms)")
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Black));
            
            let text = vec![
                Line::from(""),
                Line::from(vec![
                    Span::raw("Enter interval (100-10000 ms): "),
                    Span::styled(&app.input_buffer, Style::default().fg(Color::Yellow)),
                ]),
                Line::from(""),
                Line::from("Press Enter to confirm, Esc to cancel"),
            ];
            
            let paragraph = Paragraph::new(text)
                .block(block)
                .alignment(ratatui::layout::Alignment::Center)
                .style(Style::default().bg(Color::Black));
            
            f.render_widget(paragraph, area);
        }
        InputMode::ConfirmKill => {
            let area = centered_rect(60, 20, f.area());
            
            f.render_widget(Clear, area);
            
            let block = Block::default()
                .title("⚠ Confirm Kill Critical Process")
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Black));
            
            let text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "This appears to be a critical system process!",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
                )),
                Line::from(""),
                Line::from("Are you sure you want to kill this process?"),
                Line::from(""),
                Line::from("Press Y to confirm, N or Esc to cancel"),
            ];
            
            let paragraph = Paragraph::new(text)
                .block(block)
                .alignment(ratatui::layout::Alignment::Center)
                .style(Style::default().bg(Color::Black));
            
            f.render_widget(paragraph, area);
        }
        InputMode::UserFilter => {
            let area = centered_rect(60, 20, f.area());
            
            f.render_widget(Clear, area);
            
            let block = Block::default()
                .title("Filter by User ID")
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Black));
            
            let text = vec![
                Line::from(""),
                Line::from(vec![
                    Span::raw("Enter User ID: "),
                    Span::styled(&app.input_buffer, Style::default().fg(Color::Yellow)),
                ]),
                Line::from(""),
                Line::from("Press Enter to confirm, Esc to cancel"),
            ];
            
            let paragraph = Paragraph::new(text)
                .block(block)
                .alignment(ratatui::layout::Alignment::Center)
                .style(Style::default().bg(Color::Black));
            
            f.render_widget(paragraph, area);
        }
        InputMode::None => {}
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}