use crate::App;
use anyhow::Result;
use procfs2::proc;

impl App {
    pub fn initiate_kill(&mut self) -> Result<()> {
        if let Some(selected) = self.table_state.selected()
            && let Some(info) = self.get_process_at_flat_index(selected)
        {
            let pid = info.pid;

            // Check if it is a critical system process (PID < 10)
            let is_critical = pid < 10
                || info.name.to_lowercase().contains("systemd")
                || info.name.to_lowercase().contains("init")
                || info.name.to_lowercase().contains("kernel");

            if is_critical {
                self.pending_kill_pid = Some(pid);
                self.input_mode = crate::InputMode::ConfirmKill;
            } else {
                self.kill_pid(pid);
                self.refresh();
            }
        }
        Ok(())
    }

    /// Sends SIGKILL to `pid`. On failure records the error and shows the
    /// error overlay. Returns whether the signal was delivered.
    pub fn kill_pid(&mut self, pid: u32) -> bool {
        if proc::Process::new(pid).is_err() {
            self.errors.push((
                "Process not found".to_string(),
                format!("PID {} no longer exists", pid),
            ));
            self.input_mode = crate::InputMode::Error;
            return false;
        }

        let result = unsafe { libc::kill(pid as i32, libc::SIGKILL) };
        if result == -1 {
            let errno = unsafe { *libc::__errno_location() };
            self.errors.push((
                "Failed to kill process".to_string(),
                format!("PID {}: errno {}", pid, errno),
            ));
            self.input_mode = crate::InputMode::Error;
            return false;
        }

        true
    }

    pub fn suspend_process(&mut self) -> Result<()> {
        if let Some(selected) = self.table_state.selected()
            && let Some(info) = self.get_process_at_flat_index(selected)
        {
            let pid = info.pid;
            let result = unsafe { libc::kill(pid as i32, libc::SIGSTOP) };

            if result == -1 {
                let errno = unsafe { *libc::__errno_location() };
                self.errors.push((
                    "Failed to suspend process".to_string(),
                    format!("PID {}: errno {}", pid, errno),
                ));
                self.input_mode = crate::InputMode::Error;
            } else {
                self.refresh();
            }
        }
        Ok(())
    }

    pub fn resume_process(&mut self) -> Result<()> {
        if let Some(selected) = self.table_state.selected()
            && let Some(info) = self.get_process_at_flat_index(selected)
        {
            let pid = info.pid;
            let result = unsafe { libc::kill(pid as i32, libc::SIGCONT) };

            if result == -1 {
                let errno = unsafe { *libc::__errno_location() };
                self.errors.push((
                    "Failed to resume process".to_string(),
                    format!("PID {}: errno {}", pid, errno),
                ));
                self.input_mode = crate::InputMode::Error;
            } else {
                self.refresh();
            }
        }
        Ok(())
    }
}
