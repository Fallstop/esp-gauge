#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#[cfg(target_os = "macos")]
mod mac_menu;
mod metrics;
mod model;
mod transport;
mod worker;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, State,
};
use tauri_plugin_autostart::ManagerExt;
use worker::{Service, Snapshot};
static EXIT_READY: AtomicBool = AtomicBool::new(false);

#[tauri::command]
fn login_start(app: tauri::AppHandle, enabled: Option<bool>) -> Result<bool, String> {
    let manager = app.autolaunch();
    if let Some(enabled) = enabled {
        if enabled {
            manager.enable()
        } else {
            manager.disable()
        }
        .map_err(|e| e.to_string())?;
    }
    manager.is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
fn snapshot(service: State<'_, Service>) -> Snapshot {
    service.state.lock().unwrap().clone()
}
#[tauri::command]
fn show_window(app: tauri::AppHandle) {
    show(&app);
}
#[tauri::command]
async fn quit(app: tauri::AppHandle, service: State<'_, Service>) -> Result<(), String> {
    let service = service.inner().clone();
    let _ = tauri::async_runtime::spawn_blocking(move || service.execute(json!({"op":"release"})))
        .await;
    EXIT_READY.store(true, Ordering::SeqCst);
    app.exit(0);
    Ok(())
}
#[tauri::command]
async fn command(
    window: tauri::WebviewWindow,
    service: State<'_, Service>,
    command: Value,
) -> Result<Value, String> {
    if command["op"] == "calibrate" && !window.is_visible().unwrap_or(false) {
        return Err("Open the gauge window to calibrate.".into());
    }
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || service.execute(command))
        .await
        .map_err(|e| e.to_string())?
}
fn show(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}
fn main() {
    if std::env::args().any(|arg| arg == "--diagnose") {
        let mut metrics = metrics::Metrics::new();
        std::thread::sleep(std::time::Duration::from_millis(800));
        println!(
            "{}",
            json!({"version":env!("CARGO_PKG_VERSION"), "platform":std::env::consts::OS, "metrics":metrics.sample(), "usb_candidates":transport::candidates()})
        );
        return;
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--background"]),
        ))
        .plugin(tauri_plugin_single_instance::init(|app, _, _| show(app)))
        .setup(|app| {
            app.manage(Service::start(app.handle().clone()));
            #[cfg(target_os = "macos")]
            mac_menu::install(app)?;
            let open = MenuItem::with_id(app, "open", "Open ESP Gauge", true, None::<&str>)?;
            let pause =
                MenuItem::with_id(app, "pause", "Pause / resume gauges", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit ESP Gauge", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &pause, &quit])?;
            TrayIconBuilder::with_id("gauge")
                .icon(tauri::image::Image::from_bytes(include_bytes!(
                    "../icons/tray.png"
                ))?)
                .icon_as_template(true)
                .tooltip("ESP Gauge")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    if matches!(
                        event,
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        }
                    ) {
                        show(tray.app_handle());
                    }
                })
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show(app),
                    "pause" => {
                        let s = app.state::<Service>().inner().clone();
                        std::thread::spawn(move || {
                            let state = s.state.lock().unwrap().clone();
                            let _ = s.execute(
                                json!({"op":"pause","paused":!state.paused,"device":state.device}),
                            );
                        });
                    }
                    "quit" => {
                        let _ = tauri::Emitter::emit(app, "quit-requested", ());
                    }
                    _ => {}
                })
                .build(app)?;
            if std::env::args().any(|arg| arg == "--background") {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
            Ok(())
        })
        .on_menu_event(|app, event| {
            if event.id.as_ref() == "app_quit" {
                let _ = tauri::Emitter::emit(app, "quit-requested", ());
            } else if event.id.as_ref() == "app_open" {
                show(app);
            }
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
                let s = window.state::<Service>().inner().clone();
                std::thread::spawn(move || {
                    let device = s.state.lock().unwrap().device.clone();
                    let _ = s.execute(json!({"op":"calibrate_end","device":device}));
                });
            }
        })
        .invoke_handler(tauri::generate_handler![
            snapshot,
            command,
            login_start,
            show_window,
            quit
        ])
        .build(tauri::generate_context!())
        .expect("Unable to start ESP Gauge")
        .run(|app, event| match event {
            tauri::RunEvent::ExitRequested { api, .. } => {
                if !EXIT_READY.load(Ordering::SeqCst) {
                    api.prevent_exit();
                    let _ = tauri::Emitter::emit(app, "quit-requested", ());
                }
            }
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => show(app),
            _ => {}
        });
}
