// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! World agent — translator for everything non-character.
//!
//! See: `docs/foundation/world_design.md`, `docs/foundation/system_architecture.md`
//!
//! Holds world model (geography, physics, time, material state).
//! Enforces hard constraints (genre physics). Gives voice to mountains,
//! rivers, weather, economies. Not just a constraint enforcer but a translator.
