use std::collections::{HashMap, HashSet};
use sysinfo::Pid;

use crate::{App, ProcessInfo};

impl App {
    pub fn build_process_tree(&mut self) {
        let selected_line = self.table_state.selected();

        let mut process_infos: HashMap<Pid, ProcessInfo> = HashMap::new();
        let mut children_map: HashMap<Pid, Vec<Pid>> = HashMap::new();
        let mut has_parent: HashSet<Pid> = HashSet::new();

        let process_count = self.system.processes().len();
        process_infos.reserve(process_count);

        let cpu_number = self.system.cpus().len() as f32;
        for (pid, process) in self.system.processes() {
            let user_id = process.user_id().map(|uid| **uid);

            // Format più clean
            let status_str = match process.status() {
                sysinfo::ProcessStatus::Run => "Running".to_string(),
                sysinfo::ProcessStatus::Sleep => "Sleeping".to_string(),
                sysinfo::ProcessStatus::Idle => "Idle".to_string(),
                sysinfo::ProcessStatus::Zombie => "Zombie".to_string(),
                sysinfo::ProcessStatus::Stop => "Stopped".to_string(),
                sysinfo::ProcessStatus::Tracing => "Tracing".to_string(),
                sysinfo::ProcessStatus::Dead => "Dead".to_string(),
                sysinfo::ProcessStatus::Wakekill => "Wakekill".to_string(),
                sysinfo::ProcessStatus::Waking => "Waking".to_string(),
                sysinfo::ProcessStatus::Parked => "Parked".to_string(),
                sysinfo::ProcessStatus::LockBlocked => "LockBlocked".to_string(),
                sysinfo::ProcessStatus::UninterruptibleDiskSleep => "DiskSleep".to_string(),
                _ => format!("{:?}", process.status()),
            };

            let info = ProcessInfo {
                pid: *pid,
                name: process.name().to_string_lossy().to_string(),
                cpu_usage: process.cpu_usage() / cpu_number,
                memory: process.memory(),
                user_id,
                status: status_str,
            };
            process_infos.insert(*pid, info);

            if let Some(parent_pid) = process.parent() {
                children_map
                    .entry(parent_pid)
                    .or_insert_with(Vec::new)
                    .push(*pid);
                has_parent.insert(*pid);
            }
        }

        // Processi root del sistema da skippare (systemd and kthreadd)
        let skip_pids: HashSet<u32> = [1, 2].iter().copied().collect();

        let mut roots = Vec::new();

        // Faccio diventare root i processi senza un parent e i figli diretti dei processi 1 e 2 (systemd e kthreadd)
        for (pid, _info) in &process_infos {
            let pid_u32 = pid.as_u32();

            if skip_pids.contains(&pid_u32) {
                continue;
            }

            let should_be_root = if !has_parent.contains(pid) {
                true
            } else {
                if let Some(process) = self.system.process(*pid) {
                    if let Some(parent_pid) = process.parent() {
                        skip_pids.contains(&parent_pid.as_u32())
                    } else {
                        true
                    }
                } else {
                    false
                }
            };

            if should_be_root {
                roots.push(self.build_node(*pid, &process_infos, &children_map));
            }
        }

        self.sort_processes(&mut roots);
        self.processes = roots;

        // Aggiorno expanded_pids in modo che contenga processi ancora esistenti
        let existing_pids: HashSet<Pid> = process_infos.keys().copied().collect();
        self.expanded_pids
            .retain(|pid, _| existing_pids.contains(pid));

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