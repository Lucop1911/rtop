use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
};

use crate::{App, SortColumn};

pub fn draw_processes(f: &mut Frame, app: &mut App, area: Rect) {
    app.table_area = area;

    app.header_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: 3,
    };

    let flat_processes = app.flatten_processes();

    let sort_indicator = if app.reverse_sort { "▼" } else { "▲" };
    let header_cells = vec![
        Cell::from(if app.sort_column == SortColumn::Pid {
            format!("PID {}", sort_indicator)
        } else {
            "PID".to_string()
        }),
        Cell::from(if app.sort_column == SortColumn::Name {
            format!("Name {}", sort_indicator)
        } else {
            "Name".to_string()
        }),
        Cell::from(if app.sort_column == SortColumn::Cpu {
            format!("CPU% {}", sort_indicator)
        } else {
            "CPU%".to_string()
        }),
        Cell::from(if app.sort_column == SortColumn::Memory {
            format!("Memory {}", sort_indicator)
        } else {
            "Memory".to_string()
        }),
    ]
    .into_iter()
    .map(|c| {
        c.style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    });

    let header = Row::new(header_cells).height(1).bottom_margin(1);

    // Calculate visible area
    let visible_rows = area.height.saturating_sub(4) as usize;
    let start = app.viewport_offset.min(flat_processes.len().saturating_sub(1));
    let end = (start + visible_rows).min(flat_processes.len());
    
    // Only render visible rows
    let visible_processes = &flat_processes[start..end];

    let rows = visible_processes.iter().map(|(depth, node)| {
        let indent = "  ".repeat(*depth);
        let prefix = if !node.children.is_empty() {
            if node.expanded { "▼ " } else { "▶ " }
        } else {
            "  "
        };

        let cells = vec![
            Cell::from(node.info.pid.to_string()),
            Cell::from(format!("{}{}{}", indent, prefix, node.info.name)),
            Cell::from(format!("{:.1}", node.info.cpu_usage)),
            Cell::from(format!(
                "{:.1} MB",
                node.info.memory as f64 / 1024.0 / 1024.0
            )),
        ];
        Row::new(cells).height(1)
    });

    let widths = [
        Constraint::Length(10),
        Constraint::Percentage(50),
        Constraint::Length(12),
        Constraint::Length(15),
    ];

    let title = if app.search_mode {
        format!("Processes [Searching: {}]", app.search_query)
    } else {
        format!(
            "Processes ({} shown) - Double-click headers to sort, double-click rows to expand",
            flat_processes.len()
        )
    };

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(title))
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    // Adjust table state to show relative position within viewport
    let mut adjusted_state = TableState::default();
    if let Some(selected) = app.table_state.selected() {
        if selected >= start && selected < end {
            adjusted_state.select(Some(selected - start));
        }
    }

    f.render_stateful_widget(table, area, &mut adjusted_state);
}