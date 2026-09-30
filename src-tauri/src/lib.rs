mod config;
mod epub;
mod pdftext;
mod textload;

use config::{config_file_in, load_config_from, save_config_to, AppConfig, WindowGeometry};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, State, WebviewWindow};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

struct HideGuard {
    /// >0 表示挂起「失焦隐藏」。用计数而非布尔：文件对话框与快捷键唤起
    /// 两条路径各自加减，互不覆盖对方的状态。
    suspend_blur_hide: AtomicU32,
    /// true = 因鼠标移出而隐藏，由后台轮询光标、回到窗口范围即静默重显。
    /// 其他任何显隐路径都会清掉它，保证手动/失焦隐藏不会被鼠标扫过误唤回。
    hover_reshow: AtomicBool,
    config: Mutex<AppConfig>,
}

/// 窗口几何防抖落盘：拖动/缩放事件高频，静默 600ms 后写一次
struct GeometrySaver {
    pending: Mutex<Option<WindowGeometry>>,
    gen: AtomicU64,
}

const TOGGLE_SHORTCUT: &str = "CommandOrControl+Shift+H";

fn toggle_window(window: &WebviewWindow) -> tauri::Result<bool> {
    let armed = &window.state::<HideGuard>().hover_reshow;
    if window.is_visible().unwrap_or(false) {
        armed.store(false, Ordering::SeqCst);
        window.hide()?;
        let _ = window.emit("fr://visibility", false);
        Ok(false)
    } else {
        armed.store(false, Ordering::SeqCst);
        window.show()?;
        window.set_focus()?;
        let _ = window.emit("fr://visibility", true);
        Ok(true)
    }
}

/// 窗口刚唤起、焦点尚未稳定的窗口期内挂起「失焦隐藏」；350ms 后归还计数
fn suspend_briefly(app: &AppHandle) {
    if let Some(state) = app.try_state::<HideGuard>() {
        state.suspend_blur_hide.fetch_add(1, Ordering::SeqCst);
    }
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(350));
        if let Some(state) = handle.try_state::<HideGuard>() {
            let _ = state
                .suspend_blur_hide
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |c| c.checked_sub(1));
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
    let mut config = config;
    // 窗口几何由 Rust 侧维护：防止前端持有的旧几何把最新位置覆盖回去
    if let Ok(guard) = state.config.lock() {
        config.window = guard.window;
    }
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

/// 读取当前窗口几何（逻辑坐标）
fn snapshot_geometry(window: &WebviewWindow) -> Option<WindowGeometry> {
    let sf = window.scale_factor().ok()?;
    let pos = window.outer_position().ok()?.to_logical::<f64>(sf);
    let size = window.outer_size().ok()?.to_logical::<f64>(sf);
    Some(WindowGeometry {
        x: pos.x,
        y: pos.y,
        width: size.width,
        height: size.height,
    })
}

fn queue_geometry_save(app: &AppHandle, geo: WindowGeometry) {
    let Some(saver) = app.try_state::<GeometrySaver>() else {
        return;
    };
    *saver.pending.lock().unwrap() = Some(geo);
    let gen = saver.gen.fetch_add(1, Ordering::SeqCst) + 1;
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(600));
        let Some(saver) = handle.try_state::<GeometrySaver>() else {
            return;
        };
        if saver.gen.load(Ordering::SeqCst) != gen {
            return; // 后续移动/缩放已取代本次
        }
        let Some(geo) = saver.pending.lock().unwrap().take() else {
            return;
        };
        if let Ok(mut guard) = handle.state::<HideGuard>().config.lock() {
            guard.window = Some(geo);
            let cfg = guard.clone();
            drop(guard);
            if let Ok(path) = config_path_for(&handle) {
                let _ = save_config_to(&path, &cfg);
            }
        }
    });
}

/// 恢复上次窗口几何；完全落在所有显示器之外时放弃（外接屏被拔掉的场景）
fn restore_window_geometry(app: &AppHandle, window: &WebviewWindow) {
    let Some(geo) = app
        .state::<HideGuard>()
        .config
        .lock()
        .ok()
        .and_then(|c| c.window)
    else {
        return;
    };
    let Ok(sf) = window.scale_factor() else {
        return;
    };
    if !geometry_on_screen(window, &geo, sf) {
        return;
    }
    let _ = window.set_size(LogicalSize::new(geo.width, geo.height));
    let _ = window.set_position(LogicalPosition::new(geo.x, geo.y));
}

fn geometry_on_screen(window: &WebviewWindow, geo: &WindowGeometry, sf: f64) -> bool {
    let Ok(monitors) = window.available_monitors() else {
        return true;
    };
    if monitors.is_empty() {
        return true;
    }
    let (px, py) = (geo.x * sf, geo.y * sf);
    let (pw, ph) = (geo.width * sf, geo.height * sf);
    monitors.iter().any(|m| {
        let mx = m.position().x as f64;
        let my = m.position().y as f64;
        let mw = m.size().width as f64;
        let mh = m.size().height as f64;
        px < mx + mw && mx < px + pw && py < my + mh && my < py + ph
    })
}

fn resolve_config(app: &AppHandle) -> Result<AppConfig, String> {
    let path = config_path_for(app)?;
    load_config_from(&path)
}

fn bootstrap_config(app: &AppHandle) -> AppConfig {
    resolve_config(app).unwrap_or_default()
}

/// 系统对话框打开期间挂起「失焦隐藏」（前端成对调用 true/false）
#[tauri::command]
fn set_suspend_blur_hide(state: State<'_, HideGuard>, suspend: bool) {
    if suspend {
        state.suspend_blur_hide.fetch_add(1, Ordering::SeqCst);
    } else {
        let _ = state
            .suspend_blur_hide
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |c| c.checked_sub(1));
    }
}

/// 光标是否落在窗口范围内（四周内缩 inset 逻辑像素，防贴边抖动反复显隐）
fn cursor_inside(window: &WebviewWindow, inset: f64) -> bool {
    let Ok(cursor) = window.cursor_position() else {
        return false;
    };
    let Ok(sf) = window.scale_factor() else {
        return false;
    };
    let Ok(pos) = window.outer_position() else {
        return false;
    };
    let Ok(size) = window.outer_size() else {
        return false;
    };
    let c = cursor.to_logical::<f64>(sf);
    let p = pos.to_logical::<f64>(sf);
    let s = size.to_logical::<f64>(sf);
    c.x >= p.x + inset
        && c.x <= p.x + s.width - inset
        && c.y >= p.y + inset
        && c.y <= p.y + s.height - inset
}

/// 鼠标移出隐藏：藏起后轮询光标，回到窗口范围即重显并取回焦点
/// （非 key 窗口收不到悬停/键盘事件，不取回焦点会导致移出不隐藏、快捷键失灵）。
/// 失焦隐藏与 ⌘⇧H/托盘隐藏不走这里，避免摸鱼时鼠标扫过窗口原位误弹。
#[tauri::command]
fn hide_on_mouse_leave(app: AppHandle, window: WebviewWindow) -> Result<(), String> {
    app.state::<HideGuard>()
        .hover_reshow
        .store(true, Ordering::SeqCst);
    if let Err(e) = window.hide() {
        app.state::<HideGuard>()
            .hover_reshow
            .store(false, Ordering::SeqCst);
        return Err(e.to_string());
    }
    let _ = window.emit("fr://visibility", false);
    let handle = app.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(120));
        let Some(state) = handle.try_state::<HideGuard>() else {
            return;
        };
        if !state.hover_reshow.load(Ordering::SeqCst) {
            return; // 已被其他显隐路径取消
        }
        let Some(win) = handle.get_webview_window("main") else {
            return;
        };
        if win.is_visible().unwrap_or(true) {
            return; // 已被其他路径显示
        }
        if cursor_inside(&win, 4.0) {
            state.hover_reshow.store(false, Ordering::SeqCst);
            let _ = win.show();
            // 必须取回焦点：非 key 窗口收不到悬停/键盘事件，
            // 否则重显后「移出隐藏」失灵、E 键等快捷键全部无效
            let _ = win.set_focus();
            let _ = win.emit("fr://visibility", true);
            return;
        }
    });
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(HideGuard {
            suspend_blur_hide: AtomicU32::new(0),
            hover_reshow: AtomicBool::new(false),
            config: Mutex::new(AppConfig::default()),
        })
        .manage(GeometrySaver {
            pending: Mutex::new(None),
            gen: AtomicU64::new(0),
        })
        .invoke_handler(tauri::generate_handler![
            load_text,
            load_config_cmd,
            save_config_cmd,
            set_suspend_blur_hide,
            hide_on_mouse_leave
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
                            if toggle_window(&window).unwrap_or(false) {
                                suspend_briefly(&handle);
                            }
                        }
                    }
                },
            )?;

            if let Some(window) = app.get_webview_window("main") {
                restore_window_geometry(app.handle(), &window);
                let handle_ev = app.handle().clone();
                window.on_window_event(move |event| match event {
                    tauri::WindowEvent::Focused(false) => {
                        let Some(state) = handle_ev.try_state::<HideGuard>() else {
                            return;
                        };
                        if state.suspend_blur_hide.load(Ordering::SeqCst) > 0 {
                            return;
                        }
                        if let Some(win) = handle_ev.get_webview_window("main") {
                            let _ = win.hide();
                            let _ = win.emit("fr://visibility", false);
                        }
                    }
                    tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_) => {
                        if let Some(win) = handle_ev.get_webview_window("main") {
                            if let Some(geo) = snapshot_geometry(&win) {
                                queue_geometry_save(&handle_ev, geo);
                            }
                        }
                    }
                    _ => {}
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
                            if toggle_window(&window).unwrap_or(false) {
                                suspend_briefly(app);
                            }
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
