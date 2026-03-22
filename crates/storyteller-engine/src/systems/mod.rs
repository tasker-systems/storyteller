// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Bevy systems — the logic that runs each frame/turn.
//!
//! Systems implement the turn cycle, scene lifecycle, event processing,
//! entity promotion/demotion, and observability streaming.

pub mod arbitration;
pub mod entity_lifecycle;
pub mod event_pipeline;
pub mod observability;
pub mod rendering;
pub mod scene_lifecycle;
pub mod turn_cycle;
