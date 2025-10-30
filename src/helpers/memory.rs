use crate::App;

pub fn calculate_memory(app: &App) -> (f64, f64, u16) {
    let total_mem = app.system.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
    let used_mem = app.system.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
    let percent_used = ((used_mem / total_mem) * 100.0) as u16;
    
    return (used_mem, total_mem, percent_used)
}
