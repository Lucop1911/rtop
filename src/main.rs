mod gui;
mod helpers;

use crate::helpers::keyboard::handle_key_event;
use crate::helpers::mouse::handle_mouse;
use crate::helpers::ui::ui;
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
    layout::Rect,
    widgets::TableState,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io,
    time::{Duration, Instant},
    sync::{Arc, Mutex},
    thread,
};
use sysinfo::{Networks, Pid, System};

#[derive(PartialEq, Clone, Copy, Serialize, Deserialize)]
enum SortColumn {
    Pid,
    Name,
    Cpu,
    Memory,
}

#[derive(PartialEq)]
enum Page {
    Processes,
    SystemStats,
    Help,
}

#[derive(PartialEq)]
enum InputMode {
    None,
    UpdateInterval,
    ConfirmKill,
    UserFilter,
}

#[derive(Clone)]
struct ProcessInfo {
    pid: Pid,
    name: String,
    cpu_usage: f32,
    memory: u64,
    user_id: Option<u32>,
    status: String,
}

struct ProcessNode {
    info: ProcessInfo,
    children: Vec<ProcessNode>,
    expanded: bool,
}

#[derive(Serialize, Deserialize, Clone)]
struct Preferences {
    update_interval_ms: u64,
    sort_column: SortColumn,
    reverse_sort: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            update_interval_ms: 1000,
            sort_column: SortColumn::Cpu,
            reverse_sort: true,
        }
    }
}

struct App {
    system: System,
    networks: Networks,
    page: Page,
    sort_column: SortColumn,
    reverse_sort: bool,
    table_state: TableState,
    processes: Vec<ProcessNode>,
    expanded_pids: HashMap<Pid, bool>,
    search_mode: bool,
    search_query: String,
    last_update: Instant,
    cpu_history: Vec<Vec<f32>>,
    memory_history: Vec<f64>,
    network_history: Vec<(u64, u64)>,
    table_area: Rect,
    last_click: Option<(Instant, u16, u16)>,
    header_area: Rect,
    update_interval: Duration,
    viewport_offset: usize,
    cached_flat_processes: Option<Vec<(usize, usize)>>,
    input_mode: InputMode,
    input_buffer: String,
    pending_kill_pid: Option<Pid>,
    preferences: Preferences,
    user_filter: Option<String>,
    status_filter: Option<String>,
    cpu_threshold: Option<f32>,
    memory_threshold: Option<u64>,
}

impl App {
    fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        std::thread::sleep(Duration::from_millis(200));
        system.refresh_cpu_all();
        std::thread::sleep(Duration::from_millis(200));
        system.refresh_cpu_all();

        let networks = Networks::new_with_refreshed_list();

        // Load preferences
        let preferences = Self::load_preferences().unwrap_or_default();

        let mut app = Self {
            system,
            networks,
            page: Page::Processes,
            sort_column: preferences.sort_column,
            reverse_sort: preferences.reverse_sort,
            table_state: TableState::default(),
            processes: Vec::new(),
            expanded_pids: HashMap::new(),
            search_mode: false,
            search_query: String::new(),
            last_update: Instant::now(),
            cpu_history: vec![vec![]; 60],
            memory_history: Vec::new(),
            network_history: vec![(0, 0); 60],
            table_area: Rect::default(),
            last_click: None,
            header_area: Rect::default(),
            update_interval: Duration::from_millis(preferences.update_interval_ms),
            viewport_offset: 0,
            cached_flat_processes: None,
            input_mode: InputMode::None,
            input_buffer: String::new(),
            pending_kill_pid: None,
            preferences,
            user_filter: None,
            status_filter: None,
            cpu_threshold: None,
            memory_threshold: None,
        };

        app.build_process_tree();
        app.table_state.select(Some(0));
        app
    }
}

fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = Arc::new(Mutex::new(App::new()));

    // Thread per il refresh in background
    let app_bg = Arc::clone(&app);
    thread::spawn(move || {
        loop {
            let sleep_duration = {
                let mut app = app_bg.lock().unwrap();
                app.refresh();
                app.update_interval
            };
            thread::sleep(sleep_duration);
        }
    });

    let res = run_app(&mut terminal, Arc::clone(&app));

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: Arc<Mutex<App>>,
) -> Result<()> {
    loop {
        {
            let mut app_guard = app.lock().unwrap();
            terminal.draw(|f| ui(f, &mut app_guard))?;
        }
        
        if event::poll(Duration::from_millis(50))? { 
            match event::read()? {
                Event::Key(key) => {
                    let mut app_guard = app.lock().unwrap();
                    if handle_key_event(&mut app_guard, key.code, key.modifiers)? {
                        return Ok(());
                    }
                }
                Event::Mouse(mouse) => {
                    let mut app_guard = app.lock().unwrap();
                    handle_mouse(&mut app_guard, mouse.kind, mouse.column, mouse.row);
                }
                Event::Resize(_, _) => {
                    let mut app_guard = app.lock().unwrap();
                    terminal.draw(|f| ui(f, &mut app_guard))?;
                }
                _ => {}
            }
        }
    }
}