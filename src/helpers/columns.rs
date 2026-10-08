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
    /// Width of the Command column; `0` when the table is too narrow
    /// and the column is hidden entirely.
    pub cmd_width: u16,
    pub cpu_width: u16,
    pub mem_width: u16,
    /// `(start, end)` x-ranges relative to the table's inner left edge,
    /// one per column in draw order: line#, PID, Name, Command, CPU,
    /// Memory. The Command range is empty while the column is hidden.
    edges: [(u16, u16); 6],
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
        let leftover = available_width.saturating_sub(fixed_total);

        // Command joins the table only when the leftover is wide
        // enough to show something useful; otherwise Name takes all
        // the leftover space (as it always did) and Command disappears
        // entirely. The cap on Name matches its real content: comm is
        // at most 15 characters plus an ellipsis.
        const NAME_CAP: u16 = 20;
        const MIN_CMD: u16 = 15;
        let (name_width, cmd_width) = if leftover >= NAME_CAP + MIN_CMD {
            (NAME_CAP, leftover - NAME_CAP)
        } else {
            (leftover.max(10), 0)
        };

        let mut edges = [(0u16, 0u16); 6];
        let mut start = 0u16;
        for (i, width) in [
            line_num_width + 1,
            pid_width,
            name_width,
            cmd_width,
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
            cmd_width,
            cpu_width,
            mem_width,
            edges,
        }
    }

    pub fn constraints(&self) -> [Constraint; 6] {
        [
            Constraint::Length(self.line_num_width + 1),
            Constraint::Length(self.pid_width),
            Constraint::Length(self.name_width),
            Constraint::Length(self.cmd_width),
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
                    3 => SortColumn::Command,
                    4 => SortColumn::Cpu,
                    _ => SortColumn::Memory,
                });
            }
        }
        None
    }
}
