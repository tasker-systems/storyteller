mod commands;
mod types;

use std::sync::Arc;

use tauri::{Emitter, Manager};
use tokio::sync::Mutex;
use tracing::{info, warn};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use storyteller_client::{ClientConfig, StorytellerClient};

use crate::commands::ClientState;
use crate::types::LogEntry;

const LOG_CHANNEL: &str = "workshop:logs";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load .env from the workspace root (where `bun tauri dev` runs)
    let _ = dotenvy::dotenv();

    // Set up tracing before anything else
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,storyteller_client=debug"));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_level(true)
        .compact();

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();

            // Connect to the engine server (blocking in setup — server is required)
            let config = ClientConfig::from_env();
            info!(endpoint = %config.endpoint, "Connecting to storyteller server");

            let client =
                tauri::async_runtime::block_on(async { StorytellerClient::connect(config).await })
                    .expect(
                        "Failed to connect to storyteller server. \
                 Is the server running? Start it with: \
                 cargo run -p storyteller-server",
                    );

            let client_state: ClientState = Arc::new(Mutex::new(client));
            app.manage(client_state);

            // Spawn background log streaming task
            let log_handle = handle.clone();
            let log_config = ClientConfig::from_env();
            tokio::spawn(async move {
                match StorytellerClient::connect(log_config).await {
                    Ok(mut log_client) => {
                        info!("Log streaming client connected");
                        match log_client.stream_logs(None, None).await {
                            Ok(mut stream) => {
                                while let Ok(Some(entry)) = stream.message().await {
                                    let log_entry = LogEntry {
                                        timestamp: entry.timestamp,
                                        level: entry.level,
                                        target: entry.target,
                                        message: entry.message,
                                        fields: entry.fields,
                                    };
                                    let _ = log_handle.emit(LOG_CHANNEL, &log_entry);
                                }
                                warn!("Log stream ended");
                            }
                            Err(e) => {
                                warn!("Failed to start log stream: {e}");
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to connect log streaming client: {e}");
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::check_health,
            commands::load_catalog,
            commands::get_genre_options,
            commands::compose_scene,
            commands::submit_input,
            commands::list_sessions,
            commands::resume_session,
            commands::get_scene_state,
            commands::get_prediction_history,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
