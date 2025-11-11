use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, UpdateKind};
use crate::{App, ProcessNode};

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

    pub fn flatten_processes(&mut self) -> &Vec<(usize, Vec<usize>)> {
        if self.cached_flat_processes.is_none() {
            let mut result = Vec::with_capacity(self.processes.len() * 2);
            for (idx, node) in self.processes.iter().enumerate() {
                let mut path = vec![idx];
                self.flatten_node_with_path(node, 0, &mut path, &mut result);
            }
            self.cached_flat_processes = Some(result);
        }
        self.cached_flat_processes.as_ref().unwrap()
    }

    fn flatten_node_with_path(
        &self,
        node: &ProcessNode,
        depth: usize,
        path: &mut Vec<usize>,
        result: &mut Vec<(usize, Vec<usize>)>,
    ) {
        let (node_matches, has_matching_children) = self.check_node_and_children_match(node);
        
        // Skippo il subtree se ne il nodo ne il processo figlio hanno un match
        if !node_matches && !has_matching_children {
            return;
        }

        // Aggiungo il nodo al risultato per dare contesto
        result.push((depth, path.clone()));
        
        // Se il nodo è expanded appiattischo tutti i processi figli
        if node.expanded && !node.children.is_empty() {
            for (child_idx, child) in node.children.iter().enumerate() {
                path.push(child_idx);
                self.flatten_node_with_path(child, depth + 1, path, result);
                path.pop();
            }
        }
    }

    fn check_node_and_children_match(&self, node: &ProcessNode) -> (bool, bool) {
        let node_matches = self.node_matches_filters(node);
        
        // Ricerca ricorsiva di un match sui processi figli
        let has_matching_children = if !self.search_query.is_empty() 
            || self.user_filter.is_some() 
            || self.status_filter.is_some()
            || self.cpu_threshold.is_some()
            || self.memory_threshold.is_some() {
            node.children.iter().any(|child| {
                let (child_matches, child_has_matching) = self.check_node_and_children_match(child);
                child_matches || child_has_matching
            })
        } else {
            false
        };
        
        (node_matches, has_matching_children)
    }

    fn node_matches_filters(&self, node: &ProcessNode) -> bool {
        // Filtro ricerca
        if !self.search_query.is_empty() {
            let query_lower = self.search_query.to_lowercase();
            let name_lower = node.info.name.to_lowercase();
            let pid_str = node.info.pid.to_string();
            
            if !name_lower.contains(&query_lower) && !pid_str.contains(&self.search_query) {
                return false;
            }
        }

        // Filtro utente
        if let Some(ref user_filter) = self.user_filter {
            if let Some(uid) = node.info.user_id {
                if !uid.to_string().contains(user_filter) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Filtro stato - case insensitive
        if let Some(ref status_filter) = self.status_filter {
            let status_lower = node.info.status.to_lowercase();
            let filter_lower = status_filter.to_lowercase();
            
            if !status_lower.contains(&filter_lower) {
                return false;
            }
        }

        // Filtro soglia CPU
        if let Some(threshold) = self.cpu_threshold {
            if node.info.cpu_usage < threshold {
                return false;
            }
        }

        // Filtro soglia memoria
        if let Some(threshold) = self.memory_threshold {
            if node.info.memory < threshold {
                return false;
            }
        }

        true
    }

    pub fn get_process_at_flat_index(&self, flat_idx: usize) -> Option<&ProcessNode> {
        let cached = self.cached_flat_processes.as_ref()?;
        if flat_idx >= cached.len() {
            return None;
        }
        let (_, path) = &cached[flat_idx];
        
        // Navigo direttamente usando il path (0 depth complexity)
        let first_idx = *path.get(0)?;
        let mut current = self.processes.get(first_idx)?;
        
        for &child_idx in &path[1..] {
            current = current.children.get(child_idx)?;
        }
        
        Some(current)
    }

    pub fn toggle_expand(&mut self) {
        let Some(selected) = self.table_state.selected() else { return };
        
        // Prendo il path prima di clonare il processo
        let path = match self.cached_flat_processes.as_ref() {
            Some(cache) => match cache.get(selected) {
                Some((_, p)) => p.clone(),
                None => return,
            },
            None => return,
        };
        
        // Navigo con il path clonato
        let Some(first_idx) = path.get(0) else { return };
        let Some(root) = self.processes.get_mut(*first_idx) else { return };
        
        let mut current = root;
        for &child_idx in &path[1..] {
            let Some(child) = current.children.get_mut(child_idx) else { return };
            current = child;
        }
        
        // Se il processo ha figli faccio il toggle
        if !current.children.is_empty() {
            current.expanded = !current.expanded;
            let pid = current.info.pid;
            self.expanded_pids.insert(pid, current.expanded);
            self.cached_flat_processes = None;
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
        }
        else if index >= self.viewport_offset + visible_rows {
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
        self.search_query.clear();
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