// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Workshop scene data — hardcoded struct literals for prototype testing.
//!
//! These are hand-crafted Rust data structures representing the scenes,
//! characters, and tensors described in `docs/workshop/`. They exist to
//! drive the prototype turn cycle without requiring file parsing,
//! deserialization, or a database.
//!
//! **This module is temporary scaffolding.** It will be replaced by
//! authored content loaded from the content resource store once the
//! prototype validates the agent orchestration approach.

#[cfg(test)]
pub mod the_flute_kept;
