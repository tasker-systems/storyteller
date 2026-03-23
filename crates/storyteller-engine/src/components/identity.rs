// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Entity identity component — who/what this entity is.
//!
//! See: `docs/technical/entity-model.md`

use bevy_ecs::prelude::*;
use storyteller_core::types::entity::{EntityId, EntityOrigin};

/// Core identity for a story entity.
#[derive(Debug, Component)]
pub struct EntityIdentity {
    /// Unique identifier across the session.
    pub id: EntityId,
    /// Display name for narrative rendering.
    pub name: String,
    /// How this entity came to exist.
    pub origin: EntityOrigin,
}
