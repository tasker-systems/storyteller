//! Turn pipeline orchestration.
//!
//! See: `docs/technical/agent-message-catalog.md` (turn cycle)
//!
//! The turn cycle: player input → classification → agent deliberation →
//! reconciliation → narrative rendering → truth set update.
//! Player input is broadcast to ALL agents in parallel.
