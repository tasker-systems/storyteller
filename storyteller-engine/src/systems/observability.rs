//! Observability and turn transparency.
//!
//! See: `docs/technical/infrastructure-architecture.md` (three-layer observability)
//!
//! Three layers: system (OpenTelemetry traces), session (domain-language debug
//! events), player (progress streaming during turn processing).
//! TurnPhase events are observed by all layers but filtered/routed differently.
