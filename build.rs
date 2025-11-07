#[cfg(windows)]
fn main() {
    let mut res = winres::WindowsResource::new();
    res
        .set("ProductName", "rtop")
        .set("FileDescription", "An htop-like TUI built with Rust and Ratatui")
        .set("LegalCopyright", "Copyright (c) 2025");
    
    if let Err(e) = res.compile() {
        eprintln!("Warning: Failed to compile Windows resources: {}", e);
    }
}

#[cfg(not(windows))]
fn main() {
}