mod commands;
mod engine_state;
mod events;
mod session;
mod session_log;
mod tracing_layer;

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::Mutex;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use storyteller_engine::scene_composer::SceneComposer;

use crate::engine_state::EngineState;
use crate::session::SessionStore;
use crate::tracing_layer::TauriTracingLayer;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load .env from the workspace root (where `bun tauri dev` runs)
    let _ = dotenvy::dotenv();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage({
            let data_path = std::env::var("STORYTELLER_DATA_PATH")
                .map(PathBuf::from)
                .expect("STORYTELLER_DATA_PATH must be set");
            let composer =
                SceneComposer::load(&data_path).expect("Failed to load scene descriptors");
            Arc::new(composer)
        })
        .manage({
            let workshop_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("workshop root")
                .to_path_buf();
            let store =
                SessionStore::new(&workshop_root).expect("Failed to initialize session store");
            Arc::new(store)
        })
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
            commands::load_catalog,
            commands::get_genre_options,
            commands::compose_scene,
            commands::list_sessions,
            commands::resume_session,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
