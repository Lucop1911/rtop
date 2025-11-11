use std::time::Instant;

use crate::App;

impl App {
    pub fn refresh(&mut self) {
        self.system.refresh_cpu_all();
        self.system.refresh_memory();

        use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, UpdateKind};
        
        // Refresh degli status
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing()
                .with_cpu()
                .with_memory()
                .with_user(UpdateKind::Always),
        );

        self.networks.refresh(true);

        // Aggiorna memoria CPU
        for (i, cpu) in self.system.cpus().iter().enumerate() {
            if i >= self.cpu_history.len() {
                self.cpu_history.push(Vec::new());
            }
            self.cpu_history[i].push(cpu.cpu_usage());
            if self.cpu_history[i].len() > 60 {
                self.cpu_history[i].remove(0);
            }
        }

        // Aggiorna history memoria
        let used_mem = self.system.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
        self.memory_history.push(used_mem);
        if self.memory_history.len() > 60 {
            self.memory_history.remove(0);
        }

        // Aggiorna memoria network
        let (rx, tx) = self.networks.iter().fold((0, 0), |(rx, tx), (_, net)| {
            (rx + net.received(), tx + net.transmitted())
        });
        self.network_history.push((rx, tx));
        if self.network_history.len() > 60 {
            self.network_history.remove(0);
        }

        self.last_update = Instant::now();
        self.build_process_tree();
    }
}