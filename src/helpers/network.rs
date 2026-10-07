use crate::App;

/// (received, transmitted) bytes over all interfaces since the last sample.
pub fn calculate_network_totals(app: &App) -> (u64, u64) {
    app.net_deltas
        .iter()
        .fold((0, 0), |(rx, tx), (_, iface_rx, iface_tx)| {
            (rx + iface_rx, tx + iface_tx)
        })
}

/// Per-interface (name, received_MB, transmitted_MB) since the last sample.
pub fn per_interface_info(app: &App) -> Vec<(String, f64, f64)> {
    app.net_deltas
        .iter()
        .map(|(name, rx, tx)| {
            (
                name.clone(),
                *rx as f64 / 1024.0 / 1024.0,
                *tx as f64 / 1024.0 / 1024.0,
            )
        })
        .collect()
}
