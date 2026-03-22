// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Persistence profile — how an entity survives scene boundaries.
//!
//! See: `docs/technical/entity-model.md`

use bevy_ecs::prelude::*;
use storyteller_core::types::entity::PersistenceMode;

/// Controls how an entity persists across scene transitions.
#[derive(Debug, Component)]
pub struct PersistenceProfile {
    /// Whether this entity is permanent, scene-local, or ephemeral.
    pub mode: PersistenceMode,
}
