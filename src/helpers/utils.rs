use crate::{App, ProcessInfo};
use std::collections::HashMap;

/// Maps uid -> user name from `/etc/passwd`. Best effort: an unreadable
/// file yields an empty map and callers fall back to raw uid numbers.
pub fn load_user_names() -> HashMap<u32, String> {
    let mut map = HashMap::new();
    let Ok(content) = std::fs::read_to_string("/etc/passwd") else {
        return map;
    };
    for line in content.lines() {
        let mut parts = line.split(':');
        let (Some(name), Some(_passwd), Some(uid)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        if let Ok(uid) = uid.parse::<u32>() {
            map.insert(uid, name.to_string());
        }
    }
    map
}

impl App {
    pub fn get_process_at_flat_index(&self, flat_idx: usize) -> Option<&ProcessInfo> {
        let row = self.display.get(flat_idx)?;
        self.processes.get(row.proc_idx)
    }

    pub fn select_next(&mut self) {
        let flat_len = self.display.len();
        if flat_len > 0 {
            let i = self
                .table_state
                .selected()
                .map_or(0, |i| (i + 1).min(flat_len - 1));
            self.table_state.select(Some(i));
            self.ensure_visible(i);
        }
    }

    pub fn select_prev(&mut self) {
        let flat_len = self.display.len();
        if flat_len > 0 {
            let i = self
                .table_state
                .selected()
                .map_or(0, |i| i.saturating_sub(1));
            self.table_state.select(Some(i));
            self.ensure_visible(i);
        }
    }

    pub fn ensure_visible(&mut self, index: usize) {
        let visible_rows = self.table_area.height.saturating_sub(4) as usize;

        if index < self.viewport_offset {
            self.viewport_offset = index;
        } else if visible_rows > 0 && index >= self.viewport_offset + visible_rows {
            self.viewport_offset = (index + 1).saturating_sub(visible_rows);
        }
        self.refresh_selected_details();
    }

    /// Caches everything the detail panel shows about the selected
    /// process (uid, command line, I/O counters) so the draw path never
    /// reads `/proc`. Called on selection changes and once per sample:
    /// at most a handful of small file reads per keypress or tick.
    pub fn refresh_selected_details(&mut self) {
        let Some(pid) = self
            .table_state
            .selected()
            .and_then(|i| self.display.get(i))
            .map(|row| self.processes[row.proc_idx].pid)
        else {
            self.detail_cmdline.clear();
            self.detail_io = None;
            return;
        };
        let Some(&idx) = self.pid_index.get(&pid) else {
            self.detail_cmdline.clear();
            self.detail_io = None;
            return;
        };

        if self.processes[idx].user_id.is_none() {
            self.processes[idx].user_id = procfs2::proc::Process::new(pid)
                .ok()
                .and_then(|p| p.status().ok())
                .map(|status| status.uid.effective);
        }

        match procfs2::proc::Process::new(pid).ok() {
            Some(process) => {
                self.detail_cmdline = process
                    .cmdline()
                    .map(|args| {
                        args.iter()
                            .map(|s| s.to_string_lossy().into_owned())
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .unwrap_or_default();
                self.detail_io = process
                    .io()
                    .ok()
                    .map(|io| (io.read_bytes.0, io.write_bytes.0));
            }
            None => {
                self.detail_cmdline.clear();
                self.detail_io = None;
            }
        }
    }

    pub fn go_to_top(&mut self) {
        self.table_state.select(Some(0));
        self.viewport_offset = 0;
        self.refresh_selected_details();
    }

    pub fn go_to_bottom(&mut self) {
        let flat_len = self.display.len();

        if flat_len > 0 {
            let last_idx = flat_len - 1;
            self.table_state.select(Some(last_idx));
            let visible_rows = self.table_area.height.saturating_sub(4) as usize;
            self.viewport_offset = if visible_rows > 0 {
                (last_idx + 1).saturating_sub(visible_rows)
            } else {
                0
            };
            self.refresh_selected_details();
        }
    }

    pub fn page_down(&mut self) {
        let flat_len = self.display.len();

        if flat_len > 0 {
            let visible_rows = self.table_area.height.saturating_sub(4) as usize;
            let current = self.table_state.selected().unwrap_or(0);
            let new_idx = (current + visible_rows).min(flat_len - 1);
            self.table_state.select(Some(new_idx));
            self.ensure_visible(new_idx);
        }
    }

    pub fn page_up(&mut self) {
        let flat_len = self.display.len();

        if flat_len > 0 {
            let visible_rows = self.table_area.height.saturating_sub(4) as usize;
            let current = self.table_state.selected().unwrap_or(0);
            let new_idx = current.saturating_sub(visible_rows);
            self.table_state.select(Some(new_idx));
            self.ensure_visible(new_idx);
        }
    }

    pub fn select_first_matching(&mut self) {
        if !self.display.is_empty() {
            self.table_state.select(Some(0));
            self.viewport_offset = 0;
            self.refresh_selected_details();
        } else {
            self.table_state.select(None);
        }
    }

    pub fn clear_filters(&mut self) {
        self.user_filter = None;
        self.status_filter = None;
        self.cpu_threshold = None;
        self.memory_threshold = None;
        self.search_query.clear();
        self.rebuild_display();
    }
}

/// Renders `data` as block sparkline characters scaled against `max`
/// rather than against the window's own peak, so bar heights stay
/// meaningful across samples and rows. Values are clamped to `max`.
/// Truncates `s` to at most `max` bytes, appending `...` when cut.
/// Never splits a UTF-8 sequence (command lines with non-ASCII
/// arguments would otherwise panic on a byte slice).
pub fn truncate_ellipsis(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    if max <= 3 {
        let mut out = String::new();
        for (i, ch) in s.char_indices() {
            if i + ch.len_utf8() > max {
                break;
            }
            out.push(ch);
        }
        return out;
    }
    let mut end = max - 3;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &s[..end])
}

pub fn generate_sparkline(data: &[f32], max: f32) -> String {
    if data.is_empty() {
        return String::new();
    }

    let chars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

    if max <= 0.0 {
        return "▁".repeat(data.len());
    }

    data.iter()
        .map(|&val| {
            let ratio = (val / max).clamp(0.0, 1.0);
            let normalized = (ratio * (chars.len() - 1) as f32) as usize;
            chars[normalized]
        })
        .collect()
}

pub fn detect_terminal() -> Option<&'static str> {
    let candidates = [
        "kitty",
        "alacritty",
        "gnome-terminal",
        "konsole",
        "xfce4-terminal",
        "xterm",
        "lxterminal",
        "urxvt",
        "ghostty",
        "foot",
        "wezterm",
        "terminator",
        "tilix",
        "st",
        "qterminal",
        "sakura",
        "eterm",
        "aterm",
        "mlterm",
        "yakuake",
        "guake",
        "tilda",
        "warp-terminal",
        "rio",
        "termite",
    ];

    candidates
        .iter()
        .find(|&term| which::which(term).is_ok())
        .map(|v| v as _)
}
