// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Observability and turn transparency.
//!
//! See: `docs/technical/infrastructure-architecture.md` (three-layer observability)
//!
//! Three layers: system (OpenTelemetry traces), session (domain-language debug
//! events), player (progress streaming during turn processing).
//! TurnPhase events are observed by all layers but filtered/routed differently.
