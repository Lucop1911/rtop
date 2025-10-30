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
use std::{
    collections::HashMap,
    io,
    time::{Duration, Instant},
    sync::{Arc, Mutex},
    thread,
};
use sysinfo::{Networks, Pid, System};

#[derive(PartialEq, Clone, Copy)]
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
}

#[derive(Clone)]
struct ProcessInfo {
    pid: Pid,
    name: String,
    cpu_usage: f32,
    memory: u64,
}

struct ProcessNode {
    info: ProcessInfo,
    children: Vec<ProcessNode>,
    expanded: bool,
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
    network_history: Vec<(u64, u64)>,
    table_area: Rect,
    last_click: Option<(Instant, u16, u16)>,
    header_area: Rect,
    update_interval: Duration,
    viewport_offset: usize,
    cached_flat_processes: Option<Vec<(usize, usize)>>,
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

        let mut app = Self {
            system,
            networks,
            page: Page::Processes,
            sort_column: SortColumn::Cpu,
            reverse_sort: true,
            table_state: TableState::default(),
            processes: Vec::new(),
            expanded_pids: HashMap::new(),
            search_mode: false,
            search_query: String::new(),
            last_update: Instant::now(),
            cpu_history: vec![vec![]; 60],
            network_history: vec![(0, 0); 60],
            table_area: Rect::default(),
            last_click: None,
            header_area: Rect::default(),
            update_interval: Duration::from_millis(1000),
            viewport_offset: 0,
            cached_flat_processes: None,
        };

        app.build_process_tree();
        app.table_state.select(Some(0));
        app
    }

    fn refresh(&mut self) {
        self.system.refresh_cpu_all();
        self.system.refresh_memory();

        use sysinfo::{ProcessRefreshKind, ProcessesToUpdate};
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing()
                .with_cpu()
                .with_memory(),
        );

        self.networks.refresh(true);

        // Aggiorna CPU history
        for (i, cpu) in self.system.cpus().iter().enumerate() {
            if i >= self.cpu_history.len() {
                self.cpu_history.push(Vec::new());
            }
            self.cpu_history[i].push(cpu.cpu_usage());
            if self.cpu_history[i].len() > 60 {
                self.cpu_history[i].remove(0);
            }
        }

        // Aggiorna network history
        let (rx, tx) = self.networks.iter().fold((0, 0), |(rx, tx), (_, net)| {
            (rx + net.received(), tx + net.transmitted())
        });
        self.network_history.push((rx, tx));
        if self.network_history.len() > 60 {
            self.network_history.remove(0);
        }

        self.last_update = Instant::now();
        self.build_process_tree();
    }
}

fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = Arc::new(Mutex::new(App::new()));

    // Uso un thread per miglior performance
    let app_bg = Arc::clone(&app);
    thread::spawn(move || {
        loop {
            // lock, refresh, get interval, unlock, then sleep
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