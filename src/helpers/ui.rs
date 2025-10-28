use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
};

use crate::{
    App, Page,
    gui::{footer::draw_footer, processes::draw_processes, stats::draw_stats},
};

pub fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)].as_ref())
        .split(f.area());

    match app.page {
        Page::Processes => draw_processes(f, app, chunks[0]),
        Page::SystemStats => draw_stats(f, app, chunks[0]),
    }

    draw_footer(f, app, chunks[1]);
}
