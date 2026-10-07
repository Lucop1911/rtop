use crate::App;

pub fn calculate_memory(app: &App) -> (f64, f64, u16) {
    let total_mem = app.mem_total_gb;
    let used_mem = app.mem_used_gb;
    let percent_used = if total_mem > 0.0 {
        ((used_mem / total_mem) * 100.0) as u16
    } else {
        0
    };

    (used_mem, total_mem, percent_used)
}
