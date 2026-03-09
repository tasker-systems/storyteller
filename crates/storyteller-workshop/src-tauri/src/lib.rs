mod commands;
mod engine_state;
mod events;
mod session_log;
mod tracing_layer;

use tokio::sync::Mutex;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use crate::engine_state::EngineState;
use crate::tracing_layer::TauriTracingLayer;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load .env from the workspace root (where `bun tauri dev` runs)
    let _ = dotenvy::dotenv();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(None::<EngineState>))
        .setup(|app| {
            // Set up tracing with both a console layer and our Tauri layer.
            // We do this in setup() so we have access to AppHandle.
            let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                EnvFilter::new(
                    "info,storyteller_engine::inference=debug,storyteller_engine::agents=debug",
                )
            });

            let fmt_layer = tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_level(true)
                .compact();

            let tauri_layer = TauriTracingLayer::new(app.handle().clone());

            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt_layer)
                .with(tauri_layer)
                .init();

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::check_llm,
            commands::start_scene,
            commands::submit_input,
            commands::get_session_log,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
