use std::collections::HashMap;
use sysinfo::Pid;

use crate::{App, ProcessInfo};

impl App {
    pub fn build_process_tree(&mut self) {
        let selected_line = self.table_state.selected();

        let mut process_infos: HashMap<Pid, ProcessInfo> = HashMap::new();
        let mut children_map: HashMap<Pid, Vec<Pid>> = HashMap::new();

        let process_count = self.system.processes().len();
        process_infos.reserve(process_count);

        let cpu_number = self.system.cpus().len() as f32;
        for (pid, process) in self.system.processes() {
            let user_id = process.user_id().map(|uid| **uid);
            
            let info = ProcessInfo {
                pid: *pid,
                name: process.name().to_string_lossy().to_string(),
                cpu_usage: process.cpu_usage() / cpu_number, // Divido per il numero di cores per avere un valore tra 0-100 
                memory: process.memory(),
                user_id,
                status: format!("{:?}", process.status()),
            };
            process_infos.insert(*pid, info);

            if let Some(parent_pid) = process.parent() {
                children_map
                    .entry(parent_pid)
                    .or_insert_with(Vec::new)
                    .push(*pid);
            }
        }

        let mut roots = Vec::with_capacity(process_count);
        for (pid, _info) in &process_infos {
            roots.push(self.build_node(*pid, &process_infos, &children_map));
        }

        self.sort_processes(&mut roots);
        self.processes = roots;

        self.cached_flat_processes = None;

        let flat_len = self.flatten_processes().len();
        if flat_len > 0 {
            if let Some(idx) = selected_line {
                self.table_state.select(Some(idx.min(flat_len - 1)));
            } else {
                self.table_state.select(Some(0));
            }
        } else {
            self.table_state.select(None);
        }
    }
}