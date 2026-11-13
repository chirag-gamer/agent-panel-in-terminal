//! Tauri commands exposing the engine modules to the front end.
//! Least privilege: only these commands are reachable from the webview
//! (verified: custom `invoke_handler` commands need no capability entry;
//! no shell/fs plugins are installed).

use crate::{config::PanelConfig, detect, store};
use serde::Serialize;
use tauri::AppHandle;

#[derive(Serialize)]
pub struct ApplyResult {
    pub backup_path: String,
    pub backup_sha256: String,
    pub new_sha256: String,
}

#[tauri::command]
pub fn detect_agents() -> Vec<detect::Detection> {
    detect::detect_all()
}

#[tauri::command]
pub fn discover_models(adapter_id: String) -> Vec<String> {
    detect::ADAPTERS
        .iter()
        .find(|a| a.id == adapter_id)
        .map(detect::discover_models)
        .unwrap_or_default()
}

#[tauri::command]
pub fn compose_entry(
    adapter_id: String,
    model: Option<String>,
    effort: Option<String>,
) -> Result<crate::config::AgentEntry, String> {
    detect::ADAPTERS
        .iter()
        .find(|a| a.id == adapter_id)
        .map(|a| detect::entry_from(a, model.as_deref(), effort.as_deref()))
        .ok_or_else(|| format!("unknown adapter: {adapter_id}"))
}

#[tauri::command]
pub fn load_config() -> PanelConfig {
    store::load_config()
}

#[tauri::command]
pub fn save_config(cfg: PanelConfig) -> Result<(), String> {
    store::save_config(&cfg)
}

#[tauri::command]
pub fn apply_config(cfg: PanelConfig) -> Result<ApplyResult, String> {
    let mut cfg = cfg;
    if cfg.customize_commandline.is_none() {
        // The "Agent settings" profile must always launch the GUI exe — never
        // a CLI binary — so pin it to this process's own path at apply time.
        cfg.customize_commandline = std::env::current_exe().ok().map(|p| {
            format!("cmd.exe /c start \"\" \"{}\"", p.display())
        });
    }
    let (backup, backup_hash, new_hash) = store::compile_and_write(&cfg)?;
    Ok(ApplyResult {
        backup_path: backup.display().to_string(),
        backup_sha256: backup_hash,
        new_sha256: new_hash,
    })
}

#[tauri::command]
pub fn restore_backup() -> Result<String, String> {
    store::restore_latest_backup()
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}
