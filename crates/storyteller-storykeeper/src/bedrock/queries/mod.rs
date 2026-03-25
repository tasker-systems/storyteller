// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Query sub-modules for `PostgresBedrock`.
//!
//! Each module handles one access pattern against the `bedrock` schema:
//! - `reference` — genres and trope families (no genre filter needed)
//! - `genre_context` — bulk context assembly for one genre (Task 9)
//! - `by_genre` — per-type lists filtered by genre slug (Task 10)
//! - `by_slug` — single-entity lookups by genre + entity slug (Task 11)
//! - `dimensions` — cross-primitive dimensional analysis (Task 12)
//! - `state_variables` — state variable definitions and interactions (Task 12)

pub mod by_genre;
pub mod by_slug;
pub mod dimensions;
pub mod genre_context;
pub mod reference;
pub mod state_variables;
