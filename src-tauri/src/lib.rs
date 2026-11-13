//! Agent Panel — Tauri backend. Reuses the proven engine modules from the
//! fltk project unchanged: config, settings (JSONC-preserving editor),
//! detect (adapters + model discovery), store (config IO + backups).

pub mod commands;
pub mod config;
pub mod detect;
pub mod settings;
pub mod store;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::detect_agents,
            commands::discover_models,
            commands::compose_entry,
            commands::load_config,
            commands::save_config,
            commands::apply_config,
            commands::restore_backup,
            commands::quit_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
