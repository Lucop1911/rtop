use crate::{App, ProcessInfo, ProcessNode};
use std::collections::HashMap;
use sysinfo::Pid;

impl App {
    pub fn build_node(
        &self,
        pid: Pid,
        all_processes: &HashMap<Pid, ProcessInfo>,
        children_map: &HashMap<Pid, Vec<Pid>>,
    ) -> ProcessNode {
        let info = all_processes.get(&pid).unwrap().clone();
        let mut children = Vec::new();

        if let Some(child_pids) = children_map.get(&pid) {
            for child_pid in child_pids {
                if all_processes.contains_key(child_pid) {
                    children.push(self.build_node(*child_pid, all_processes, children_map));
                }
            }
        }

        ProcessNode {
            info,
            children,
            expanded: self.expanded_pids.get(&pid).copied().unwrap_or(false),
        }
    }
}
