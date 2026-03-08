//! Tasker-core workflow dispatch via RabbitMQ.
//!
//! See: `docs/technical/event-system.md` (Deferred priority tier)
//!
//! Events classified as `Deferred` priority are dispatched to tasker-core
//! for asynchronous workflow processing. Results flow back via RabbitMQ
//! and are integrated into the truth set on arrival.
