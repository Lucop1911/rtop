use crate::{App, Page, SortColumn};
use crossterm::event::MouseEventKind;
use ratatui::layout::Constraint;
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

        // Controllo click sull'header
        if self.page == Page::Processes && self.header_area.contains((x, y).into()) {
            let header_y = self.header_area.y + 1;
            if y == header_y {
                let table_width = self.header_area.width.saturating_sub(2);
                let relative_x = x.saturating_sub(self.header_area.x + 1);

                // Stessi constraints di draw_processes.rs
                let constraints = [
                    Constraint::Length(10),
                    Constraint::Percentage(50),
                    Constraint::Length(12),
                    Constraint::Length(15),
                ];

                // Spazio colonna -> caratteri
                let mut widths = vec![];
                let mut remaining = table_width;
                for c in constraints {
                    let w = match c {
                        Constraint::Length(n) => n,
                        Constraint::Percentage(p) => (table_width * p as u16 / 100).max(1),
                        _ => 1,
                    };
                    widths.push(w.min(remaining));
                    remaining = remaining.saturating_sub(w);
                }

                let mut col_edges = Vec::new();
                let mut start = 0;
                for w in &widths {
                    col_edges.push((start, start + *w));
                    start += *w;
                }

                // Posizione -> range
                let new_column = if relative_x < col_edges[0].1 {
                    Some(SortColumn::Pid)
                } else if relative_x < col_edges[1].1 {
                    Some(SortColumn::Name)
                } else if relative_x < col_edges[2].1 {
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
                            self.reverse_sort = matches!(col, SortColumn::Cpu | SortColumn::Memory);
                        }
                        self.force_refresh();
                    }
                }

                return true;
            }
        }

        // Controllo click su una riga
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
