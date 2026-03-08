//! Apache AGE graph query operations.
//!
//! See: `docs/technical/infrastructure-architecture.md`, `docs/technical/technical-stack.md`
//!
//! All graph data (relational web, narrative graph, setting topology) lives in
//! PostgreSQL with Apache AGE providing openCypher query support. No separate
//! graph database needed.

pub mod narrative;
pub mod relational_web;
pub mod settings;
