use crate::App;
use crate::helpers::utils::detect_terminal;
use procfs2::proc;
use procfs2::proc::process::FdTarget;
use std::io::Write;
use std::process::Command;

impl App {
    pub fn process_open_files(&mut self) {
        let Some(selected) = self.table_state.selected() else {
            return;
        };

        let Some(info) = self.get_process_at_flat_index(selected) else {
            return;
        };

        let pid = info.pid;
        let name = info.name.clone();

        let output = match read_fd_list(pid, &name) {
            Ok(output) => output,
            Err(msg) => {
                self.errors.push(("Open files".to_string(), msg));
                self.input_mode = crate::InputMode::Error;
                return;
            }
        };

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

fn read_fd_list(pid: u32, name: &str) -> Result<String, String> {
    let process = proc::Process::new(pid).map_err(|_| format!("PID {} no longer exists", pid))?;
    let mut fds = process
        .fds()
        .map_err(|_| format!("Cannot read /proc/{}/fd", pid))?;
    fds.sort_by_key(|fd| fd.number);

    let mut output = format!("Open file descriptors for PID {} ({})\n\n", pid, name);
    if fds.is_empty() {
        output.push_str("(none)\n");
    }
    for fd in fds {
        output.push_str(&format!(
            "FD {:>4}: {}\n",
            fd.number,
            format_target(&fd.target)
        ));
    }
    Ok(output)
}

fn format_target(target: &FdTarget) -> String {
    match target {
        FdTarget::File(path) => path.display().to_string(),
        FdTarget::Socket(inode) => format!("socket:[{}]", inode),
        FdTarget::Pipe(inode) => format!("pipe:[{}]", inode),
        FdTarget::AnonInode(kind) => format!("anon_inode:{}", kind),
        FdTarget::MemFD(name) => format!("/memfd:{}", name),
        FdTarget::Other(raw) => raw.to_string(),
    }
}
