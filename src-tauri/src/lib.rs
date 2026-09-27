mod database;
mod lessons;
mod linux_update;

use std::{path::PathBuf, sync::{Arc, Mutex}};
use database::{Database, Lesson, Progress, ProgressInput};
use tauri::{Emitter, Manager, State};

struct Storage {
    directory: PathBuf,
    database: Mutex<Option<Database>>,
}

impl Storage {
    fn access<T>(&self, operation: impl FnOnce(&mut Database) -> database::Result<T>) -> database::Result<T> {
        let mut guard = self.database.lock().map_err(|_| "Storage unavailable. Restart the app.".to_string())?;
        if guard.is_none() { *guard = Some(Database::open(&self.directory)?); }
        operation(guard.as_mut().ok_or("Storage unavailable.")?)
    }
}

// Database work is serialized off the webview/UI thread. Failed opens can be
// retried from the interface without leaving the application stuck at startup.
async fn with_storage<T: Send + 'static>(
    storage: Arc<Storage>,
    operation: impl FnOnce(&mut Database) -> database::Result<T> + Send + 'static,
) -> database::Result<T> {
    tauri::async_runtime::spawn_blocking(move || storage.access(operation))
        .await.map_err(|_| "Storage operation could not complete.".to_string())?
}

#[tauri::command]
async fn get_lessons(state: State<'_, Arc<Storage>>) -> database::Result<Vec<Lesson>> {
    with_storage(state.inner().clone(), |db| db.lessons()).await
}

#[tauri::command]
async fn save_progress(input: ProgressInput, state: State<'_, Arc<Storage>>) -> database::Result<()> {
    with_storage(state.inner().clone(), move |db| db.save_progress(input)).await
}

#[tauri::command]
async fn get_progress(username: String, limit: Option<i64>, before: Option<i64>, state: State<'_, Arc<Storage>>) -> database::Result<Vec<Progress>> {
    with_storage(state.inner().clone(), move |db| db.progress(&username, limit.unwrap_or(100), before)).await
}

#[tauri::command]
fn stop_application(app: tauri::AppHandle) { app.exit(0); }

#[tauri::command]
fn restart_application(app: tauri::AppHandle) { app.restart(); }

#[tauri::command]
fn app_version() -> String { env!("CARGO_PKG_VERSION").to_string() }

/// Describes whether this copy supports Linux `~/.local` self-updates.
/// Always `managed: false` off Linux; the frontend then uses the stock
/// Tauri updater flow (Windows/macOS).
#[tauri::command]
fn linux_install_info() -> linux_update::LinuxInstallInfo {
    linux_update::describe_install_target()
}

/// Download, signature-verify, and install `expected_version` over the
/// `~/.local`-style prefix the app is running from. Progress is emitted on
/// `linux-update-progress`; the frontend restarts the app afterwards.
#[tauri::command]
async fn install_linux_update(
    app: tauri::AppHandle,
    expected_version: String,
) -> Result<(), String> {
    if std::env::consts::OS != "linux" {
        return Err("Linux self-updates only apply on Linux.".to_string());
    }
    let current_exe = std::env::current_exe()
        .map_err(|_| "Could not locate the running application.".to_string())?;
    if linux_update::is_dev_build_path(&current_exe) {
        return Err("Updates are only supported for installed builds, not development builds.".to_string());
    }
    let prefix = linux_update::install_prefix_from_exe(&current_exe)
        .ok_or("Could not determine the install location.".to_string())?;
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        linux_update::perform_install(&prefix, &expected_version, &|progress| {
            let _ = handle.emit("linux-update-progress", progress);
        })
    })
    .await
    .map_err(|_| "The update task could not complete.".to_string())?
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let mut directory = app.path().app_data_dir()?;
            if cfg!(debug_assertions) { directory = directory.join("development"); }
            app.manage(Arc::new(Storage { directory, database: Mutex::new(None) }));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_lessons, save_progress, get_progress, stop_application, restart_application, app_version, linux_install_info, install_linux_update])
        .run(tauri::generate_context!())
        .expect("failed to run Colemak-DH Tutor");
}
