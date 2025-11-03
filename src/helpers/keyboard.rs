use crate::{App, InputMode};
use anyhow::{Ok, Result};
use crossterm::event::{KeyCode, KeyModifiers};
use std::time::Duration;

pub fn handle_key_event(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> Result<bool> {
    // Handle input modes first
    match app.input_mode {
        InputMode::UpdateInterval => {
            return handle_update_interval_input(app, code);
        }
        InputMode::ConfirmKill => {
            return handle_confirm_kill(app, code);
        }
        InputMode::UserFilter => {
            return handle_user_filter_input(app, code);
        }
        InputMode::None => {}
    }

    if app.search_mode {
        match code {
            KeyCode::Esc => {
                app.search_mode = false;
                app.search_query.clear();
                app.cached_flat_processes = None;
                app.force_refresh();
            }
            KeyCode::Enter => {
                app.search_mode = false;
                app.select_first_matching();
            }
            KeyCode::Char(c) => {
                app.search_query.push(c);
                app.cached_flat_processes = None;
                app.force_refresh();
                app.select_first_matching();
            }
            KeyCode::Backspace => {
                app.search_query.pop();
                app.cached_flat_processes = None;
                app.force_refresh();
                app.select_first_matching();
            }
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
                app.save_preferences().ok();
                return Ok(true);
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.save_preferences().ok();
                return Ok(true)
            },
            KeyCode::Esc => {
                if app.page != crate::Page::Help {
                    app.save_preferences().ok();
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
                app.initiate_kill()?;
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                app.force_refresh();
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                app.suspend_process()?;
            }
            KeyCode::Char('u') | KeyCode::Char('U') => {
                app.resume_process()?;
            }
            KeyCode::Char('i') | KeyCode::Char('I') => {
                app.input_mode = InputMode::UpdateInterval;
                app.input_buffer = app.update_interval.as_millis().to_string();
            }
            KeyCode::Char('l') | KeyCode::Char('L') => {
                app.clear_filters();
            }
            KeyCode::Char('w') | KeyCode::Char('W') => {
                app.input_mode = InputMode::UserFilter;
                app.input_buffer.clear();
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
                app.preferences.sort_column = app.sort_column;
                app.preferences.reverse_sort = app.reverse_sort;
                app.force_refresh();
            }
            KeyCode::Char('n') => {
                app.sort_column = crate::SortColumn::Name;
                app.reverse_sort = !app.reverse_sort;
                app.preferences.sort_column = app.sort_column;
                app.preferences.reverse_sort = app.reverse_sort;
                app.force_refresh();
            }
            KeyCode::Char('c') => {
                app.sort_column = crate::SortColumn::Cpu;
                app.reverse_sort = !app.reverse_sort;
                app.preferences.sort_column = app.sort_column;
                app.preferences.reverse_sort = app.reverse_sort;
                app.force_refresh();
            }
            KeyCode::Char('m') => {
                app.sort_column = crate::SortColumn::Memory;
                app.reverse_sort = !app.reverse_sort;
                app.preferences.sort_column = app.sort_column;
                app.preferences.reverse_sort = app.reverse_sort;
                app.force_refresh();
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                let new_interval = app.update_interval.saturating_sub(Duration::from_millis(100));
                app.update_interval = new_interval.max(Duration::from_millis(100));
                app.preferences.update_interval_ms = app.update_interval.as_millis() as u64;
            }
            KeyCode::Char('-') | KeyCode::Char('_') => {
                let new_interval = app.update_interval + Duration::from_millis(100);
                app.update_interval = new_interval.min(Duration::from_millis(10000));
                app.preferences.update_interval_ms = app.update_interval.as_millis() as u64;
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

fn handle_update_interval_input(app: &mut App, code: KeyCode) -> Result<bool> {
    match code {
        KeyCode::Enter => {
            if let std::result::Result::Ok(ms) = app.input_buffer.parse::<u64>() {
                let ms = ms.clamp(100, 10000);
                app.update_interval = Duration::from_millis(ms);
                app.preferences.update_interval_ms = ms;
                app.save_preferences().ok();
            }
            app.input_mode = InputMode::None;
            app.input_buffer.clear();
        }
        KeyCode::Esc => {
            app.input_mode = InputMode::None;
            app.input_buffer.clear();
        }
        KeyCode::Char(c) if c.is_ascii_digit() => {
            app.input_buffer.push(c);
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        _ => {}
    }
    Ok(false)
}

fn handle_confirm_kill(app: &mut App, code: KeyCode) -> Result<bool> {
    match code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            if let Some(pid) = app.pending_kill_pid {
                if let Some(process) = app.system.process(pid) {
                    process.kill();
                }
                app.force_refresh();
            }
            app.input_mode = InputMode::None;
            app.pending_kill_pid = None;
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.input_mode = InputMode::None;
            app.pending_kill_pid = None;
        }
        _ => {}
    }
    Ok(false)
}

fn handle_user_filter_input(app: &mut App, code: KeyCode) -> Result<bool> {
    match code {
        KeyCode::Enter => {
            if app.input_buffer.is_empty() {
                app.user_filter = None;
            } else {
                app.user_filter = Some(app.input_buffer.clone());
            }
            app.input_mode = InputMode::None;
            app.input_buffer.clear();
            app.cached_flat_processes = None;
            app.force_refresh();
        }
        KeyCode::Esc => {
            app.input_mode = InputMode::None;
            app.input_buffer.clear();
        }
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        _ => {}
    }
    Ok(false)
}