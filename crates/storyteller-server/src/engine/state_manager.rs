//! SWMR session state manager.
//!
//! Uses `DashMap` for lock-free session lookup and `ArcSwap` for
//! lock-free reads of runtime snapshots. Writers publish new snapshots
//! at pipeline phase boundaries.
//!
//! ## Concurrency model
//!
//! - **Multiple readers**: Any thread can call `get_runtime_snapshot` without
//!   blocking — `ArcSwap::load_full()` is a single atomic pointer load.
//! - **Single writer per session**: `update_runtime_snapshot` acquires a per-session
//!   `tokio::sync::Mutex` before publishing. Prevents two concurrent gRPC calls from
//!   racing on the same session's state.
//! - **Session map**: `DashMap` shards the lock, so readers and writers on different
//!   sessions never contend.

use std::sync::Arc;

use arc_swap::ArcSwap;
use dashmap::DashMap;

use super::types::{Composition, RuntimeSnapshot};

struct SessionState {
    composition: Arc<Composition>,
    runtime: ArcSwap<RuntimeSnapshot>,
    /// SWMR guard: only one writer may publish a snapshot at a time per session.
    /// Wrapped in `Arc` so it can be cloned out before the DashMap ref is dropped,
    /// avoiding holding a shard lock across `.await` boundaries.
    write_handle: Arc<tokio::sync::Mutex<()>>,
}

impl std::fmt::Debug for SessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionState")
            .field("composition", &"Arc<Composition>")
            .field("runtime", &"ArcSwap<RuntimeSnapshot>")
            .finish()
    }
}

/// Manages all active sessions with lock-free reads.
///
/// Construct once at server startup and share via `Arc<EngineStateManager>`.
#[derive(Debug, Default)]
pub struct EngineStateManager {
    sessions: DashMap<String, SessionState>,
}

impl EngineStateManager {
    /// Create a new, empty state manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new session with the given composition.
    ///
    /// If a session with this ID already exists, it is replaced.
    pub fn create_session(&self, session_id: &str, composition: Composition) {
        let state = SessionState {
            composition: Arc::new(composition),
            runtime: ArcSwap::from_pointee(RuntimeSnapshot::default()),
            write_handle: Arc::new(tokio::sync::Mutex::new(())),
        };
        self.sessions.insert(session_id.to_string(), state);
    }

    /// Get the immutable composition for a session.
    ///
    /// Returns `None` if the session does not exist.
    pub fn get_composition(&self, session_id: &str) -> Option<Arc<Composition>> {
        self.sessions.get(session_id).map(|s| s.composition.clone())
    }

    /// Get a snapshot of the current runtime state (lock-free).
    ///
    /// Returns `None` if the session does not exist.
    /// The returned `Arc` is cheap to clone — callers can hold it across `await`
    /// points without blocking writers.
    pub fn get_runtime_snapshot(&self, session_id: &str) -> Option<Arc<RuntimeSnapshot>> {
        self.sessions.get(session_id).map(|s| s.runtime.load_full())
    }

    /// Update the runtime snapshot atomically.
    ///
    /// Acquires the per-session write mutex to enforce SWMR: only one concurrent
    /// writer per session. The closure receives the current snapshot and returns
    /// the replacement. Does nothing if the session does not exist.
    pub async fn update_runtime_snapshot(
        &self,
        session_id: &str,
        f: impl FnOnce(&RuntimeSnapshot) -> RuntimeSnapshot,
    ) {
        // Clone the write_handle Arc before dropping the DashMap ref,
        // so we don't hold a shard lock across the .await boundary.
        let write_handle = match self.sessions.get(session_id) {
            Some(state) => Arc::clone(&state.write_handle),
            None => return,
        };
        // DashMap ref is dropped here — shard lock released.

        let _guard = write_handle.lock().await;

        // Re-acquire briefly for the atomic swap.
        if let Some(state) = self.sessions.get(session_id) {
            let current = state.runtime.load();
            let new_snapshot = f(&current);
            state.runtime.store(Arc::new(new_snapshot));
        }
    }

    /// Remove a session and release its resources.
    pub fn remove_session(&self, session_id: &str) {
        self.sessions.remove(session_id);
    }

    /// Check if a session exists.
    pub fn has_session(&self, session_id: &str) -> bool {
        self.sessions.contains_key(session_id)
    }

    /// List all active session IDs.
    ///
    /// Order is unspecified (DashMap shard iteration).
    pub fn session_ids(&self) -> Vec<String> {
        self.sessions.iter().map(|e| e.key().clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_composition() -> Composition {
        Composition {
            scene: serde_json::json!({"title": "test"}),
            characters: vec![],
            goals: None,
            intentions: None,
            selections: serde_json::json!({}),
        }
    }

    #[test]
    fn create_and_get_session() {
        let mgr = EngineStateManager::new();
        let session_id = "test-session";
        mgr.create_session(session_id, make_test_composition());

        assert!(mgr.get_composition(session_id).is_some());
        assert!(mgr.get_runtime_snapshot(session_id).is_some());
    }

    #[test]
    fn get_nonexistent_session_returns_none() {
        let mgr = EngineStateManager::new();
        assert!(mgr.get_composition("nope").is_none());
        assert!(mgr.get_runtime_snapshot("nope").is_none());
    }

    #[test]
    fn has_session_tracks_create_and_remove() {
        let mgr = EngineStateManager::new();
        assert!(!mgr.has_session("s1"));

        mgr.create_session("s1", make_test_composition());
        assert!(mgr.has_session("s1"));

        mgr.remove_session("s1");
        assert!(!mgr.has_session("s1"));
    }

    #[test]
    fn session_ids_lists_active_sessions() {
        let mgr = EngineStateManager::new();
        mgr.create_session("a", make_test_composition());
        mgr.create_session("b", make_test_composition());

        let mut ids = mgr.session_ids();
        ids.sort();
        assert_eq!(ids, vec!["a", "b"]);

        mgr.remove_session("a");
        assert_eq!(mgr.session_ids(), vec!["b"]);
    }

    #[test]
    fn fresh_snapshot_has_zero_turns() {
        let mgr = EngineStateManager::new();
        mgr.create_session("s1", make_test_composition());
        let snap = mgr.get_runtime_snapshot("s1").unwrap();
        assert_eq!(snap.turn_count, 0);
        assert!(snap.journal.entries.is_empty());
    }

    #[test]
    fn replace_existing_session_resets_state() {
        let mgr = EngineStateManager::new();
        mgr.create_session("s1", make_test_composition());

        // Simulate some state via a blocking runtime for synchronous update
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(mgr.update_runtime_snapshot("s1", |snap| {
            let mut new = snap.clone();
            new.turn_count = 3;
            new
        }));
        assert_eq!(mgr.get_runtime_snapshot("s1").unwrap().turn_count, 3);

        // Creating a new session with the same ID resets state
        mgr.create_session("s1", make_test_composition());
        assert_eq!(mgr.get_runtime_snapshot("s1").unwrap().turn_count, 0);
    }

    #[tokio::test]
    async fn update_runtime_snapshot_persists() {
        let mgr = EngineStateManager::new();
        mgr.create_session("s1", make_test_composition());

        mgr.update_runtime_snapshot("s1", |snap| {
            let mut new = snap.clone();
            new.turn_count = 5;
            new
        })
        .await;

        let snap = mgr.get_runtime_snapshot("s1").unwrap();
        assert_eq!(snap.turn_count, 5);
        assert!(snap.journal.entries.is_empty());
    }

    #[tokio::test]
    async fn update_nonexistent_session_is_noop() {
        let mgr = EngineStateManager::new();
        // Should not panic
        mgr.update_runtime_snapshot("ghost", |snap| snap.clone())
            .await;
    }

    #[tokio::test]
    async fn concurrent_reads_during_write() {
        let mgr = Arc::new(EngineStateManager::new());
        mgr.create_session("s1", make_test_composition());

        let mgr2 = mgr.clone();
        let reader = tokio::spawn(async move {
            for _ in 0..100 {
                let _ = mgr2.get_runtime_snapshot("s1");
            }
        });

        // Writer updates snapshot while reader spins
        mgr.update_runtime_snapshot("s1", |snap| {
            let mut new = snap.clone();
            new.turn_count = 5;
            new
        })
        .await;

        reader.await.unwrap();

        let snap = mgr.get_runtime_snapshot("s1").unwrap();
        assert_eq!(snap.turn_count, 5);
    }

    #[tokio::test]
    async fn sequential_writes_accumulate_state() {
        let mgr = EngineStateManager::new();
        mgr.create_session("s1", make_test_composition());

        for i in 1..=5 {
            mgr.update_runtime_snapshot("s1", move |snap| {
                let mut new = snap.clone();
                new.turn_count = i;
                new
            })
            .await;
        }

        let snap = mgr.get_runtime_snapshot("s1").unwrap();
        assert_eq!(snap.turn_count, 5);
    }
}
