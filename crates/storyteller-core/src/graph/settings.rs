// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Setting topology queries — spatial/conceptual structure of the story world.
//!
//! See: `docs/technical/scene-model.md` (scene-setting distinction)
//!
//! Settings are re-enterable locations/contexts. Scenes happen *within* settings
//! but are not the same thing — the same setting hosts different scenes at
//! different times, and its state may change between visits.
