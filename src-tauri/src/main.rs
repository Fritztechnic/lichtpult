#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use lichtpult::api::Core;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
fn api(core: State<'_, Arc<Core>>, method: String, body: String) -> Result<String, String> {
    core.call(&method, &body)
}

fn main() {
    let core = Core::start();
    tauri::Builder::default()
        .manage(core)
        .invoke_handler(tauri::generate_handler![api])
        .run(tauri::generate_context!())
        .expect("error while running Lichtpult");
}
