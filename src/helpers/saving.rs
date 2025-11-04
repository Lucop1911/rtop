use crate::{App, Preferences};
use anyhow::Result;
use std::{fs, path::PathBuf};

impl App {
    pub fn config_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("procmon");
        fs::create_dir_all(&path).ok();
        path.push("preferences.json");
        path
    }

    pub fn load_preferences() -> Option<Preferences> {
        let path = Self::config_path();
        let contents = fs::read_to_string(path).ok()?;
        serde_json::from_str(&contents).ok()
    }

    pub fn save_preferences(&self) -> Result<()> {
        let path = Self::config_path();
        let contents = serde_json::to_string_pretty(&self.preferences)?;
        fs::write(path, contents)?;
        Ok(())
    }
}