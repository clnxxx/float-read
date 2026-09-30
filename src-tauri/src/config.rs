use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const DEFAULT_FONT_FAMILY: &str = "PingFang SC";
pub const DEFAULT_FONT_SIZE: u32 = 18;
pub const MIN_FONT_SIZE: u32 = 12;
pub const MAX_FONT_SIZE: u32 = 48;
const MAX_RECENT: usize = 10;
pub const MIN_WINDOW_WIDTH: f64 = 80.0;
pub const MIN_WINDOW_HEIGHT: f64 = 18.0;
pub const DEFAULT_AUTO_SCROLL_SPEED: u32 = 40;

/// 窗口几何（逻辑坐标），由 Rust 侧维护并持久化
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WindowGeometry {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Default for WindowGeometry {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 260.0,
            height: 320.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppConfig {
    pub font_family: String,
    pub font_size: u32,
    pub font_color: String,
    pub recent_files: Vec<String>,
    pub progress: HashMap<String, f64>,
    pub window: Option<WindowGeometry>,
    pub auto_scroll_speed: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            font_family: DEFAULT_FONT_FAMILY.to_string(),
            font_size: DEFAULT_FONT_SIZE,
            font_color: "white".to_string(),
            recent_files: Vec::new(),
            progress: HashMap::new(),
            window: None,
            auto_scroll_speed: DEFAULT_AUTO_SCROLL_SPEED,
        }
    }
}

impl AppConfig {
    pub fn normalized(mut self) -> Self {
        if self.font_family.trim().is_empty() {
            self.font_family = DEFAULT_FONT_FAMILY.to_string();
        }
        self.font_size = self.font_size.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE);
        if self.font_color != "black" && self.font_color != "white" {
            self.font_color = "white".to_string();
        }
        // dedup keeping first occurrence, then cap
        let mut seen = std::collections::HashSet::new();
        self.recent_files.retain(|p| seen.insert(p.clone()));
        self.recent_files.truncate(MAX_RECENT);
        for v in self.progress.values_mut() {
            *v = v.clamp(0.0, 1.0);
        }
        self.auto_scroll_speed = self.auto_scroll_speed.clamp(10, 400);
        let valid = self.window.is_some_and(|w| {
            w.x.is_finite() && w.y.is_finite() && w.width.is_finite() && w.height.is_finite()
        });
        match &mut self.window {
            Some(w) if valid => {
                w.width = w.width.clamp(MIN_WINDOW_WIDTH, 10000.0);
                w.height = w.height.clamp(MIN_WINDOW_HEIGHT, 10000.0);
            }
            _ => self.window = None,
        }
        self
    }
}

pub fn load_config_from(path: &Path) -> Result<AppConfig, String> {
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    match serde_json::from_str::<AppConfig>(&raw) {
        Ok(cfg) => Ok(cfg.normalized()),
        Err(e) => {
            // 配置损坏时先挪到 .bak 再回默认，避免下次保存直接覆盖、进度无从找回
            let backup = path.with_extension("bak");
            let _ = std::fs::rename(path, backup);
            Err(e.to_string())
        }
    }
}

pub fn save_config_to(path: &Path, config: &AppConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let normalized = config.clone().normalized();
    let raw = serde_json::to_string_pretty(&normalized).map_err(|e| e.to_string())?;
    // 先写临时文件再原子替换，写一半被强退也不会损坏旧配置
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, raw).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

/// 配置文件位于 Tauri `app_config_dir`/float-read.json
pub fn config_file_in(dir: &Path) -> PathBuf {
    dir.join("float-read.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_clamps_font_size_and_progress() {
        let mut cfg = AppConfig {
            font_family: "  ".into(),
            font_size: 99,
            font_color: "red".into(),
            recent_files: vec!["a".into(), "a".into(), "b".into(), "a".into()],
            progress: HashMap::from([
                ("a".into(), 0.5),
                ("bad".into(), 2.0),
            ]),
            ..AppConfig::default()
        };
        cfg = cfg.normalized();
        assert_eq!(cfg.font_family, DEFAULT_FONT_FAMILY);
        assert_eq!(cfg.font_size, MAX_FONT_SIZE);
        assert_eq!(cfg.font_color, "white");
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
        assert!(raw.contains("\"fontColor\""), "got {raw}");
        assert!(raw.contains("\"recentFiles\""), "got {raw}");
        let back: AppConfig = serde_json::from_str(
            r#"{"fontFamily":"Songti SC","fontSize":22,"fontColor":"black","recentFiles":["/a.txt"],"progress":{"/a.txt":0.3},"window":{"x":11.5,"y":22,"width":300,"height":200},"autoScrollSpeed":80}"#,
        )
        .unwrap();
        assert_eq!(back.font_family, "Songti SC");
        assert_eq!(back.font_size, 22);
        assert_eq!(back.font_color, "black");
        assert_eq!(back.recent_files, vec!["/a.txt".to_string()]);
        assert_eq!(back.auto_scroll_speed, 80);
        assert_eq!(
            back.window,
            Some(WindowGeometry {
                x: 11.5,
                y: 22.0,
                width: 300.0,
                height: 200.0
            })
        );
    }

    #[test]
    fn normalized_clamps_window_and_speed() {
        let cfg = AppConfig {
            window: Some(WindowGeometry {
                x: 0.0,
                y: 0.0,
                width: 5.0,
                height: f64::NAN,
            }),
            auto_scroll_speed: 99_999,
            ..AppConfig::default()
        };
        let cfg = cfg.normalized();
        assert_eq!(cfg.auto_scroll_speed, 400);
        assert!(cfg.window.is_none());
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
        cfg.window = Some(WindowGeometry {
            x: 120.0,
            y: 88.0,
            width: 400.0,
            height: 500.0,
        });
        cfg.auto_scroll_speed = 65;
        save_config_to(&path, &cfg).unwrap();
        let loaded = load_config_from(&path).unwrap();
        assert_eq!(loaded.font_family, "Menlo");
        assert_eq!(loaded.font_size, 20);
        assert_eq!(loaded.recent_files, vec!["/tmp/x.txt".to_string()]);
        assert_eq!(loaded.progress.get("/tmp/x.txt"), Some(&0.75));
        assert_eq!(
            loaded.window,
            Some(WindowGeometry {
                x: 120.0,
                y: 88.0,
                width: 400.0,
                height: 500.0
            })
        );
        assert_eq!(loaded.auto_scroll_speed, 65);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
