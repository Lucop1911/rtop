use crate::{App, Page, SortColumn};
use crossterm::event::MouseEventKind;

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
        // Sort on header click: clicking a column sorts by it, clicking
        // the active column inverts the direction (htop behaviour).
        if self.page == Page::Processes && self.header_area.contains((x, y).into()) {
            let header_y = self.header_area.y + 1;
            if y == header_y {
                let relative_x = x.saturating_sub(self.header_area.x + 1);
                let columns = crate::helpers::columns::TableColumns::new(
                    self.table_area.width.saturating_sub(4),
                    self.display.len(),
                );

                if let Some(col) = columns.column_at(relative_x) {
                    if self.sort_column == col {
                        self.reverse_sort = !self.reverse_sort;
                    } else {
                        self.sort_column = col;
                        self.reverse_sort = matches!(col, SortColumn::Cpu | SortColumn::Memory);
                    }
                    self.preferences.sort_column = self.sort_column;
                    self.preferences.reverse_sort = self.reverse_sort;
                    self.rebuild_display();
                }

                return true;
            }
        }

        if self.page == Page::Processes && self.table_area.contains((x, y).into()) {
            let row_offset = 3;
            if y > self.table_area.y + row_offset {
                let clicked_row = (y - self.table_area.y - row_offset + 1) as usize;
                let actual_index = self.viewport_offset + clicked_row;

                if actual_index < self.display.len() {
                    self.table_state.select(Some(actual_index));
                    self.refresh_selected_details();
                }
            }
        }

        false
    }
}
