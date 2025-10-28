use std::collections::HashMap;
use sysinfo::Pid;

use crate::{App, ProcessInfo};
impl App {
    pub fn build_process_tree(&mut self) {
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
                children_map
                    .entry(parent_pid)
                    .or_insert_with(Vec::new)
                    .push(*pid);
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
}
