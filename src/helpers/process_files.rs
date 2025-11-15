use crate::App;
use crate::helpers::utils::detect_terminal;
use std::fs::{self};
use std::process::Command;
use std::io::Write;

impl App {
    pub fn process_open_files(&mut self) {
        let Some(selected) = self.table_state.selected() else {
            return;
        };

        let Some(node) = self.get_process_at_flat_index(selected) else {
            return;
        };

        let pid = node.info.pid.as_u32() as i32;
        let proc_fd_path = format!("/proc/{}/fd", pid);
        let name = node.info.name.clone();
        
        if fs::read_dir(&proc_fd_path).is_err() {
            return;
        }

        let mut output = format!("Open file descriptors for PID {} ({})\n\n", pid, name);

        let entries = match fs::read_dir(&proc_fd_path) {
            Ok(e) => e,
            Err(_) => return,
        };

        let mut fds: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        fds.sort_by_key(|e| {
            e.file_name()
                .to_string_lossy()
                .parse::<u32>()
                .unwrap_or(0)
        });

        for entry in fds {
            let fd_num = entry.file_name().to_string_lossy().to_string();
            let fd_path = entry.path();

            let line = match fs::read_link(&fd_path) {
                Ok(target) => format!("FD {:>4}: {}\n", fd_num, target.display()),
                Err(_) => format!("FD {:>4}: <unreadable>\n", fd_num),
            };

            output.push_str(&line);
        }

        let terminal = detect_terminal().unwrap_or("xterm");

        let temp_file = format!("/tmp/rtop_files_{}.txt", pid);
        if let Ok(mut file) = std::fs::File::create(&temp_file) {
            let _ = file.write_all(output.as_bytes());
            let _ = Command::new(terminal)
                .arg("-e")
                .arg("sh")
                .arg("-c")
                .arg(format!("less {}; rm {}", temp_file, temp_file))
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
        }
    }
}