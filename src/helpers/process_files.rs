use crate::App;
use std::fs::{self, File};
use std::time::{SystemTime, UNIX_EPOCH};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

impl App {
    pub fn logs_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("rtop");
        fs::create_dir_all(&path).ok(); 
        
        path.push("open_files");
        fs::create_dir_all(&path).ok();
        path
    }

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
        
        if let Err(_e) = fs::read_dir(&proc_fd_path) {
            return;
        }

        let now = SystemTime::now();
        let duration_since_epoch = now
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let timestamp = format!(
            "{}.{:03}",
            duration_since_epoch.as_secs(),
            duration_since_epoch.subsec_millis()
        );
        
        let logs_dir = Self::logs_path();
        
        let safe_name = name.replace("/", "_").replace(" ", "_");
        let filename = format!("{}-{}.txt", safe_name, timestamp);
        let file_path = logs_dir.join(&filename);
        
        let mut file = match File::create(&file_path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Error creating file {}: {}", file_path.display(), e);
                return;
            }
        };

        writeln!(file, "Open Files for Process: {} (PID: {})", name, pid).ok();
        writeln!(file, "Timestamp: {}", timestamp).ok();
        writeln!(file, "File saved at: {}", file_path.display()).ok();
        writeln!(file, "{}", "=".repeat(80)).ok();
        writeln!(file).ok();

        let entries_result = fs::read_dir(&proc_fd_path);
        
        match entries_result {
            Ok(entries) => {
                let mut fd_count = 0;
                let mut fds: Vec<_> = entries.filter_map(|e| e.ok()).collect();
                fds.sort_by_key(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .parse::<u32>()
                        .unwrap_or(0)
                });
                
                for entry in fds {
                    let fd_num = entry.file_name();
                    let fd_path = entry.path();
                    
                    match fs::read_link(&fd_path) {
                        Ok(target) => {
                            writeln!(
                                file,
                                "FD {:>4}: {}",
                                fd_num.to_string_lossy(),
                                target.display()
                            ).ok();
                            fd_count += 1;
                        }
                        Err(_) => {
                            writeln!(
                                file,
                                "FD {:>4}: <unreadable>",
                                fd_num.to_string_lossy()
                            ).ok();
                            fd_count += 1;
                        }
                    }
                }
                
                writeln!(file).ok();
                writeln!(file, "{}", "=".repeat(80)).ok();
                writeln!(file, "Total open file descriptors: {}", fd_count).ok();
                
                if fd_count == 0 {
                    writeln!(file, "\nNote: No file descriptors found. This is unusual.").ok();
                }
            }
            Err(e) => {
                writeln!(file, "Error reading /proc/{}/fd: {}", pid, e).ok();
                writeln!(file, "\nPossible reasons:").ok();
                writeln!(file, "1. Insufficient permissions - try running with sudo").ok();
                writeln!(file, "2. Process no longer exists").ok();
                writeln!(file, "3. Process is a kernel thread (like kworker) which requires root access").ok();
                
                return;
            }
        }

        drop(file);

        let editors = vec![
            ("xdg-open", vec![file_path.to_str().unwrap()]),
            ("gnome-terminal", vec!["--", "less", file_path.to_str().unwrap()]),
            ("xterm", vec!["-e", "less", file_path.to_str().unwrap()]),
            ("konsole", vec!["-e", "less", file_path.to_str().unwrap()]),
            ("less", vec![file_path.to_str().unwrap()]),
        ];

        let mut opened = false;
        for (editor, args) in editors {
            if let Ok(_) = Command::new(editor).args(&args).spawn() {
                opened = true;
                break;
            }
        }

        if !opened {
            eprintln!("Could not open file automatically. File saved at: {}", file_path.display());
            eprintln!("You can open it manually with: cat {}", file_path.display());
        }
    }
}