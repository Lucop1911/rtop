use std::collections::HashMap;
use std::time::Instant;

use procfs2::proc;
use procfs2::proc::stat::CpuTime;

use crate::{App, ProcessInfo};

impl App {
    pub fn refresh(&mut self) {
        if self.refresh {
            self.sample_all();
        }
    }

    pub fn force_refresh(&mut self) {
        self.sample_all();
    }

    /// Samples everything from `/proc` and rebuilds the display list.
    fn sample_all(&mut self) {
        self.sample_cpu();
        self.sample_memory();
        self.sample_network();
        self.sample_processes();
        self.rebuild_display();
        self.last_update = Instant::now();
        self.last_sample = Some(Instant::now());
    }

    fn sample_cpu(&mut self) {
        let Ok(stat) = proc::stat() else {
            return;
        };

        self.boot_time = stat.btime;

        let cores: Vec<(u64, u64)> = stat.per_cpu.iter().map(|c| busy_total(&c.times)).collect();
        let total = busy_total(&stat.cpu_total);

        if self.prev_core_jiffies.len() == cores.len() {
            self.cpu_usage = cores
                .iter()
                .zip(&self.prev_core_jiffies)
                .map(|(cur, prev)| delta_percent(*prev, *cur))
                .collect();
            self.cpu_total_usage = delta_percent(self.prev_total_jiffies, total);
        } else {
            // First sample (or the CPU count changed): no delta yet.
            self.cpu_usage = vec![0.0; cores.len()];
            self.cpu_total_usage = 0.0;
        }

        for (i, usage) in self.cpu_usage.iter().enumerate() {
            if i >= self.cpu_history.len() {
                self.cpu_history.push(Vec::new());
            }
            self.cpu_history[i].push(*usage);
            if self.cpu_history[i].len() > 60 {
                self.cpu_history[i].remove(0);
            }
        }

        self.prev_core_jiffies = cores;
        self.prev_total_jiffies = total;
    }

    fn sample_memory(&mut self) {
        let Ok(mem) = proc::meminfo() else {
            return;
        };

        let total_kb = mem.total.0;
        let used_kb = total_kb.saturating_sub(mem.available.0);

        self.mem_total_gb = total_kb as f64 / 1024.0 / 1024.0;
        self.mem_used_gb = used_kb as f64 / 1024.0 / 1024.0;

        self.memory_history.push(self.mem_used_gb);
        if self.memory_history.len() > 60 {
            self.memory_history.remove(0);
        }
    }

    fn sample_network(&mut self) {
        let mut deltas = Vec::new();
        let mut new_prev: HashMap<String, (u64, u64)> = HashMap::new();
        let (mut sum_rx, mut sum_tx) = (0u64, 0u64);

        for dev in proc::net::dev().filter_map(|d| d.ok()) {
            let rx = dev.rx_bytes.0;
            let tx = dev.tx_bytes.0;
            // On the very first sample there is no baseline yet, so report
            // 0 instead of the cumulative byte counter since boot.
            let (delta_rx, delta_tx) = match self.prev_net_bytes.get(dev.name.as_ref()) {
                Some(&(prev_rx, prev_tx)) => {
                    (rx.saturating_sub(prev_rx), tx.saturating_sub(prev_tx))
                }
                None => (0, 0),
            };
            sum_rx += delta_rx;
            sum_tx += delta_tx;
            deltas.push((dev.name.to_string(), delta_rx, delta_tx));
            new_prev.insert(dev.name.to_string(), (rx, tx));
        }

        self.prev_net_bytes = new_prev;
        self.net_deltas = deltas;

        self.network_history.push((sum_rx, sum_tx));
        if self.network_history.len() > 60 {
            self.network_history.remove(0);
        }
    }

    fn sample_processes(&mut self) {
        let now = Instant::now();
        let elapsed = self
            .last_sample
            .map(|prev| now.duration_since(prev).as_secs_f64())
            .unwrap_or(0.0);

        let prev_jiffies = std::mem::take(&mut self.prev_proc_jiffies);
        let mut new_jiffies = HashMap::with_capacity(prev_jiffies.len());
        let mut processes: Vec<ProcessInfo> = Vec::with_capacity(prev_jiffies.len().max(64));

        for process in proc::Process::all().filter_map(|p| p.ok()) {
            let Ok(stat) = process.stat() else {
                continue;
            };

            // `/proc/PID/status` is only needed to resolve the uid, which
            // is only used by the user filter — skip the read otherwise
            // and let the detail panel resolve it on demand.
            let user_id = if self.user_filter.is_some() {
                process.status().map(|status| status.uid.effective).ok()
            } else {
                None
            };

            let cpu_time = stat.utime.saturating_add(stat.stime);
            // Normalize across all cores (htop with "Irix mode" off):
            // 100% means the whole machine, so a process on one full
            // core shows 100/ncpus % instead of 100%. `sample_cpu` runs
            // first and keeps `self.cpu_usage` at one entry per core.
            let cores = self.cpu_usage.len().max(1) as f64;
            let cpu_usage = match prev_jiffies.get(&stat.pid) {
                Some(&prev) if elapsed > 0.0 => {
                    let jiffies = cpu_time.saturating_sub(prev) as f64;
                    (jiffies / self.clk_tck as f64 / elapsed * 100.0 / cores) as f32
                }
                _ => 0.0,
            };
            new_jiffies.insert(stat.pid, cpu_time);

            processes.push(ProcessInfo {
                pid: stat.pid,
                ppid: stat.ppid,
                name: stat.comm.to_string(),
                status: state_name(stat.state).to_string(),
                cpu_usage,
                memory: stat.rss.max(0) as u64 * self.page_size,
                vsize: stat.vsize,
                user_id,
                start_time: self.boot_time + stat.starttime / self.clk_tck,
                num_threads: stat.num_threads,
            });
        }

        self.prev_proc_jiffies = new_jiffies;
        self.processes = processes;
    }
}

/// Returns (busy, total) jiffies for a CPU time counter.
///
/// Guest time is excluded from the total because it is already
/// accounted for in user time.
fn busy_total(times: &CpuTime) -> (u64, u64) {
    let total = times.user.0
        + times.nice.0
        + times.system.0
        + times.idle.0
        + times.iowait.0
        + times.irq.0
        + times.softirq.0
        + times.steal.0;
    let idle = times.idle.0 + times.iowait.0;
    (total.saturating_sub(idle), total)
}

fn delta_percent(prev: (u64, u64), cur: (u64, u64)) -> f32 {
    let total = cur.1.saturating_sub(prev.1);
    if total == 0 {
        0.0
    } else {
        (cur.0.saturating_sub(prev.0) as f64 / total as f64 * 100.0) as f32
    }
}

fn state_name(state: char) -> &'static str {
    match state {
        'R' => "Running",
        'S' => "Sleeping",
        'D' => "Waiting",
        'Z' => "Zombie",
        'T' | 't' => "Stopped",
        'X' | 'x' => "Dead",
        'I' => "Idle",
        'K' => "Wakekill",
        'W' => "Waking",
        'P' => "Parked",
        _ => "Unknown",
    }
}
