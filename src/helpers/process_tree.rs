use std::collections::HashMap;
use sysinfo::Pid;

use crate::{App, ProcessInfo};

impl App {
    pub fn build_process_tree(&mut self) {
        // Remember selection line number before rebuilding
        let selected_line = self.table_state.selected();

        let mut process_infos: HashMap<Pid, ProcessInfo> = HashMap::new();
        let mut children_map: HashMap<Pid, Vec<Pid>> = HashMap::new();

        // Pre-allocate with estimated capacity
        let process_count = self.system.processes().len();
        process_infos.reserve(process_count);

        // Collect all process information
        for (pid, process) in self.system.processes() {
            let info = ProcessInfo {
                pid: *pid,
                name: process.name().to_string_lossy().to_string(),
                cpu_usage: process.cpu_usage(),
                memory: process.memory(),
            };
            process_infos.insert(*pid, info);

            // Build parent-child relationships
            if let Some(parent_pid) = process.parent() {
                children_map
                    .entry(parent_pid)
                    .or_insert_with(Vec::new)
                    .push(*pid);
            }
        }

        // Show all processes as roots (flat structure)
        let mut roots = Vec::with_capacity(process_count);
        for (pid, _info) in &process_infos {
            roots.push(self.build_node(*pid, &process_infos, &children_map));
        }

        self.sort_processes(&mut roots);
        self.processes = roots;

        // Invalidate cache after rebuilding tree
        self.cached_flat_processes = None;

        // Restore selection to same line number
        let flat_len = self.flatten_processes().len();
        if flat_len > 0 {
            if let Some(idx) = selected_line {
                // Keep same line number, but clamp to valid range
                self.table_state.select(Some(idx.min(flat_len - 1)));
            } else {
                // No previous selection, select first item
                self.table_state.select(Some(0));
            }
        } else {
            self.table_state.select(None);
        }
    }
}