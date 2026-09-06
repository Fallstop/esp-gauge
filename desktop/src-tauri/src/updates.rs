use crate::{firmware_flash, releases, worker::Service};
use serde::Serialize;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_updater::UpdaterExt;

#[derive(Clone, Default, Serialize)]
pub struct UpdateStatus {
    pub checked: bool,
    pub app_version: String,
    pub firmware_version: String,
    pub busy: bool,
    pub stage: String,
    pub progress: f64,
    pub indeterminate: bool,
    pub operation: String,
    pub error: Option<String>,
}
#[derive(Default)]
pub struct Updates {
    status: Mutex<UpdateStatus>,
    release: Mutex<Option<(releases::Release, releases::Firmware)>>,
    busy: AtomicBool,
}
impl Updates {
    fn publish(&self, app: &AppHandle, edit: impl FnOnce(&mut UpdateStatus)) {
        let mut status = self.status.lock().unwrap();
        edit(&mut status);
        let _ = app.emit("updates", status.clone());
    }
    pub fn progress(&self, app: &AppHandle, stage: &str, progress: f64) {
        self.publish(app, |s| {
            s.stage = stage.into();
            s.progress = progress.clamp(0.0, 100.0);
            s.indeterminate = false;
        });
    }
    pub fn waiting(&self, app: &AppHandle, stage: &str) {
        self.publish(app, |s| {
            s.stage = stage.into();
            s.indeterminate = true;
        });
    }
    fn begin(&self, app: &AppHandle, operation: &str) -> Result<(), String> {
        self.busy
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map_err(|_| "An update is already in progress.")?;
        self.publish(app, |s| {
            s.busy = true;
            s.error = None;
            s.progress = 0.0;
            s.operation = operation.into();
            s.stage = "Preparing update".into();
            s.indeterminate = true;
        });
        Ok(())
    }
    fn finish(&self, app: &AppHandle, result: &Result<(), String>) {
        self.busy.store(false, Ordering::SeqCst);
        self.publish(app, |s| {
            s.busy = false;
            s.indeterminate = false;
            if result.is_ok() {
                s.progress = 100.0;
                s.stage = match s.operation.as_str() {
                    "firmware" => "Firmware installed · board verified",
                    "app" => "App installed",
                    _ => "",
                }
                .into();
            } else {
                s.stage = "Update could not finish".into();
            }
            s.error = result.as_ref().err().cloned();
        });
    }
    pub fn is_busy(&self) -> bool {
        self.busy.load(Ordering::SeqCst)
    }
}
#[tauri::command]
pub fn update_status(updates: State<'_, Updates>) -> UpdateStatus {
    updates.status.lock().unwrap().clone()
}
#[tauri::command]
pub async fn check_updates(app: AppHandle) -> Result<(), String> {
    let updates = app.state::<Updates>();
    updates.begin(&app, "check")?;
    updates.waiting(&app, "Checking releases");
    let handle = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let client = releases::client()?;
        let release = releases::Release::latest(&client)?;
        let firmware = release.firmware(&client)?;
        let updates = handle.state::<Updates>();
        updates.publish(&handle, |s| {
            s.firmware_version = firmware.version.clone();
        });
        *updates.release.lock().unwrap() = Some((release, firmware));
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())
    .and_then(|r| r);
    let result = match result {
        Ok(()) => match app.updater() {
            Ok(updater) => updater
                .check()
                .await
                .map(|update| {
                    updates.publish(&app, |s| {
                        s.checked = true;
                        s.app_version = update.map(|u| u.version).unwrap_or_default();
                    });
                })
                .map_err(|e| e.to_string()),
            Err(e) => Err(e.to_string()),
        },
        Err(e) => Err(e),
    };
    updates.finish(&app, &result);
    result
}
#[tauri::command]
pub async fn install_firmware(app: AppHandle, path: String) -> Result<(), String> {
    let updates = app.state::<Updates>();
    updates.begin(&app, "firmware")?;
    let handle = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let updates = handle.state::<Updates>();
        let (release, firmware) = updates
            .release
            .lock()
            .unwrap()
            .clone()
            .ok_or("Check for a release before installing firmware.")?;
        updates.waiting(&handle, "Starting firmware download");
        let data = firmware.download(&release, &releases::client()?, |done, total| {
            updates.progress(
                &handle,
                "Downloading and checking firmware",
                done as f64 / total.max(1) as f64 * 20.0,
            );
        })?;
        firmware_flash::install(&handle, &handle.state::<Service>(), &path, &firmware, &data)
    })
    .await
    .map_err(|e| e.to_string())
    .and_then(|r| r);
    updates.finish(&app, &result);
    result
}
#[tauri::command]
pub async fn install_app_update(app: AppHandle) -> Result<(), String> {
    let updates = app.state::<Updates>();
    updates.begin(&app, "app")?;
    let result = async {
        updates.waiting(&app, "Checking app update");
        let update = app
            .updater()
            .map_err(|e| e.to_string())?
            .check()
            .await
            .map_err(|e| e.to_string())?
            .ok_or("ESP Gauge is already up to date.")?;
        updates.waiting(&app, "Starting app download");
        let mut downloaded = 0_u64;
        let bytes = update
            .download(
                |length, total| {
                    downloaded += length as u64;
                    if total.is_none() {
                        updates.waiting(&app, "Downloading app");
                        return;
                    }
                    updates.progress(
                        &app,
                        "Downloading app",
                        total
                            .map(|n| downloaded as f64 / n.max(1) as f64 * 100.0)
                            .unwrap_or(0.0),
                    );
                },
                || {},
            )
            .await
            .map_err(|e| e.to_string())?;
        let service = app.state::<Service>().inner().clone();
        tauri::async_runtime::spawn_blocking(move || {
            service.execute(serde_json::json!({"op":"maintenance","active":true}))
        })
        .await
        .map_err(|e| e.to_string())??;
        updates.waiting(&app, "Installing app · restart follows");
        if let Err(error) = update.install(bytes) {
            let service = app.state::<Service>().inner().clone();
            let _ = tauri::async_runtime::spawn_blocking(move || {
                service.execute(serde_json::json!({"op":"maintenance","active":false}))
            })
            .await;
            return Err(error.to_string());
        }
        crate::EXIT_READY.store(true, Ordering::SeqCst);
        app.restart();
    }
    .await;
    updates.finish(&app, &result);
    result
}
