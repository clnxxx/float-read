mod config;
mod epub;
mod pdftext;
mod textload;

use config::{config_file_in, load_config_from, save_config_to, AppConfig};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, State, WebviewWindow};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

struct HideGuard {
    suspend_blur_hide: AtomicBool,
    config: Mutex<AppConfig>,
}

const TOGGLE_SHORTCUT: &str = "CommandOrControl+Shift+H";

fn toggle_window(window: &WebviewWindow) -> tauri::Result<bool> {
    if window.is_visible().unwrap_or(false) {
        window.hide()?;
        Ok(false)
    } else {
        window.show()?;
        window.set_focus()?;
        Ok(true)
    }
}

fn schedule_unsuspend(app: AppHandle) {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(350));
        if let Some(state) = app.try_state::<HideGuard>() {
            state.suspend_blur_hide.store(false, Ordering::SeqCst);
        }
    });
}

/// 按扩展名分发：txt / epub / pdf
#[tauri::command]
fn load_text(path: String) -> Result<textload::LoadedText, String> {
    textload::load_book(&path)
}

#[tauri::command]
fn load_config_cmd(app: AppHandle, state: State<'_, HideGuard>) -> AppConfig {
    if let Ok(cfg) = state.config.lock() {
        return cfg.clone();
    }
    resolve_config(&app).unwrap_or_default()
}

#[tauri::command]
fn save_config_cmd(
    app: AppHandle,
    state: State<'_, HideGuard>,
    config: AppConfig,
) -> Result<(), String> {
    let path = config_path_for(&app)?;
    save_config_to(&path, &config)?;
    if let Ok(mut guard) = state.config.lock() {
        *guard = config.normalized();
    }
    Ok(())
}

fn config_path_for(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("config dir: {e}"))?;
    Ok(config_file_in(&dir))
}

fn resolve_config(app: &AppHandle) -> Result<AppConfig, String> {
    let path = config_path_for(app)?;
    load_config_from(&path)
}

fn bootstrap_config(app: &AppHandle) -> AppConfig {
    resolve_config(app).unwrap_or_default()
}

/// 系统对话框打开期间挂起「失焦隐藏」
#[tauri::command]
fn set_suspend_blur_hide(state: State<'_, HideGuard>, suspend: bool) {
    state.suspend_blur_hide.store(suspend, Ordering::SeqCst);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(HideGuard {
            suspend_blur_hide: AtomicBool::new(false),
            config: Mutex::new(AppConfig::default()),
        })
        .invoke_handler(tauri::generate_handler![
            load_text,
            load_config_cmd,
            save_config_cmd,
            set_suspend_blur_hide
        ])
        .setup(|app| {
            {
                let cfg = bootstrap_config(app.handle());
                if let Ok(mut guard) = app.state::<HideGuard>().config.lock() {
                    *guard = cfg;
                }
            }
            let handle = app.handle().clone();
            app.global_shortcut().on_shortcut(
                TOGGLE_SHORTCUT,
                move |_app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        if let Some(window) = handle.get_webview_window("main") {
                            let shown = toggle_window(&window).unwrap_or(false);
                            if let Some(state) = handle.try_state::<HideGuard>() {
                                state.suspend_blur_hide.store(shown, Ordering::SeqCst);
                            }
                            if shown {
                                schedule_unsuspend(handle.clone());
                            }
                        }
                    }
                },
            )?;

            if let Some(window) = app.get_webview_window("main") {
                let handle_blur = app.handle().clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::Focused(false) = event {
                        let Some(state) = handle_blur.try_state::<HideGuard>() else {
                            return;
                        };
                        if state.suspend_blur_hide.load(Ordering::SeqCst) {
                            return;
                        }
                        if let Some(win) = handle_blur.get_webview_window("main") {
                            let _ = win.hide();
                        }
                    }
                });
            }

            let show = MenuItem::with_id(app, "show", "显示 / 隐藏", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            let _tray = TrayIconBuilder::with_id("main-tray")
                .icon(
                    app.default_window_icon()
                        .cloned()
                        .expect("default window icon required"),
                )
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = toggle_window(&window);
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running float-read");
}
