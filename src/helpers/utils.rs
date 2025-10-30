use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
use crate::{App, ProcessNode};

impl App {

    pub fn force_refresh(&mut self) {
        self.system.refresh_cpu_all();
        self.system.refresh_memory();
        let mut system = System::new_all();

        system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing()
                .with_cpu()
                .with_memory(),
        );

        self.networks.refresh(true);
        self.build_process_tree();
    }

    pub fn flatten_processes(&self) -> Vec<(usize, &ProcessNode)> {
        let mut result = Vec::with_capacity(self.processes.len() * 2); // Estimate
        for node in &self.processes {
            self.flatten_node(node, 0, &mut result);
        }
        result
    }

    pub fn flatten_node<'a>(
        &'a self,
        node: &'a ProcessNode,
        depth: usize,
        result: &mut Vec<(usize, &'a ProcessNode)>,
    ) {
        // Apply search filter
        if self.search_mode && !self.search_query.is_empty() {
            let query_lower = self.search_query.to_lowercase();
            let name_lower = node.info.name.to_lowercase();
            let pid_str = node.info.pid.to_string();
            
            // Search by name or PID
            if !name_lower.contains(&query_lower) && !pid_str.contains(&self.search_query) {
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

    pub fn toggle_expand(&mut self) {
        if let Some(selected) = self.table_state.selected() {
            let flat = self.flatten_processes();
            if selected < flat.len() {
                let node = flat[selected].1;
                // Only toggle if process has children
                if !node.children.is_empty() {
                    let pid = node.info.pid;
                    let expanded = self.expanded_pids.get(&pid).copied().unwrap_or(false);
                    self.expanded_pids.insert(pid, !expanded);
                }
            }
        }
    }

    pub fn select_next(&mut self) {
        let flat = self.flatten_processes();
        if !flat.is_empty() {
            let i = self
                .table_state
                .selected()
                .map_or(0, |i| (i + 1).min(flat.len() - 1));
            self.table_state.select(Some(i));
            self.ensure_visible(i);
        }
    }

    pub fn select_prev(&mut self) {
        if !self.flatten_processes().is_empty() {
            let i = self
                .table_state
                .selected()
                .map_or(0, |i| i.saturating_sub(1));
            self.table_state.select(Some(i));
            self.ensure_visible(i);
        }
    }

    // Ensure the selected item is visible in the viewport
    pub fn ensure_visible(&mut self, index: usize) {
        let visible_rows = self.table_area.height.saturating_sub(4) as usize;
        
        // If selected item is above viewport, scroll up
        if index < self.viewport_offset {
            self.viewport_offset = index;
        }
        // If selected item is below viewport, scroll down
        else if index >= self.viewport_offset + visible_rows {
            self.viewport_offset = index.saturating_sub(visible_rows - 1);
        }
    }

    // Go to top - resets viewport
    pub fn go_to_top(&mut self) {
        self.table_state.select(Some(0));
        self.viewport_offset = 0;
    }

    // Go to bottom - scrolls viewport to bottom
    pub fn go_to_bottom(&mut self) {
        let flat = self.flatten_processes();
        if !flat.is_empty() {
            let last_idx = flat.len() - 1;
            self.table_state.select(Some(last_idx));
            let visible_rows = self.table_area.height.saturating_sub(4) as usize;
            self.viewport_offset = last_idx.saturating_sub(visible_rows - 1);
        }
    }

    // Remember selected process PID
    pub fn remember_selection(&mut self) {
        self.selected_pid = self.table_state.selected().and_then(|idx| {
            let flat = self.flatten_processes();
            flat.get(idx).map(|(_, node)| node.info.pid)
        });
    }

    // Restore selection after rebuild - find the PID but don't change viewport
    pub fn restore_selection(&mut self) {
        if let Some(pid) = self.selected_pid {
            let flat = self.flatten_processes();
            if let Some(new_idx) = flat.iter().position(|(_, node)| node.info.pid == pid) {
                // Update selection index without forcing viewport to follow
                self.table_state.select(Some(new_idx));
            } else if !flat.is_empty() {
                // Process no longer exists, select first item and reset viewport
                self.table_state.select(Some(0));
                self.viewport_offset = 0;
                self.selected_pid = None;
            }
        }
    }
}