mod helpers;
mod gui;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, TableState},
    Frame, Terminal,
};
use std::{
    collections::HashMap,
    io,
    time::{Duration, Instant},
};
use sysinfo::{Networks, Pid, System, ProcessRefreshKind};

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
    parent: Option<Pid>,
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
}

impl App {
    fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        std::thread::sleep(Duration::from_millis(500));
        system.refresh_cpu_all(); // 1st measurement
        std::thread::sleep(Duration::from_millis(500));
        system.refresh_cpu_all(); // 2nd measurement

        
        // Wait for initial CPU measurements
        std::thread::sleep(Duration::from_millis(1000));
        system.refresh_cpu_all();
        system.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::everything().with_cpu()
        );

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
        };
        
        app.build_process_tree();
        app.table_state.select(Some(0));
        app
    }

    fn update(&mut self) {
        if self.last_update.elapsed() >= Duration::from_millis(1000) {
            self.system.refresh_cpu_all(); // CPU usage update
            self.system.refresh_memory();
            self.system.refresh_processes_specifics(
                sysinfo::ProcessesToUpdate::All,
                true,
                ProcessRefreshKind::everything().with_cpu()
            );

            self.networks.refresh(true);
            
            // Update CPU history
            for (i, cpu) in self.system.cpus().iter().enumerate() {
                if i >= self.cpu_history.len() {
                    self.cpu_history.push(Vec::new());
                }
                self.cpu_history[i].push(cpu.cpu_usage());
                if self.cpu_history[i].len() > 60 {
                    self.cpu_history[i].remove(0);
                }
            }
            
            // Update network history
            let (rx, tx) = self.networks.iter()
                .fold((0, 0), |(rx, tx), (_, net)| {
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

    fn force_refresh(&mut self) {
        self.system.refresh_cpu_all(); // CPU usage update
        self.system.refresh_memory();
        self.system.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::everything().with_cpu()
        );

        self.networks.refresh(true);
        self.build_process_tree();
    }

    fn build_process_tree(&mut self) {
        let mut process_infos: HashMap<Pid, ProcessInfo> = HashMap::new();
        let mut children_map: HashMap<Pid, Vec<Pid>> = HashMap::new();

        // Collect all process information
        for (pid, process) in self.system.processes() {
            let info = ProcessInfo {
                pid: *pid,
                name: process.name().to_string_lossy().to_string(),
                parent: process.parent(),
                cpu_usage: process.cpu_usage(),
                memory: process.memory(),
            };
            process_infos.insert(*pid, info);
            
            // Build parent-child relationships
            if let Some(parent_pid) = process.parent() {
                children_map.entry(parent_pid).or_insert_with(Vec::new).push(*pid);
            }
        }

        // Find root processes (those without parents or whose parents don't exist)
        let mut roots = Vec::new();
        for (pid, info) in &process_infos {
            let is_root = match info.parent {
                None => true,
                Some(parent_pid) => !process_infos.contains_key(&parent_pid),
            };
            
            if is_root {
                roots.push(self.build_node(*pid, &process_infos, &children_map));
            }
        }

        self.sort_processes(&mut roots);
        self.processes = roots;
    }

    fn build_node(&self, pid: Pid, all_processes: &HashMap<Pid, ProcessInfo>, children_map: &HashMap<Pid, Vec<Pid>>) -> ProcessNode {
        let info = all_processes.get(&pid).unwrap().clone();
        let mut children = Vec::new();

        if let Some(child_pids) = children_map.get(&pid) {
            for child_pid in child_pids {
                if all_processes.contains_key(child_pid) {
                    children.push(self.build_node(*child_pid, all_processes, children_map));
                }
            }
        }

        ProcessNode {
            info,
            children,
            expanded: self.expanded_pids.get(&pid).copied().unwrap_or(false),
        }
    }

    fn sort_processes(&self, nodes: &mut Vec<ProcessNode>) {
        nodes.sort_by(|a, b| {
            let ordering = match self.sort_column {
                SortColumn::Pid => a.info.pid.cmp(&b.info.pid),
                SortColumn::Name => a.info.name.cmp(&b.info.name),
                SortColumn::Cpu => a.info.cpu_usage.partial_cmp(&b.info.cpu_usage).unwrap(),
                SortColumn::Memory => a.info.memory.cmp(&b.info.memory),
            };
            if self.reverse_sort {
                ordering.reverse()
            } else {
                ordering
            }
        });

        for node in nodes {
            self.sort_processes(&mut node.children);
        }
    }

    fn flatten_processes(&self) -> Vec<(usize, &ProcessNode)> {
        let mut result = Vec::new();
        for node in &self.processes {
            self.flatten_node(node, 0, &mut result);
        }
        result
    }

    fn flatten_node<'a>(&'a self, node: &'a ProcessNode, depth: usize, result: &mut Vec<(usize, &'a ProcessNode)>) {
        if self.search_mode && !self.search_query.is_empty() {
            if !node.info.name.to_lowercase().contains(&self.search_query.to_lowercase()) {
                return;
            }
        }
        
        result.push((depth, node));
        if node.expanded {
            for child in &node.children {
                self.flatten_node(child, depth + 1, result);
            }
        }
    }

    fn toggle_expand(&mut self) {
        if let Some(selected) = self.table_state.selected() {
            let flat = self.flatten_processes();
            if selected < flat.len() {
                let pid = flat[selected].1.info.pid;
                let expanded = self.expanded_pids.get(&pid).copied().unwrap_or(false);
                self.expanded_pids.insert(pid, !expanded);
            }
        }
    }

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
        
        // Check if clicking on header for sorting
        if self.page == Page::Processes && self.header_area.contains((x, y).into()) {
            let header_y = self.header_area.y + 1; // Account for border
            if y == header_y {
                // Determine which column was clicked based on X position
                let relative_x = x.saturating_sub(self.header_area.x + 1);
                let name_col_start = 10;
                let name_col_width = self.header_area.width.saturating_sub(37);
                let cpu_col_start = name_col_start + name_col_width;
                let mem_col_start = cpu_col_start + 12;
                
                let new_column = if relative_x < name_col_start {
                    Some(SortColumn::Pid)
                } else if relative_x < cpu_col_start {
                    Some(SortColumn::Name)
                } else if relative_x < mem_col_start {
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
                            self.reverse_sort = match col {
                                SortColumn::Cpu | SortColumn::Memory => true,
                                _ => false,
                            };
                        }
                        self.force_refresh();
                    }
                }
                return true;
            }
        }
        
        // Check if clicking on process rows
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

    fn kill_selected(&mut self) -> Result<()> {
        if let Some(selected) = self.table_state.selected() {
            let flat = self.flatten_processes();
            if selected < flat.len() {
                let pid = flat[selected].1.info.pid;
                if let Some(process) = self.system.process(pid) {
                    process.kill();
                }
                // Immediate refresh to show the process is gone
                self.force_refresh();
            }
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app);

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

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        app.update();
        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    if app.search_mode {
                        match key.code {
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
                        match key.code {
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return Ok(()),
                            KeyCode::F(1) => app.page = Page::Processes,
                            KeyCode::F(2) => app.page = Page::SystemStats,
                            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => app.search_mode = true,
                            KeyCode::Char('Q') if key.modifiers.contains(KeyModifiers::SHIFT) => { 
                                app.kill_selected()?; 
                            }
                            KeyCode::Enter => {
                                app.toggle_expand();
                                app.force_refresh();
                            }
                            KeyCode::Down => {
                                let flat = app.flatten_processes();
                                if !flat.is_empty() {
                                    let i = app.table_state.selected().map_or(0, |i| (i + 1).min(flat.len().saturating_sub(1)));
                                    app.table_state.select(Some(i));
                                }
                            }
                            KeyCode::Up => {
                                let i = app.table_state.selected().map_or(0, |i| i.saturating_sub(1));
                                app.table_state.select(Some(i));
                            }
                            KeyCode::Char('p') => {
                                app.sort_column = SortColumn::Pid;
                                app.reverse_sort = !app.reverse_sort;
                                app.force_refresh();
                            }
                            KeyCode::Char('n') => {
                                app.sort_column = SortColumn::Name;
                                app.reverse_sort = !app.reverse_sort;
                                app.force_refresh();
                            }
                            KeyCode::Char('c') => {
                                app.sort_column = SortColumn::Cpu;
                                app.reverse_sort = !app.reverse_sort;
                                app.force_refresh();
                            }
                            KeyCode::Char('m') => {
                                app.sort_column = SortColumn::Memory;
                                app.reverse_sort = !app.reverse_sort;
                                app.force_refresh();
                            }
                            KeyCode::PageDown => {
                                let flat = app.flatten_processes();
                                if !flat.is_empty() {
                                    let i = app.table_state.selected().map_or(0, |i| (i + 10).min(flat.len().saturating_sub(1)));
                                    app.table_state.select(Some(i));
                                }
                            }
                            KeyCode::PageUp => {
                                let i = app.table_state.selected().map_or(0, |i| i.saturating_sub(10));
                                app.table_state.select(Some(i));
                            }
                            _ => {}
                        }
                    }
                }
                Event::Mouse(mouse) => {
                    match mouse.kind {
                        MouseEventKind::Down(_) => {
                            app.handle_mouse_click(mouse.column, mouse.row);
                        }
                        MouseEventKind::ScrollDown => {
                            let flat = app.flatten_processes();
                            if !flat.is_empty() {
                                let i = app.table_state.selected().map_or(0, |i| (i + 1).min(flat.len().saturating_sub(1)));
                                app.table_state.select(Some(i));
                            }
                        }
                        MouseEventKind::ScrollUp => {
                            let i = app.table_state.selected().map_or(0, |i| i.saturating_sub(1));
                            app.table_state.select(Some(i));
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
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

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let text = if app.search_mode {
        vec![Line::from(vec![
            Span::raw("Search: "),
            Span::styled(&app.search_query, Style::default().fg(Color::Yellow)),
            Span::raw(" | ESC: Cancel | Enter: Confirm"),
        ])]
    } else {
        vec![Line::from(vec![
            Span::raw("F1: Processes | F2: Stats | Ctrl+F: Search | Shift+Q: Kill | Enter: Expand | ↑↓: Navigate | Mouse: Click & Scroll | Ctrl+C: Exit"),
        ])]
    };

    let footer = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan));

    f.render_widget(footer, area);
}