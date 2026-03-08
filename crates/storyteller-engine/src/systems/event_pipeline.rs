//! Event classification and truth set management.
//!
//! See: `docs/technical/event-system.md`
//!
//! Two-track classification: factual (fast, deterministic) and interpretive
//! (may use LLM, asynchronous). The truth set is a materialized view
//! reconstructable from the event ledger.
