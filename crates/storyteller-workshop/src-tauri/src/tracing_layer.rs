//! Custom tracing layer that forwards log events to the Tauri frontend.
//!
//! Events are serialized to JSON and emitted on the `"workshop:logs"` channel.
//! The frontend renders them in the Logs inspector tab.

use std::fmt;
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tracing::field::{Field, Visit};
use tracing::Subscriber;
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

/// Tauri event channel for log entries.
pub const LOG_EVENT_CHANNEL: &str = "workshop:logs";

/// A single structured log entry emitted to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
    pub fields: serde_json::Value,
}

/// Custom tracing layer that emits log events as Tauri events.
///
/// Clones the `AppHandle` and uses it to emit JSON-serialized log entries
/// on the `"workshop:logs"` channel.
pub struct TauriTracingLayer {
    app_handle: Arc<AppHandle>,
}

impl TauriTracingLayer {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle: Arc::new(app_handle),
        }
    }
}

/// Visitor that collects tracing event fields into a JSON map.
struct JsonVisitor {
    fields: serde_json::Map<String, serde_json::Value>,
    message: Option<String>,
}

impl JsonVisitor {
    fn new() -> Self {
        Self {
            fields: serde_json::Map::new(),
            message: None,
        }
    }
}

impl Visit for JsonVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let val = format!("{value:?}");
        if field.name() == "message" {
            self.message = Some(val);
        } else {
            self.fields
                .insert(field.name().to_string(), serde_json::Value::String(val));
        }
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.fields
            .insert(field.name().to_string(), serde_json::Value::from(value));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.fields
            .insert(field.name().to_string(), serde_json::Value::from(value));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.fields
            .insert(field.name().to_string(), serde_json::Value::from(value));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.fields
            .insert(field.name().to_string(), serde_json::Value::from(value));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        } else {
            self.fields.insert(
                field.name().to_string(),
                serde_json::Value::String(value.to_string()),
            );
        }
    }
}

impl<S: Subscriber> Layer<S> for TauriTracingLayer {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let target = metadata.target();

        // Filter: only storyteller crates, with level thresholds
        let dominated_by = |prefix: &str| target.starts_with(prefix);
        let dominated = dominated_by("storyteller_engine") || dominated_by("storyteller_workshop");
        if !dominated {
            return;
        }

        let level = *metadata.level();
        let deep_target = dominated_by("storyteller_engine::inference")
            || dominated_by("storyteller_engine::agents");

        // DEBUG only for inference and agents targets; INFO+ for everything else
        if !deep_target && level > tracing::Level::INFO {
            return;
        }

        let mut visitor = JsonVisitor::new();
        event.record(&mut visitor);

        let entry = LogEntry {
            timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            level: level.to_string(),
            target: target.to_string(),
            message: visitor.message.unwrap_or_default(),
            fields: serde_json::Value::Object(visitor.fields),
        };

        let _ = self.app_handle.emit(LOG_EVENT_CHANNEL, &entry);
    }
}
