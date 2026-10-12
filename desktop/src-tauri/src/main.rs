#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod audio;
mod firmware_flash;
mod hardware;
#[cfg(target_os = "macos")]
mod mac_menu;
mod metrics;
mod model;
mod providers;
mod releases;
mod transport;
mod updates;
mod usb_driver;
#[cfg(any(windows, test))]
mod window_layout;
#[cfg(any(windows, test))]
mod windows_driver;
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
// Set while the gauge window closes itself, so the last-window exit request keeps the tray running.
static CLOSING: AtomicBool = AtomicBool::new(false);

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
async fn show_window(app: tauri::AppHandle) {
    show(&app);
}
// Destroys the webview rather than hiding it, so a closed window costs no CPU or memory.
#[tauri::command]
fn close_window(app: tauri::AppHandle, service: State<'_, Service>) {
    if let Some(window) = app.get_webview_window("main") {
        CLOSING.store(true, Ordering::SeqCst);
        let _ = window.destroy();
    }
    #[cfg(target_os = "macos")]
    let _ = app.set_dock_visibility(false);
    let s = service.inner().clone();
    std::thread::spawn(move || {
        let device = s.state.lock().unwrap().device.clone();
        let _ = s.execute(json!({"op":"calibrate_end","device":device}));
    });
}
#[tauri::command]
async fn quit(app: tauri::AppHandle, service: State<'_, Service>) -> Result<(), String> {
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || exit(&app, &service))
        .await
        .map_err(|e| e.to_string())?
}
fn exit(app: &tauri::AppHandle, service: &Service) -> Result<(), String> {
    if app.state::<updates::Updates>().is_busy() {
        return Err("Wait for the update to finish before quitting.".into());
    }
    let _ = service.execute(json!({"op":"release"}));
    EXIT_READY.store(true, Ordering::SeqCst);
    app.exit(0);
    Ok(())
}
// The open window flushes unsaved edits before quitting; with no window there is nothing to flush.
fn request_quit(app: &tauri::AppHandle) {
    if app.get_webview_window("main").is_some() {
        let _ = tauri::Emitter::emit(app, "quit-requested", ());
    } else {
        let app = app.clone();
        std::thread::spawn(move || {
            let service = app.state::<Service>().inner().clone();
            let _ = exit(&app, &service);
        });
    }
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
    CLOSING.store(false, Ordering::SeqCst);
    #[cfg(target_os = "macos")]
    let _ = app.set_dock_visibility(true);
    let window = match app.get_webview_window("main") {
        Some(w) => w,
        None => match open_window(app) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("Unable to open the gauge window: {e}");
                return;
            }
        },
    };
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
}
fn open_window(app: &tauri::AppHandle) -> tauri::Result<tauri::WebviewWindow> {
    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|w| w.label == "main")
        .expect("main window config")
        .clone();
    let window = tauri::WebviewWindowBuilder::from_config(app, &config)?.build()?;
    #[cfg(windows)]
    let _ = window_layout::fit_work_area(&window);
    Ok(window)
}
fn main() {
    #[cfg(windows)]
    if std::env::args().any(|arg| arg == "--check-usb-driver") {
        std::process::exit(match windows_driver::available() {
            Ok(true) => 0,
            Ok(false) => 1,
            Err(_) => 2,
        });
    }
    if std::env::args().any(|arg| arg == "--diagnose") {
        let mut metrics = metrics::Metrics::new();
        std::thread::sleep(std::time::Duration::from_millis(800));
        let providers = providers::diagnose();
        let hardware = hardware::Hardware::new().sample();
        println!(
            "{}",
            json!({"version":env!("CARGO_PKG_VERSION"), "platform":std::env::consts::OS, "metrics":metrics.sample(), "usb_candidates":transport::candidates(),"sources":providers.sources,"provider_metrics":providers.values,"hardware_sources":hardware.sources,"hardware_metrics":hardware.values})
        );
        return;
    }
    tauri::Builder::default()
        .manage(updates::Updates::default())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--background"]),
        ))
        .plugin(tauri_plugin_single_instance::init(|app, _, _| show(app)))
        .setup(|app| {
            #[cfg(windows)]
            std::thread::spawn(windows_driver::remove_legacy_shortcut);
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
                    "quit" => request_quit(app),
                    _ => {}
                })
                .build(app)?;
            if std::env::args().any(|arg| arg == "--background") {
                #[cfg(target_os = "macos")]
                let _ = app.handle().set_dock_visibility(false);
            } else {
                show(app.handle());
            }
            Ok(())
        })
        .on_menu_event(|app, event| {
            if event.id.as_ref() == "app_quit" {
                request_quit(app);
            } else if event.id.as_ref() == "app_open" {
                show(app);
            }
        })
        .on_window_event(|window, event| {
            // The page saves pending edits, then calls close_window.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if !CLOSING.load(Ordering::SeqCst) {
                    api.prevent_close();
                    let _ = tauri::Emitter::emit(window, "close-requested", ());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            providers::products::search_products,
            providers::products::product_stores,
            usb_driver::usb_driver_status,
            usb_driver::install_usb_driver,
            updates::check_updates,
            updates::update_status,
            updates::install_firmware,
            updates::install_app_update,
            snapshot,
            command,
            login_start,
            show_window,
            close_window,
            quit
        ])
        .build(tauri::generate_context!())
        .expect("Unable to start ESP Gauge")
        .run(|app, event| match event {
            tauri::RunEvent::ExitRequested { code, api, .. }
                if !EXIT_READY.load(Ordering::SeqCst) =>
            {
                api.prevent_exit();
                if !(code.is_none() && CLOSING.swap(false, Ordering::SeqCst)) {
                    request_quit(app);
                }
            }
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => show(app),
            _ => {}
        });
}
