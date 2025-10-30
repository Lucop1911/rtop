use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table},
};

use crate::App;

pub fn draw_processes(f: &mut Frame, app: &mut App, area: Rect) {
    // Split the area into main table and detail panel
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(70),  // Process table
            Constraint::Percentage(30),  // Detail panel
        ])
        .split(area);

    app.table_area = chunks[0];
    
    let flat = app.flatten_processes().clone();
    let visible_rows = chunks[0].height.saturating_sub(4) as usize;
    
    // Range visibile
    let start = app.viewport_offset;
    let end = (start + visible_rows).min(flat.len());
    let visible_processes = &flat[start..end];

    // Spazio per il line number 
    let max_line_num = flat.len();
    let line_num_width = max_line_num.to_string().len().max(3) as u16;

    let rows: Vec<Row> = visible_processes
        .iter()
        .enumerate()
        .map(|(i, (depth, _))| {
            let actual_idx = start + i;
            let node = app.get_process_at_flat_index(actual_idx).unwrap();
            
            // Indicatori espansione nodo
            let indent = "  ".repeat(*depth);
            let expand_indicator = if !node.children.is_empty() {
                if node.expanded { "▼ " } else { "▶ " }
            } else {
                "  "
            };
            let name = format!("{}{}{}", indent, expand_indicator, node.info.name);
            
            let is_selected = Some(actual_idx) == app.table_state.selected();
            let style = if is_selected {
                Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            // Numero linea nella colonna 1
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

    let header = Row::new(vec!["#", "PID", "Name", "CPU%", "Memory"])
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

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
            .title(format!("Processes ({}/{})", flat.len(), app.system.processes().len())),
    );

    // Salvo la zona dell'header per gestire i click del mouse dopo
    app.header_area = Rect {
        x: chunks[0].x,
        y: chunks[0].y,
        width: chunks[0].width,
        height: 3,
    };

    f.render_widget(table, chunks[0]);
    draw_detail_panel(f, app, chunks[1]);
}

fn draw_detail_panel(f: &mut Frame, app: &App, area: Rect) {
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
            
            // PID genitore
            if let Some(parent_pid) = proc.parent() {
                lines.push(Line::from(vec![
                    Span::styled("Parent PID: ", Style::default().fg(Color::Cyan)),
                    Span::raw(format!("{}", parent_pid.as_u32())),
                ]));
            }

            // Stato
            lines.push(Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{:?}", proc.status())),
            ]));

            // V RAM
            lines.push(Line::from(vec![
                Span::styled("Virtual Memory: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{:.2} MB", proc.virtual_memory() as f64 / 1024.0 / 1024.0)),
            ]));

            // Disco
            let disk_usage = proc.disk_usage();
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Disk Read: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{:.2} MB", disk_usage.read_bytes as f64 / 1024.0 / 1024.0)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("Disk Write: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{:.2} MB", disk_usage.written_bytes as f64 / 1024.0 / 1024.0)),
            ]));

            // Run time
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Run Time: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{}s", proc.run_time())),
            ]));

            // Start time
            lines.push(Line::from(vec![
                Span::styled("Start Time: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{}", proc.start_time())),
            ]));

            // Command line (truncated)
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

            // Numero children 
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
                .style(Style::default().fg(Color::White))
        )
        .style(Style::default());

    f.render_widget(paragraph, area);
}