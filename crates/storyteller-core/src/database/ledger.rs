//! Event ledger operations — append-only log of all narrative events.
//!
//! See: `docs/technical/event-system.md`, `docs/technical/infrastructure-architecture.md`
//!
//! Design decision: Command sourcing — player input is persisted to the event
//! ledger BEFORE processing begins. Server crashes are recoverable via
//! checkpoint + ledger replay.
