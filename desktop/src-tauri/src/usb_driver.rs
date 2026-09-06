#[derive(serde::Serialize)]
pub struct DriverStatus {
    supported: bool,
    installed: Option<bool>,
}

#[tauri::command]
pub async fn usb_driver_status() -> DriverStatus {
    #[cfg(windows)]
    let installed = tauri::async_runtime::spawn_blocking(crate::windows_driver::available)
        .await
        .ok()
        .and_then(Result::ok);
    #[cfg(not(windows))]
    let installed = None;
    DriverStatus {
        supported: cfg!(windows),
        installed,
    }
}

#[tauri::command]
pub async fn install_usb_driver(app: tauri::AppHandle) -> Result<(), String> {
    #[cfg(windows)]
    {
        use tauri::Manager;
        static INSTALLING: std::sync::atomic::AtomicBool =
            std::sync::atomic::AtomicBool::new(false);
        if app.state::<crate::updates::Updates>().is_busy() {
            return Err(
                "Wait for the current update to finish before opening USB driver setup.".into(),
            );
        }
        let path = app
            .path()
            .resolve("drivers/CH341SER.EXE", tauri::path::BaseDirectory::Resource)
            .map_err(|error| error.to_string())?;
        if INSTALLING.swap(true, std::sync::atomic::Ordering::SeqCst) {
            return Err("USB driver setup is already open.".into());
        }
        let result =
            tauri::async_runtime::spawn_blocking(move || crate::windows_driver::install(path))
                .await
                .map_err(|error| error.to_string())
                .and_then(|result| result);
        INSTALLING.store(false, std::sync::atomic::Ordering::SeqCst);
        result
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        Err("The bundled USB driver installer is for Windows.".into())
    }
}
