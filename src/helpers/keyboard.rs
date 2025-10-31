use crate::App;
use anyhow::{Ok, Result};
use crossterm::event::{KeyCode, KeyModifiers};
use std::time::Duration;

pub fn handle_key_event(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> Result<bool> {
    if app.search_mode {
        match code {
            KeyCode::Esc => {
                app.search_mode = false;
                app.search_query.clear();
                app.cached_flat_processes = None;  // Invalida la cache
                app.force_refresh();
            }
            KeyCode::Enter => {
                app.search_mode = false;
                app.select_first_matching(); // Becca il primo match
            }
            KeyCode::Char(c) => {
                app.search_query.push(c);
                app.cached_flat_processes = None;  // Invalida la cache
                app.force_refresh();
                app.select_first_matching();
            }
            KeyCode::Backspace => {
                app.search_query.pop();
                app.cached_flat_processes = None;  // Invalida la cache
                app.force_refresh();
                // Auto seleziona primo match quanto l'utente elimina caratteri
                app.select_first_matching();
            }
            // Navigazione durante la ricerca
            KeyCode::Down => {
                app.select_next();
            }
            KeyCode::Up => {
                app.select_prev();
            }
            _ => {}
        }
    } else {
        match code {
            KeyCode::Up => {
                app.select_prev();
            }
            KeyCode::Down => {
                app.select_next();
            }
            KeyCode::Char('c') | KeyCode::Char('C') if modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(true);
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                return Ok(true)
            },
            KeyCode::Esc => {
                if app.page != crate::Page::Help {
                    return Ok(true);
                } else {
                    app.page = crate::Page::Processes;
                }
            }
            KeyCode::F(1) | KeyCode::Char('1') => {
                app.page = crate::Page::Processes;
            }
            KeyCode::F(2) | KeyCode::Char('2') => {
                app.page = crate::Page::SystemStats;
            }
            KeyCode::F(3) | KeyCode::Char('3') | KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Char('?') => {
                app.page = crate::Page::Help
            }
            KeyCode::Char('f') | KeyCode::Char('F') if modifiers.contains(KeyModifiers::CONTROL) => {
                app.search_mode = true;
            }
            KeyCode::Char('/') => {
                app.search_mode = true;
            }
            KeyCode::Char('k') | KeyCode::Char('K') | KeyCode::Delete => {
                app.kill_selected()?;
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                app.force_refresh();
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                app.toggle_expand();
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                app.go_to_top();
            }
            KeyCode::Char('b') | KeyCode::Char('B') => {
                app.go_to_bottom();
            }
            KeyCode::Char('p') => {
                app.sort_column = crate::SortColumn::Pid;
                app.reverse_sort = !app.reverse_sort;
                app.force_refresh();
            }
            KeyCode::Char('n') => {
                app.sort_column = crate::SortColumn::Name;
                app.reverse_sort = !app.reverse_sort;
                app.force_refresh();
            }
            KeyCode::Char('c') => {
                app.sort_column = crate::SortColumn::Cpu;
                app.reverse_sort = !app.reverse_sort;
                app.force_refresh();
            }
            KeyCode::Char('m') => {
                app.sort_column = crate::SortColumn::Memory;
                app.reverse_sort = !app.reverse_sort;
                app.force_refresh();
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                // Increase update frequency
                let new_interval = app.update_interval.saturating_sub(Duration::from_millis(100));
                app.update_interval = new_interval.max(Duration::from_millis(100));
            }
            KeyCode::Char('-') | KeyCode::Char('_') => {
                // Decrease update frequency
                let new_interval = app.update_interval + Duration::from_millis(100);
                app.update_interval = new_interval.min(Duration::from_millis(5000));
            }
            KeyCode::PageDown => {
                app.page_down();
            }
            KeyCode::PageUp => {
                app.page_up();
            }
            _ => {}
        }
    }
    Ok(false)
}