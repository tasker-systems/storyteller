//! Broadcast tracing layer for the `StreamLogs` RPC.
//!
//! A [`tracing_subscriber::Layer`] captures tracing events and sends them as
//! [`LogEntry`] proto messages over a
//! [`tokio::sync::broadcast`] channel. The `stream_logs`
//! RPC handler subscribes to this channel and streams entries to clients.

use std::collections::HashMap;
use std::fmt;

use tokio::sync::broadcast;
use tracing::field::{Field, Visit};
use tracing::Subscriber;
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

use crate::proto::LogEntry;

/// Broadcast sender carrying proto [`LogEntry`] messages.
///
/// Clone this to pass it around — `broadcast::Sender` is `Clone` and each clone
/// shares the same underlying channel.
pub type LogBroadcast = broadcast::Sender<LogEntry>;

/// Create a new [`LogBroadcast`] channel with a bounded buffer.
///
/// The buffer size of 256 provides headroom for bursty log output. Slow
/// subscribers that fall behind will receive `RecvError::Lagged` and can
/// skip missed entries.
pub fn create_log_broadcast() -> LogBroadcast {
    let (tx, _) = broadcast::channel(256);
    tx
}

/// A [`tracing_subscriber::Layer`] that forwards events to a [`LogBroadcast`].
///
/// Modeled after the workshop's `TauriTracingLayer` but outputs proto
/// [`LogEntry`] messages instead of Tauri events.
pub struct BroadcastTracingLayer {
    sender: LogBroadcast,
}

impl BroadcastTracingLayer {
    /// Create a new layer that sends log entries on `sender`.
    pub fn new(sender: LogBroadcast) -> Self {
        Self { sender }
    }
}

impl fmt::Debug for BroadcastTracingLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BroadcastTracingLayer")
            .field("receiver_count", &self.sender.receiver_count())
            .finish()
    }
}

/// Visitor that collects tracing event fields into a `HashMap<String, String>`.
struct FieldVisitor {
    fields: HashMap<String, String>,
    message: Option<String>,
}

impl FieldVisitor {
    fn new() -> Self {
        Self {
            fields: HashMap::new(),
            message: None,
        }
    }
}

impl Visit for FieldVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let val = format!("{value:?}");
        if field.name() == "message" {
            self.message = Some(val);
        } else {
            self.fields.insert(field.name().to_string(), val);
        }
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        } else {
            self.fields
                .insert(field.name().to_string(), value.to_string());
        }
    }
}

/// Targets that produce high-volume transport or runtime noise not useful
/// in the workshop Logs tab. Filtered at the layer level so the broadcast
/// channel doesn't overflow.
const NOISY_TARGETS: &[&str] = &[
    "h2",
    "hyper",
    "tower",
    "tonic::transport",
    "ort",     // ONNX Runtime internals (+NEW/+DROP Value, lifetime tracking)
    "reqwest", // HTTP client internals
    "log",     // bridged log crate entries ("log shouldn't retry")
];

impl<S: Subscriber> Layer<S> for BroadcastTracingLayer {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();

        // Skip TRACE entirely — too noisy for the workshop Logs tab.
        // Skip DEBUG from known-noisy targets (transport, runtime internals).
        let level = *metadata.level();
        let target = metadata.target();
        if level >= tracing::Level::TRACE {
            return;
        }
        if level >= tracing::Level::DEBUG
            && NOISY_TARGETS
                .iter()
                .any(|prefix| target.starts_with(prefix))
        {
            return;
        }

        let mut visitor = FieldVisitor::new();
        event.record(&mut visitor);

        let entry = LogEntry {
            level: metadata.level().to_string(),
            target: target.to_string(),
            message: visitor.message.unwrap_or_default(),
            timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            fields: visitor.fields,
        };

        // Ignore send errors — no subscribers is fine.
        let _ = self.sender.send(entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_log_broadcast_returns_sender_with_no_receivers() {
        let tx = create_log_broadcast();
        assert_eq!(tx.receiver_count(), 0);
    }

    #[test]
    fn subscribe_increments_receiver_count() {
        let tx = create_log_broadcast();
        let _rx = tx.subscribe();
        assert_eq!(tx.receiver_count(), 1);
    }

    #[test]
    fn broadcast_tracing_layer_debug_impl() {
        let tx = create_log_broadcast();
        let layer = BroadcastTracingLayer::new(tx);
        let debug = format!("{layer:?}");
        assert!(debug.contains("BroadcastTracingLayer"));
        assert!(debug.contains("receiver_count"));
    }

    #[test]
    fn field_visitor_starts_empty() {
        let visitor = FieldVisitor::new();
        assert!(visitor.message.is_none());
        assert!(visitor.fields.is_empty());
    }
}
