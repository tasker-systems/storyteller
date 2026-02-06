//! Authentication middleware.
//!
//! Provides auth extraction and validation that deployment crates can
//! optionally apply. The auth strategy (JWT, session tokens, platform-managed)
//! depends on the deployment target.
