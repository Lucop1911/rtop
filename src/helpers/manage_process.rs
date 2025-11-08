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
                        if !process.kill() {
                            eprintln!("Failed to kill process {}", pid.as_u32());
                        }
                    }
                    self.force_refresh();
                }
            }
        }
        Ok(())
    }

    pub fn suspend_process(&mut self) -> Result<()> {
        if let Some(selected) = self.table_state.selected() {
            if let Some(node) = self.get_process_at_flat_index(selected) {
                let pid = node.info.pid.as_u32() as i32;
                
                let result = unsafe { libc::kill(pid, libc::SIGSTOP) };
                
                if result == -1 {
                    let errno = unsafe { *libc::__errno_location() };
                    eprintln!("Failed to suspend process {}: errno {}", pid, errno);
                } else {
                    self.force_refresh();
                }
            }
        }        
        Ok(())
    }

    pub fn resume_process(&mut self) -> Result<()> {

        if let Some(selected) = self.table_state.selected() {
            if let Some(node) = self.get_process_at_flat_index(selected) {
                let pid = node.info.pid.as_u32() as i32;
                
                let result = unsafe { libc::kill(pid, libc::SIGCONT) };
                
                if result == -1 {
                    let errno = unsafe { *libc::__errno_location() };
                    eprintln!("Failed to resume process {}: errno {}", pid, errno);
                } else {
                    self.force_refresh();
                }
            }
        }
        Ok(())
    }
}