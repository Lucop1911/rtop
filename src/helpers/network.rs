use crate::App;

/// Returns (total_received_bytes, total_transmitted_bytes)
pub fn calculate_network_totals(app: &App) -> (u64, u64) {
    app.networks
        .iter()
        .map(|(_, net)| (net.received(), net.transmitted()))
        .fold((0, 0), |(rx, tx), (r, t)| (rx + r, tx + t))
}

/// Returns per-interface info as Vec<(name, received_MB, transmitted_MB)>
pub fn per_interface_info(app: &App) -> Vec<(String, f64, f64)> {
    app.networks
        .iter()
        .map(|(name, net)| {
            (
                name.clone(),
                net.received() as f64 / 1024.0 / 1024.0,
                net.transmitted() as f64 / 1024.0 / 1024.0,
            )
        })
        .collect()
}
