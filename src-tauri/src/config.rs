use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const DEFAULT_FONT_FAMILY: &str = "PingFang SC";
pub const DEFAULT_FONT_SIZE: u32 = 18;
pub const MIN_FONT_SIZE: u32 = 12;
pub const MAX_FONT_SIZE: u32 = 48;
const MAX_RECENT: usize = 10;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
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
    pub fn normalized(mut self) -> Self {
        if self.font_family.trim().is_empty() {
            self.font_family = DEFAULT_FONT_FAMILY.to_string();
        }
        self.font_size = self.font_size.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE);
        // dedup keeping first occurrence, then cap
        let mut seen = std::collections::HashSet::new();
        self.recent_files.retain(|p| seen.insert(p.clone()));
        self.recent_files.truncate(MAX_RECENT);
        for v in self.progress.values_mut() {
            *v = v.clamp(0.0, 1.0);
        }
        self
    }
}

fn default_config_path() -> PathBuf {
    // 与 Tauri identifier `com.floatread.app` 的 app_config_dir 对齐（macOS）
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    home.join("Library/Application Support/com.floatread.app/float-read.json")
}

fn resolve_config_path(base_dir: Option<&Path>) -> PathBuf {
    match base_dir {
        Some(dir) => dir.join("float-read.json"),
        None => default_config_path(),
    }
}

pub fn load_config_from(path: &Path) -> Result<AppConfig, String> {
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let cfg: AppConfig = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    Ok(cfg.normalized())
}

pub fn save_config_to(path: &Path, config: &AppConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let normalized = config.clone().normalized();
    let raw = serde_json::to_string_pretty(&normalized).map_err(|e| e.to_string())?;
    std::fs::write(path, raw).map_err(|e| e.to_string())
}

/// 供 command 使用：把 Tauri 的 app_config_dir 作为配置目录。
pub fn config_file_in(dir: &Path) -> PathBuf {
    resolve_config_path(Some(dir))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_clamps_font_size_and_progress() {
        let mut cfg = AppConfig {
            font_family: "  ".into(),
            font_size: 99,
            recent_files: vec!["a".into(), "a".into(), "b".into(), "a".into()],
            progress: HashMap::from([
                ("a".into(), 0.5),
                ("bad".into(), 2.0),
            ]),
        };
        cfg = cfg.normalized();
        assert_eq!(cfg.font_family, DEFAULT_FONT_FAMILY);
        assert_eq!(cfg.font_size, MAX_FONT_SIZE);
        assert_eq!(cfg.recent_files, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(cfg.progress.get("a"), Some(&0.5));
        assert_eq!(cfg.progress.get("bad"), Some(&1.0));
    }

    #[test]
    fn serde_uses_camel_case_for_ipc() {
        let cfg = AppConfig::default();
        let raw = serde_json::to_string(&cfg).unwrap();
        assert!(raw.contains("\"fontFamily\""), "got {raw}");
        assert!(raw.contains("\"fontSize\""), "got {raw}");
        assert!(raw.contains("\"recentFiles\""), "got {raw}");
        let back: AppConfig = serde_json::from_str(
            r#"{"fontFamily":"Songti SC","fontSize":22,"recentFiles":["/a.txt"],"progress":{"/a.txt":0.3}}"#,
        )
        .unwrap();
        assert_eq!(back.font_family, "Songti SC");
        assert_eq!(back.font_size, 22);
        assert_eq!(back.recent_files, vec!["/a.txt".to_string()]);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = std::env::temp_dir().join(format!("float-read-cfg-{}", std::process::id()));
        let path = config_file_in(&dir);
        let mut cfg = AppConfig::default();
        cfg.font_family = "Menlo".into();
        cfg.font_size = 20;
        cfg.recent_files = vec!["/tmp/x.txt".into()];
        cfg.progress.insert("/tmp/x.txt".into(), 0.75);
        save_config_to(&path, &cfg).unwrap();
        let loaded = load_config_from(&path).unwrap();
        assert_eq!(loaded.font_family, "Menlo");
        assert_eq!(loaded.font_size, 20);
        assert_eq!(loaded.recent_files, vec!["/tmp/x.txt".to_string()]);
        assert_eq!(loaded.progress.get("/tmp/x.txt"), Some(&0.75));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
