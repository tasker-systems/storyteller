//! PostgreSQL database operations.
//!
//! See: `docs/technical/infrastructure-architecture.md`
//!
//! Handles the event ledger, checkpoint read/write, and session state management.
//! All database access flows through sqlx with the PostgreSQL driver.

pub mod checkpoint;
pub mod ledger;
pub mod session;
