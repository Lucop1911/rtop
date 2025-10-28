// src/helpers/mouse.rs
use crate::App;
use crossterm::event::MouseEventKind;

pub fn handle_mouse(app: &mut App, kind: MouseEventKind, x: u16, y: u16) {
    match kind {
        MouseEventKind::Down(_) => { app.handle_mouse_click(x, y); }
        MouseEventKind::ScrollDown => { app.select_next(); }
        MouseEventKind::ScrollUp => { app.select_prev(); }
        _ => {}
    }
}
