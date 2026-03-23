// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Re-exports from storyteller-composer for backward compatibility.
//!
//! Scene composition logic now lives in the `storyteller-composer` crate.
//! This module re-exports the public API so existing consumers
//! (workshop, tests) don't need to change their imports immediately.

pub use storyteller_composer::*;
