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

            // Connect to the engine server in a background task.
            // Tauri 2's setup callback runs before the async runtime is fully
            // available on the main thread, so we cannot block_on here.
            // Instead we spawn the connection and manage state once connected.
            let client_state: ClientState = Arc::new(Mutex::new(None));
            app.manage(client_state.clone());

            let connect_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                let config = ClientConfig::from_env();
                info!(endpoint = %config.endpoint, "Connecting to storyteller server");

                match StorytellerClient::connect(config).await {
                    Ok(client) => {
                        info!("Connected to storyteller server");
                        *client_state.lock().await = Some(client);
                    }
                    Err(e) => {
                        tracing::error!("Failed to connect to storyteller server: {e}");
                        tracing::error!(
                            "Is the server running? Start it with: cargo run -p storyteller-server"
                        );
                    }
                }

                // Start log streaming on a separate client connection
                let log_config = ClientConfig::from_env();
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
                                    let _ = connect_handle.emit(LOG_CHANNEL, &log_entry);
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
            commands::get_profiles_for_genre,
            commands::get_archetypes_for_genre,
            commands::get_dynamics_for_genre,
            commands::get_names_for_genre,
            commands::get_settings_for_genre,
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
