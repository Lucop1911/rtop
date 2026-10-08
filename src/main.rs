mod gui;
mod helpers;

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
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use crate::helpers::{keyboard::handle_key_event, mouse::handle_mouse, ui::ui};

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
enum SortColumn {
    Pid,
    Name,
    Command,
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
    SelectFilter,
    UpdateInterval,
    ConfirmKill,
    UserFilter,
    StatusFilter,
    CpuThreshold,
    MemoryThreshold,
    Error,
}

/// A process as sampled from `/proc` on the last refresh.
///
/// This is the flat source of truth: no tree, no children, no per-node
/// expansion flag. The visible table view is derived from this list in
/// `App::rebuild_display`.
#[derive(Clone)]
struct ProcessInfo {
    pid: u32,
    /// Parent process id
    ppid: u32,
    name: String,
    /// Full command line (arguments joined by spaces); `-` for kernel
    /// threads and zombies, which have no command line.
    command: String,
    /// Human readable state, e.g. "Running", "Sleeping", "Zombie".
    status: String,
    /// CPU usage since the previous sample, in percent of all cores
    /// (100% = the whole machine), like htop with Irix mode off.
    cpu_usage: f32,
    /// Resident set size in bytes.
    memory: u64,
    /// Virtual memory size in bytes.
    vsize: u64,
    /// Effective uid, resolved only while a user filter is active
    /// (`None` otherwise; the detail panel resolves it on demand).
    user_id: Option<u32>,
    /// Process start time as a Unix timestamp (seconds since epoch).
    start_time: u64,
    num_threads: i64,
}

/// One row of the derived table view: an index into the flat list.
#[derive(Clone, Copy)]
struct DisplayRow {
    /// Index into [`App::processes`].
    proc_idx: usize,
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
    page: Page,
    sort_column: SortColumn,
    reverse_sort: bool,
    table_state: TableState,

    // Flat process list (source of truth) and the view derived from it.
    processes: Vec<ProcessInfo>,
    /// pid -> index into `processes`.
    pid_index: HashMap<u32, usize>,
    /// Visible table rows (indices into `processes`, sorted globally by
    /// the current sort column), rebuilt on every sample and on every
    /// filter/sort/search change.
    display: Vec<DisplayRow>,
    /// uid -> user name, loaded once from `/etc/passwd` so filters and
    /// the detail panel can show names instead of raw uid numbers.
    user_names: HashMap<u32, String>,

    // Sampling state for delta-based CPU/network usage.
    /// pid -> total CPU jiffies (utime + stime) at the previous sample.
    prev_proc_jiffies: HashMap<u32, u64>,
    /// Per-core (busy, total) jiffies at the previous sample.
    prev_core_jiffies: Vec<(u64, u64)>,
    /// Aggregate (busy, total) jiffies at the previous sample.
    prev_total_jiffies: (u64, u64),
    /// Interface -> (rx, tx) cumulative bytes at the previous sample.
    prev_net_bytes: HashMap<String, (u64, u64)>,
    /// Wall clock of the previous sample, for percentage math.
    last_sample: Option<Instant>,
    clk_tck: u64,
    page_size: u64,
    /// Boot time as a Unix timestamp, from `/proc/stat`.
    boot_time: u64,

    // Values sampled from /proc, read by the drawing code.
    /// Per-core CPU usage in percent, one entry per logical CPU.
    cpu_usage: Vec<f32>,
    cpu_total_usage: f32,
    mem_used_gb: f64,
    mem_total_gb: f64,
    /// Per-interface bytes received/transmitted since the last sample.
    net_deltas: Vec<(String, u64, u64)>,
    /// Discovered GPUs (amdgpu sysfs + NVML), sampled every refresh.
    gpus: Vec<helpers::gpu::GpuInfo>,

    // Cached details of the selected process for the detail panel, so
    // drawing never reads `/proc` (refreshed on selection change and
    // once per sample).
    detail_cmdline: String,
    detail_io: Option<(u64, u64)>,

    search_mode: bool,
    search_query: String,
    last_update: Instant,
    cpu_history: Vec<Vec<f32>>,
    network_history: Vec<(u64, u64)>,
    table_area: Rect,
    header_area: Rect,
    update_interval: Duration,
    viewport_offset: usize,
    input_mode: InputMode,
    input_buffer: String,
    pending_kill_pid: Option<u32>,
    preferences: Preferences,
    user_filter: Option<String>,
    status_filter: Option<String>,
    cpu_threshold: Option<f32>,
    memory_threshold: Option<u64>,
    refresh: bool,
    errors: Vec<(String, String)>,
}

impl App {
    fn new() -> Self {
        let clk_tck = unsafe { libc::sysconf(libc::_SC_CLK_TCK) }.max(1) as u64;
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) }.max(1) as u64;

        let mut preferences = Self::load_preferences().unwrap_or_default();
        preferences.update_interval_ms = preferences.update_interval_ms.clamp(100, 6000);

        let mut app = Self {
            page: Page::Processes,
            sort_column: preferences.sort_column,
            reverse_sort: preferences.reverse_sort,
            table_state: TableState::default(),
            processes: Vec::new(),
            pid_index: HashMap::new(),
            display: Vec::new(),
            user_names: helpers::utils::load_user_names(),
            prev_proc_jiffies: HashMap::new(),
            prev_core_jiffies: Vec::new(),
            prev_total_jiffies: (0, 0),
            prev_net_bytes: HashMap::new(),
            last_sample: None,
            clk_tck,
            page_size,
            boot_time: 0,
            cpu_usage: Vec::new(),
            cpu_total_usage: 0.0,
            mem_used_gb: 0.0,
            mem_total_gb: 0.0,
            net_deltas: Vec::new(),
            gpus: helpers::gpu::discover(),
            detail_cmdline: String::new(),
            detail_io: None,
            search_mode: false,
            search_query: String::new(),
            last_update: Instant::now(),
            cpu_history: vec![vec![]; 60],
            network_history: vec![(0, 0); 60],
            table_area: Rect::default(),
            header_area: Rect::default(),
            update_interval: Duration::from_millis(preferences.update_interval_ms),
            viewport_offset: 0,
            input_mode: InputMode::None,
            input_buffer: String::new(),
            pending_kill_pid: None,
            preferences,
            user_filter: None,
            status_filter: None,
            cpu_threshold: None,
            memory_threshold: None,
            refresh: true,
            errors: Vec::new(),
        };

        // Two samples a moment apart so CPU percentages are meaningful
        // on the very first frame.
        app.force_refresh();
        thread::sleep(Duration::from_millis(200));
        app.force_refresh();
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
    let should_stop = Arc::new(AtomicBool::new(false));

    // Thread for background refresh
    let app_bg = Arc::clone(&app);
    let stop_flag = Arc::clone(&should_stop);
    let bg_thread = thread::spawn(move || {
        loop {
            if stop_flag.load(Ordering::Relaxed) {
                break;
            }
            let sleep_duration = {
                let mut app = app_bg.lock().unwrap();
                app.refresh();
                app.update_interval
            };

            // Minimal chunks so we can exit faster
            let chunk_size = Duration::from_millis(50);
            let mut remaining = sleep_duration;
            while remaining > Duration::ZERO {
                if stop_flag.load(Ordering::Relaxed) {
                    break;
                }
                let to_sleep = remaining.min(chunk_size);
                thread::sleep(to_sleep);
                remaining = remaining.saturating_sub(to_sleep);
            }
        }
    });

    let res = run_app(&mut terminal, Arc::clone(&app));

    // Send the stop signal to the thread
    should_stop.store(true, Ordering::Relaxed);
    bg_thread.join().ok();

    // Cleanup
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

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: Arc<Mutex<App>>) -> Result<()>
where
    B::Error: std::error::Error + Send + Sync + 'static,
{
    // Redraw only when something actually changed: user input or a new
    // sample from the refresh thread. Drawing on every 50 ms poll wakeup
    // kept the screen rendering 20x per second for nothing (~25% CPU).
    let mut dirty = true;
    let mut drawn_update: Option<Instant> = None;

    loop {
        {
            let mut app_guard = app.lock().unwrap();
            if dirty || drawn_update != Some(app_guard.last_update) {
                terminal.draw(|f| ui(f, &mut app_guard))?;
                drawn_update = Some(app_guard.last_update);
                dirty = false;
            }
        }

        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    let mut app_guard = app.lock().unwrap();
                    if handle_key_event(&mut app_guard, key.code, key.modifiers)? {
                        return Ok(());
                    }
                    dirty = true;
                }
                Event::Mouse(mouse) => {
                    let mut app_guard = app.lock().unwrap();
                    handle_mouse(&mut app_guard, mouse.kind, mouse.column, mouse.row);
                    dirty = true;
                }
                Event::Resize(_, _) => {
                    dirty = true;
                }
                _ => {}
            }
        }
    }
}
