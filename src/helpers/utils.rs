use crate::{App, ProcessNode};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, UpdateKind};

impl App {
    pub fn force_refresh(&mut self) {
        self.system.refresh_cpu_all();
        self.system.refresh_memory();

        self.system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing()
                .with_cpu()
                .with_memory()
                .with_user(UpdateKind::Always),
        );

        self.networks.refresh(true);
        self.build_process_tree();
        self.cached_flat_processes = None;
    }

    pub fn flatten_processes(&mut self) -> &Vec<(usize, usize)> {
        if self.cached_flat_processes.is_none() {
            let mut result = Vec::with_capacity(self.processes.len() * 2);
            for (idx, node) in self.processes.iter().enumerate() {
                self.flatten_node_indexed(idx, node, 0, &mut result);
            }
            self.cached_flat_processes = Some(result);
        }
        self.cached_flat_processes.as_ref().unwrap()
    }

    fn flatten_node_indexed(
        &self,
        node_idx: usize,
        node: &ProcessNode,
        depth: usize,
        result: &mut Vec<(usize, usize)>,
    ) {
        // Filtri
        if !self.search_query.is_empty() {
            let query_lower = self.search_query.to_lowercase();
            let name_lower = node.info.name.to_lowercase();
            let pid_str = node.info.pid.to_string();

            if !name_lower.contains(&query_lower) && !pid_str.contains(&self.search_query) {
                return;
            }
        }

        if let Some(ref user_filter) = self.user_filter {
            if let Some(uid) = node.info.user_id {
                if !uid.to_string().contains(user_filter) {
                    return;
                }
            } else {
                return;
            }
        }

        if let Some(ref status_filter) = self.status_filter {
            if !node.info.status.to_lowercase().contains(&status_filter.to_lowercase()) {
                return;
            }
        }

        if let Some(threshold) = self.cpu_threshold {
            if node.info.cpu_usage < threshold {
                return;
            }
        }

        if let Some(threshold) = self.memory_threshold {
            if node.info.memory < threshold {
                return;
            }
        }

        result.push((depth, node_idx));
        if node.expanded {
            for (child_idx, child) in node.children.iter().enumerate() {
                self.flatten_node_indexed(child_idx, child, depth + 1, result);
            }
        }
    }

    pub fn get_process_at_flat_index(&self, flat_idx: usize) -> Option<&ProcessNode> {
        if let Some(ref cached) = self.cached_flat_processes {
            if flat_idx < cached.len() {
                let (_, node_idx) = cached[flat_idx];
                return self.processes.get(node_idx);
            }
        }
        None
    }

    pub fn toggle_expand(&mut self) {
        if let Some(selected) = self.table_state.selected() {
            if let Some(node) = self.get_process_at_flat_index(selected) {
                if !node.children.is_empty() {
                    let pid = node.info.pid;
                    let expanded = self.expanded_pids.get(&pid).copied().unwrap_or(false);
                    self.expanded_pids.insert(pid, !expanded);
                    self.cached_flat_processes = None;
                    self.force_refresh();
                }
            }
        }
    }

    pub fn select_next(&mut self) {
        if let Some(ref cached) = self.cached_flat_processes {
            let flat_len = cached.len();
            if flat_len > 0 {
                let i = self
                    .table_state
                    .selected()
                    .map_or(0, |i| (i + 1).min(flat_len - 1));
                self.table_state.select(Some(i));
                self.ensure_visible(i);
            }
        } else {
            // Buildo la cache solo se necessario
            let flat_len = self.flatten_processes().len();
            if flat_len > 0 {
                let i = self
                    .table_state
                    .selected()
                    .map_or(0, |i| (i + 1).min(flat_len - 1));
                self.table_state.select(Some(i));
                self.ensure_visible(i);
            }
        }
    }

    pub fn select_prev(&mut self) {
        if let Some(ref cached) = self.cached_flat_processes {
            let flat_len = cached.len();
            if flat_len > 0 {
                let i = self
                    .table_state
                    .selected()
                    .map_or(0, |i| i.saturating_sub(1));
                self.table_state.select(Some(i));
                self.ensure_visible(i);
            }
        } else {
            // Build cache only if needed
            let flat_len = self.flatten_processes().len();
            if flat_len > 0 {
                let i = self
                    .table_state
                    .selected()
                    .map_or(0, |i| i.saturating_sub(1));
                self.table_state.select(Some(i));
                self.ensure_visible(i);
            }
        }
    }

    pub fn ensure_visible(&mut self, index: usize) {
        let visible_rows = self.table_area.height.saturating_sub(4) as usize;

        if index < self.viewport_offset {
            self.viewport_offset = index;
        } else if index >= self.viewport_offset + visible_rows {
            self.viewport_offset = index.saturating_sub(visible_rows - 1);
        }
    }

    pub fn go_to_top(&mut self) {
        self.table_state.select(Some(0));
        self.viewport_offset = 0;
    }

    pub fn go_to_bottom(&mut self) {
        let flat_len = if let Some(ref cached) = self.cached_flat_processes {
            cached.len()
        } else {
            self.flatten_processes().len()
        };

        if flat_len > 0 {
            let last_idx = flat_len - 1;
            self.table_state.select(Some(last_idx));
            let visible_rows = self.table_area.height.saturating_sub(4) as usize;
            self.viewport_offset = last_idx.saturating_sub(visible_rows - 1);
        }
    }

    pub fn page_down(&mut self) {
        let flat_len = if let Some(ref cached) = self.cached_flat_processes {
            cached.len()
        } else {
            self.flatten_processes().len()
        };

        if flat_len > 0 {
            let visible_rows = self.table_area.height.saturating_sub(4) as usize;
            let current = self.table_state.selected().unwrap_or(0);
            let new_idx = (current + visible_rows).min(flat_len - 1);
            self.table_state.select(Some(new_idx));
            self.ensure_visible(new_idx);
        }
    }

    pub fn page_up(&mut self) {
        let flat_len = if let Some(ref cached) = self.cached_flat_processes {
            cached.len()
        } else {
            self.flatten_processes().len()
        };

        if flat_len > 0 {
            let visible_rows = self.table_area.height.saturating_sub(4) as usize;
            let current = self.table_state.selected().unwrap_or(0);
            let new_idx = current.saturating_sub(visible_rows);
            self.table_state.select(Some(new_idx));
            self.ensure_visible(new_idx);
        }
    }

    pub fn select_first_matching(&mut self) {
        let flat = self.flatten_processes();
        if !flat.is_empty() {
            self.table_state.select(Some(0));
            self.viewport_offset = 0;
        } else {
            self.table_state.select(None);
        }
    }

    pub fn clear_filters(&mut self) {
        self.user_filter = None;
        self.status_filter = None;
        self.cpu_threshold = None;
        self.memory_threshold = None;
        self.cached_flat_processes = None;
        self.force_refresh();
    }
}

pub fn calculate_avg_cpu(app: &App) -> f32 {
    app.system.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>() / app.system.cpus().len() as f32
}

pub fn generate_sparkline(data: &[f32]) -> String {
    if data.is_empty() {
        return String::new();
    }

    let chars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let max = data.iter().cloned().fold(0.0f32, f32::max);

    if max == 0.0 {
        return "▁".repeat(data.len());
    }

    data.iter()
        .map(|&val| {
            let normalized = (val / max * (chars.len() - 1) as f32) as usize;
            chars[normalized]
        })
        .collect()
}

pub fn generate_sparkline_with_max(data: &[f32], max_value: f32) -> String {
    if data.is_empty() {
        return String::new();
    }

    let chars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

    if max_value == 0.0 {
        return "▁".repeat(data.len());
    }

    data.iter()
        .map(|&val| {
            let normalized = ((val / max_value) * (chars.len() - 1) as f32) as usize;
            chars[normalized.min(chars.len() - 1)]
        })
        .collect()
}
