// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Entity promotion, demotion, and decay.
//!
//! See: `docs/technical/entity-model.md`
//!
//! Entities can be promoted (prop → presence → character) or demoted
//! based on narrative relevance. Ephemeral entities decay when no longer
//! needed. Budget management ensures the system doesn't exceed token limits.
