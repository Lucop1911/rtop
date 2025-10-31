use crate::App;
use anyhow::Result;

impl App {
    pub fn initiate_kill(&mut self) -> Result<()> {
        if let Some(selected) = self.table_state.selected() {
            if let Some(node) = self.get_process_at_flat_index(selected) {
                let pid = node.info.pid;
                
                // Controllo se è un processo critico (PID < 10)
                let is_critical = pid.as_u32() < 10 || 
                    node.info.name.to_lowercase().contains("systemd") ||
                    node.info.name.to_lowercase().contains("init") ||
                    node.info.name.to_lowercase().contains("kernel");
                
                if is_critical {
                    self.pending_kill_pid = Some(pid);
                    self.input_mode = crate::InputMode::ConfirmKill;
                } else {
                    if let Some(process) = self.system.process(pid) {
                        process.kill();
                    }
                    self.force_refresh();
                }
            }
        }
        Ok(())
    }

    pub fn suspend_process(&mut self) -> Result<()> {
        #[cfg(unix)]
        {
            if let Some(selected) = self.table_state.selected() {
                if let Some(node) = self.get_process_at_flat_index(selected) {
                    let pid = node.info.pid.as_u32() as i32;
                    unsafe {
                        libc::kill(pid, libc::SIGSTOP);
                    }
                    self.force_refresh();
                }
            }
        }
        Ok(())
    }

    pub fn resume_process(&mut self) -> Result<()> {
        #[cfg(unix)]
        {
            if let Some(selected) = self.table_state.selected() {
                if let Some(node) = self.get_process_at_flat_index(selected) {
                    let pid = node.info.pid.as_u32() as i32;
                    unsafe {
                        libc::kill(pid, libc::SIGCONT);
                    }
                    self.force_refresh();
                }
            }
        }
        Ok(())
    }

}

/*
impl App {
    pub fn kill_selected(&mut self) -> Result<()> {
        if let Some(selected) = self.table_state.selected() {
            if let Some(node) = self.get_process_at_flat_index(selected) {
                let pid = node.info.pid;
                if let Some(process) = self.system.process(pid) {
                    process.kill();
                }
                // Refresh immediato
                self.force_refresh();
            }
        }
        Ok(())
    }
}    
*/