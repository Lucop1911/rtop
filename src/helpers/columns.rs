use crate::SortColumn;
use ratatui::layout::Constraint;

/// Column layout of the process table, shared by the renderer and the
/// mouse handler so header hit-testing always matches what is drawn.
///
/// All widths are computed once per use from the same inputs; nothing
/// is stored between frames.
pub struct TableColumns {
    pub line_num_width: u16,
    pub pid_width: u16,
    pub name_width: u16,
    pub cpu_width: u16,
    pub mem_width: u16,
    /// `(start, end)` x-ranges relative to the table's inner left edge,
    /// one per column in draw order: line#, PID, Name, CPU, Memory.
    edges: [(u16, u16); 5],
}

impl TableColumns {
    /// `available_width` is the usable width inside the block borders,
    /// `max_line_num` the number of rows in the table (widest line
    /// number).
    pub fn new(available_width: u16, max_line_num: usize) -> Self {
        let line_num_width = max_line_num.to_string().len().max(3) as u16;
        let pid_width = 10u16;
        let cpu_width = 12u16;
        let mem_width = 15u16;
        let fixed_total = line_num_width + 1 + pid_width + cpu_width + mem_width;
        let name_width = if available_width > fixed_total {
            available_width.saturating_sub(fixed_total).max(10)
        } else {
            10
        };

        let mut edges = [(0u16, 0u16); 5];
        let mut start = 0u16;
        for (i, width) in [
            line_num_width + 1,
            pid_width,
            name_width,
            cpu_width,
            mem_width,
        ]
        .iter()
        .enumerate()
        {
            edges[i] = (start, start + width);
            start += width;
        }

        Self {
            line_num_width,
            pid_width,
            name_width,
            cpu_width,
            mem_width,
            edges,
        }
    }

    pub fn constraints(&self) -> [Constraint; 5] {
        [
            Constraint::Length(self.line_num_width + 1),
            Constraint::Length(self.pid_width),
            Constraint::Length(self.name_width),
            Constraint::Length(self.cpu_width),
            Constraint::Length(self.mem_width),
        ]
    }

    /// Sort column for a pointer position `relative_x` (relative to the
    /// table's inner left edge). `None` on the line-number column or
    /// outside the table.
    pub fn column_at(&self, relative_x: u16) -> Option<SortColumn> {
        for (i, &(start, end)) in self.edges.iter().enumerate().skip(1) {
            if relative_x >= start && relative_x < end {
                return Some(match i {
                    1 => SortColumn::Pid,
                    2 => SortColumn::Name,
                    3 => SortColumn::Cpu,
                    _ => SortColumn::Memory,
                });
            }
        }
        None
    }
}
