use crate::{App, ProcessNode};

impl App {
    pub fn flatten_processes(&self) -> Vec<(usize, &ProcessNode)> {
        let mut result = Vec::new();
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
        if self.search_mode && !self.search_query.is_empty() {
            if !node
                .info
                .name
                .to_lowercase()
                .contains(&self.search_query.to_lowercase())
            {
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
                let pid = flat[selected].1.info.pid;
                let expanded = self.expanded_pids.get(&pid).copied().unwrap_or(false);
                self.expanded_pids.insert(pid, !expanded);
            }
        }
    }

    // Selection helpers used by helpers/keyboard.rs and helpers/mouse.rs
    pub fn select_next(&mut self) {
        let flat = self.flatten_processes();
        if !flat.is_empty() {
            let i = self
                .table_state
                .selected()
                .map_or(0, |i| (i + 1).min(flat.len().saturating_sub(1)));
            self.table_state.select(Some(i));
        }
    }

    pub fn select_prev(&mut self) {
        let i = self
            .table_state
            .selected()
            .map_or(0, |i| i.saturating_sub(1));
        self.table_state.select(Some(i));
    }

    pub fn page_down(&mut self) {
        let flat = self.flatten_processes();
        if !flat.is_empty() {
            let i = self
                .table_state
                .selected()
                .map_or(0, |i| (i + 10).min(flat.len().saturating_sub(1)));
            self.table_state.select(Some(i));
        }
    }

    pub fn page_up(&mut self) {
        let i = self
            .table_state
            .selected()
            .map_or(0, |i| i.saturating_sub(10));
        self.table_state.select(Some(i));
    }
}
