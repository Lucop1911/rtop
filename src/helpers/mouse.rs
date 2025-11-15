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

        // Controllo click sull header
        if self.page == Page::Processes && self.header_area.contains((x, y).into()) {
            let header_y = self.header_area.y + 1;
            if y == header_y {
                let _table_width = self.header_area.width.saturating_sub(2);
                let relative_x = x.saturating_sub(self.header_area.x + 1);

                // Calculate dynamic column widths (same logic as in draw_processes)
                let available_width = self.table_area.width.saturating_sub(4);
                
                let max_line_num = self.flatten_processes().len();
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

                let mut col_edges = Vec::new();
                let mut start = 0u16;
                
                // Numero riga
                let line_col_width = line_num_width + 1;
                col_edges.push((start, start + line_col_width));
                start += line_col_width;
                
                // PID
                col_edges.push((start, start + pid_width));
                start += pid_width;
                
                // Name
                col_edges.push((start, start + name_width));
                start += name_width;
                
                // CPU
                col_edges.push((start, start + cpu_width));
                start += cpu_width;
                
                // Memory
                col_edges.push((start, start + mem_width));

                let new_column = if relative_x < col_edges[0].1 {
                    None
                } else if relative_x < col_edges[1].1 {
                    Some(SortColumn::Pid)
                } else if relative_x < col_edges[2].1 {
                    Some(SortColumn::Name)
                } else if relative_x < col_edges[3].1 {
                    Some(SortColumn::Cpu)
                } else if relative_x < col_edges[4].1 {
                    Some(SortColumn::Memory)
                } else {
                    None
                };

                if let Some(col) = new_column {
                    if is_double_click {
                        if self.sort_column == col {
                            self.reverse_sort = !self.reverse_sort;
                        } else {
                            self.sort_column = col;
                            self.reverse_sort = matches!(col, SortColumn::Cpu | SortColumn::Memory);
                        }
                        if self.refresh {self.force_refresh()}
                    }
                }

                return true;
            }
        }

        if self.page == Page::Processes && self.table_area.contains((x, y).into()) {
            let row_offset = 3;
            if y > self.table_area.y + row_offset {
                let clicked_row = (y - self.table_area.y - row_offset + 1) as usize;
                let actual_index = self.viewport_offset + clicked_row;
                let flat = self.flatten_processes();

                if actual_index < flat.len() {
                    self.table_state.select(Some(actual_index));

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