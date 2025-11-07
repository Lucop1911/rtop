use crate::App;

#[cfg(not(target_os = "windows"))]
use procfs;

#[cfg(target_os = "windows")]
use windows::Win32::{
    Foundation::CloseHandle,
    System::{
        Threading::{OpenProcess, PROCESS_QUERY_INFORMATION},
        IO::{GetProcessIoCounters, IO_COUNTERS},
    },
};

pub fn calculate_memory(app: &App) -> (f64, f64, u16) {
    let total_mem = app.system.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
    let used_mem = app.system.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
    let percent_used = ((used_mem / total_mem) * 100.0) as u16;

    return (used_mem, total_mem, percent_used);
}

impl App {
    #[cfg(not(target_os = "windows"))]
    pub fn calculate_process_io(&self) -> Option<(u64, u64)> {
        if let Some(selected) = self.table_state.selected() {
            if let Some(node) = self.get_process_at_flat_index(selected) {
                let pid = node.info.pid.as_u32() as i32;
                if let Ok(procfs_proc) = procfs::process::Process::new(pid) {
                    if let Ok(io) = procfs_proc.io() {
                        return Some((io.read_bytes, io.write_bytes));
                    }
                }
            }
        }
        None
    }

    #[cfg(target_os = "windows")]
    pub fn calculate_process_io(&self) -> Option<(u64, u64)> {
        if let Some(selected) = self.table_state.selected() {
            if let Some(node) = self.get_process_at_flat_index(selected) {
                let pid = node.info.pid.as_u32();
                
                unsafe {
                    if let Ok(handle) = OpenProcess(PROCESS_QUERY_INFORMATION, false, pid) {
                        let mut io_counters = IO_COUNTERS::default();
                        
                        if GetProcessIoCounters(handle, &mut io_counters).is_ok() {
                            let read_bytes = io_counters.ReadTransferCount;
                            let write_bytes = io_counters.WriteTransferCount;
                            
                            let _ = CloseHandle(handle);
                            
                            return Some((read_bytes, write_bytes));
                        }
                        
                        let _ = CloseHandle(handle);
                    }
                }
            }
        }
        None
    }
}