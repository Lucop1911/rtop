use crate::App;
use anyhow::Result;

impl App {
    pub fn kill_selected(&mut self) -> Result<()> {
        if let Some(selected) = self.table_state.selected() {
            let flat = self.flatten_processes();
            if selected < flat.len() {
                let pid = flat[selected].1.info.pid;
                if let Some(process) = self.system.process(pid) {
                    process.kill();
                }
                // Immediate refresh to show the process is gone
                self.force_refresh();
            }
        }
        Ok(())
    }
}
