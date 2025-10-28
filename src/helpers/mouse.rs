use crate::{App, Page, SortColumn};
use crossterm::event::MouseEventKind;
use std::time::{Duration, Instant};

pub fn handle_mouse(app: &mut App, kind: MouseEventKind, x: u16, y: u16) {
    match kind {
        MouseEventKind::Down(_) => {
            app.handle_mouse_click(x, y);
        }
        MouseEventKind::ScrollDown => {
            app.select_next();
        }
        MouseEventKind::ScrollUp => {
            app.select_prev();
        }
        _ => {}
    }
}

impl App {
    fn handle_mouse_click(&mut self, x: u16, y: u16) -> bool {
        let now = Instant::now();
        let is_double_click = if let Some((last_time, last_x, last_y)) = self.last_click {
            now.duration_since(last_time) < Duration::from_millis(500)
                && last_x.abs_diff(x) <= 2
                && last_y.abs_diff(y) <= 2
        } else {
            false
        };

        self.last_click = Some((now, x, y));

        // Check if clicking on header for sorting
        if self.page == Page::Processes && self.header_area.contains((x, y).into()) {
            let header_y = self.header_area.y + 1; // Account for border
            if y == header_y {
                // Determine which column was clicked based on X position
                let relative_x = x.saturating_sub(self.header_area.x + 1);
                let name_col_start = 10;
                let name_col_width = self.header_area.width.saturating_sub(37);
                let cpu_col_start = name_col_start + name_col_width;
                let mem_col_start = cpu_col_start + 12;

                let new_column = if relative_x < name_col_start {
                    Some(SortColumn::Pid)
                } else if relative_x < cpu_col_start {
                    Some(SortColumn::Name)
                } else if relative_x < mem_col_start {
                    Some(SortColumn::Cpu)
                } else {
                    Some(SortColumn::Memory)
                };

                if let Some(col) = new_column {
                    if is_double_click {
                        if self.sort_column == col {
                            self.reverse_sort = !self.reverse_sort;
                        } else {
                            self.sort_column = col;
                            self.reverse_sort = match col {
                                SortColumn::Cpu | SortColumn::Memory => true,
                                _ => false,
                            };
                        }
                        self.force_refresh();
                    }
                }
                return true;
            }
        }

        // Check if clicking on process rows
        if self.page == Page::Processes && self.table_area.contains((x, y).into()) {
            let row_offset = 3;
            if y >= self.table_area.y + row_offset {
                let clicked_row = (y - self.table_area.y - row_offset) as usize;
                let flat = self.flatten_processes();

                if clicked_row < flat.len() {
                    self.table_state.select(Some(clicked_row));

                    if is_double_click {
                        self.toggle_expand();
                        return true;
                    }
                }
            }
        }

        false
    }
}
