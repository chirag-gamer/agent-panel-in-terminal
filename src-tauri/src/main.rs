// Prevents an extra console window on Windows in release builds (Tauri default).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    agent_panel_lib::run()
}
