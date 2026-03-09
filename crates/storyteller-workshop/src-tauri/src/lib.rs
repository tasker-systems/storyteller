mod commands;
mod engine_state;
mod session_log;

use tokio::sync::Mutex;

use crate::engine_state::EngineState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(None::<EngineState>))
        .invoke_handler(tauri::generate_handler![
            commands::test_ollama,
            commands::start_scene,
            commands::submit_input,
            commands::get_session_log,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
