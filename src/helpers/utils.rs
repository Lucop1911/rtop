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
        // Invalidate cache after refresh
        self.cached_flat_processes = None;
    }

    pub fn flatten_processes(&mut self) -> &Vec<(usize, usize)> {
        // Return cached result if available
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
        // Apply search filter
        if !self.search_query.is_empty() {
            let query_lower = self.search_query.to_lowercase();
            let name_lower = node.info.name.to_lowercase();
            let pid_str = node.info.pid.to_string();
            
            // Search by name or PID
            if !name_lower.contains(&query_lower) && !pid_str.contains(&self.search_query) {
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
                // Only toggle if process has children
                if !node.children.is_empty() {
                    let pid = node.info.pid;
                    let expanded = self.expanded_pids.get(&pid).copied().unwrap_or(false);
                    self.expanded_pids.insert(pid, !expanded);
                    // Invalidate cache when expanding/collapsing
                    self.cached_flat_processes = None;
                    self.force_refresh();
                }
            }
        }
    }

    pub fn select_next(&mut self) {
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

    pub fn select_prev(&mut self) {
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
        let flat_len = self.flatten_processes().len();
        if flat_len > 0 {
            let last_idx = flat_len - 1;
            self.table_state.select(Some(last_idx));
            let visible_rows = self.table_area.height.saturating_sub(4) as usize;
            self.viewport_offset = last_idx.saturating_sub(visible_rows - 1);
        }
    }

    // Page down
    pub fn page_down(&mut self) {
        let flat_len = self.flatten_processes().len();
        if flat_len > 0 {
            let visible_rows = self.table_area.height.saturating_sub(4) as usize;
            let current = self.table_state.selected().unwrap_or(0);
            let new_idx = (current + visible_rows).min(flat_len - 1);
            self.table_state.select(Some(new_idx));
            self.ensure_visible(new_idx);
        }
    }

    // Page up
    pub fn page_up(&mut self) {
        let flat_len = self.flatten_processes().len();
        if flat_len > 0 {
            let visible_rows = self.table_area.height.saturating_sub(4) as usize;
            let current = self.table_state.selected().unwrap_or(0);
            let new_idx = current.saturating_sub(visible_rows);
            self.table_state.select(Some(new_idx));
            self.ensure_visible(new_idx);
        }
    }

    // Select first matching process in search results
    pub fn select_first_matching(&mut self) {
        let flat = self.flatten_processes();
        if !flat.is_empty() {
            // First matching process is at index 0 after filtering
            self.table_state.select(Some(0));
            self.viewport_offset = 0;
        } else {
            // No matches
            self.table_state.select(None);
        }
    }

    // Remember selected line position (not PID)
    pub fn remember_selection(&mut self) {
        // We don't need to remember anything - selection stays at the same line number
    }

    // Restore selection after rebuild - keep same line number
    pub fn restore_selection(&mut self) {
        let flat_len = self.flatten_processes().len();
        if flat_len > 0 {
            // Keep the current selection index, but clamp it to valid range
            if let Some(current_idx) = self.table_state.selected() {
                if current_idx >= flat_len {
                    // If the selected line no longer exists (list got shorter), select last item
                    self.table_state.select(Some(flat_len - 1));
                }
                // Otherwise keep the same line number selected
            } else {
                // No selection, select first item
                self.table_state.select(Some(0));
            }
        } else {
            // No processes in list
            self.table_state.select(None);
        }
    }
}