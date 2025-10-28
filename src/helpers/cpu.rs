use crate::App;

pub fn calculate_avg_cpu(app: &App) -> f32 {
    app.system.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>() / app.system.cpus().len() as f32
}
