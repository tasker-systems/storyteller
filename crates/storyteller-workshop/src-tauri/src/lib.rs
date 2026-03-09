mod commands;
mod engine_state;
mod events;
mod session_log;

use tokio::sync::Mutex;

use crate::engine_state::EngineState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load .env from the workspace root (where `bun tauri dev` runs)
    let _ = dotenvy::dotenv();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(None::<EngineState>))
        .invoke_handler(tauri::generate_handler![
            commands::check_llm,
            commands::start_scene,
            commands::submit_input,
            commands::get_session_log,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
