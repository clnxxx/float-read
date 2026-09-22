use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

pub const DEFAULT_FONT_FAMILY: &str = "PingFang SC";
pub const DEFAULT_FONT_SIZE: u32 = 18;
pub const MIN_FONT_SIZE: u32 = 12;
pub const MAX_FONT_SIZE: u32 = 48;
const MAX_RECENT: usize = 10;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub font_family: String,
    pub font_size: u32,
    pub recent_files: Vec<String>,
    pub progress: HashMap<String, f64>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            font_family: DEFAULT_FONT_FAMILY.to_string(),
            font_size: DEFAULT_FONT_SIZE,
            recent_files: Vec::new(),
            progress: HashMap::new(),
        }
    }
}

impl AppConfig {
    pub fn load_or_default() -> Self {
        match load_config() {
            Ok(cfg) => cfg,
            Err(_) => Self::default(),
        }
    }

    pub fn normalized(mut self) -> Self {
        if self.font_family.trim().is_empty() {
            self.font_family = DEFAULT_FONT_FAMILY.to_string();
        }
        self.font_size = self.font_size.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE);
        self.recent_files.truncate(MAX_RECENT);
        self.recent_files.dedup();
        self.progress.retain(|_, v| (0.0..=1.0).contains(v));
        self
    }
}

fn config_path() -> Option<PathBuf> {
    // 与 Tauri app_config_dir 对齐：~/Library/Application Support/com.floatread.app
    let home = std::env::var_os("HOME")?;
    let mut path = PathBuf::from(home);
    path.push("Library/Application Support/com.floatread.app");
    path.push("float-read.json");
    Some(path)
}

pub fn load_config() -> Result<AppConfig, String> {
    let path = config_path().ok_or_else(|| "cannot resolve config path".to_string())?;
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let cfg: AppConfig = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    Ok(cfg.normalized())
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let path = config_path().ok_or_else(|| "cannot resolve config path".to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let normalized = config.clone().normalized();
    let raw = serde_json::to_string_pretty(&normalized).map_err(|e| e.to_string())?;
    std::fs::write(&path, raw).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_clamps_font_size_and_progress() {
        let mut cfg = AppConfig {
            font_family: "  ".into(),
            font_size: 99,
            recent_files: vec!["a".into(), "a".into(), "b".into()],
            progress: HashMap::from([
                ("a".into(), 0.5),
                ("bad".into(), 2.0),
            ]),
        };
        cfg = cfg.normalized();
        assert_eq!(cfg.font_family, DEFAULT_FONT_FAMILY);
        assert_eq!(cfg.font_size, MAX_FONT_SIZE);
        assert_eq!(cfg.recent_files, vec!["a".to_string(), "b".to_string()]);
        assert!(cfg.progress.contains_key("a"));
        assert!(!cfg.progress.contains_key("bad"));
    }
}
