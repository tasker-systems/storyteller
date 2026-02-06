//! Checkpoint operations — periodic snapshots for fast recovery.
//!
//! See: `docs/technical/infrastructure-architecture.md`
//!
//! Checkpoints capture the full in-memory state (truth set, entity states,
//! scene context) at regular intervals. Recovery replays only events since
//! the last checkpoint.
