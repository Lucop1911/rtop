// src/helpers/keyboard.rs
use crate::App;
use crossterm::event::{KeyCode, KeyModifiers};
use anyhow::Result;

pub fn handle_key_event(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> Result<bool> {
    if app.search_mode {
        match code {
            KeyCode::Esc => {
                app.search_mode = false;
                app.search_query.clear();
            }
            KeyCode::Enter => {
                app.search_mode = false;
            }
            KeyCode::Char(c) => {
                app.search_query.push(c);
            }
            KeyCode::Backspace => {
                app.search_query.pop();
            }
            _ => {}
        }
    } else {
        match code {
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => return Ok(true),
            KeyCode::F(1) => app.page = crate::Page::Processes,
            KeyCode::F(2) => app.page = crate::Page::SystemStats,
            KeyCode::Char('f') if modifiers.contains(KeyModifiers::CONTROL) => app.search_mode = true,
            KeyCode::Char('Q') if modifiers.contains(KeyModifiers::SHIFT) => { app.kill_selected()?; }
            KeyCode::Enter => { app.toggle_expand(); app.force_refresh(); }
            KeyCode::Down => app.select_next(),
            KeyCode::Up => app.select_prev(),
            KeyCode::Char('p') => { app.sort_column = crate::SortColumn::Pid; app.reverse_sort = !app.reverse_sort; app.force_refresh(); }
            KeyCode::Char('n') => { app.sort_column = crate::SortColumn::Name; app.reverse_sort = !app.reverse_sort; app.force_refresh(); }
            KeyCode::Char('c') => { app.sort_column = crate::SortColumn::Cpu; app.reverse_sort = !app.reverse_sort; app.force_refresh(); }
            KeyCode::Char('m') => { app.sort_column = crate::SortColumn::Memory; app.reverse_sort = !app.reverse_sort; app.force_refresh(); }
            KeyCode::PageDown => app.page_down(),
            KeyCode::PageUp => app.page_up(),
            _ => {}
        }
    }
    Ok(false)
}
