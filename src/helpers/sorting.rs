use crate::{App, ProcessNode, SortColumn};

impl App {
    pub fn sort_processes(&self, nodes: &mut Vec<ProcessNode>) {
        nodes.sort_by(|a, b| {
            let ordering = match self.sort_column {
                SortColumn::Pid => a.info.pid.cmp(&b.info.pid),
                SortColumn::Name => a.info.name.cmp(&b.info.name),
                SortColumn::Cpu => a.info.cpu_usage.partial_cmp(&b.info.cpu_usage).unwrap(),
                SortColumn::Memory => a.info.memory.cmp(&b.info.memory),
            };
            if self.reverse_sort {
                ordering.reverse()
            } else {
                ordering
            }
        });

        for node in nodes {
            self.sort_processes(&mut node.children);
        }
    }
}
