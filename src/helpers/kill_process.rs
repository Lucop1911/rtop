use crate::App;
use anyhow::Result;

impl App {
    pub fn kill_selected(&mut self) -> Result<()> {
        if let Some(selected) = self.table_state.selected() {
            if let Some(node) = self.get_process_at_flat_index(selected) {
                let pid = node.info.pid;
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