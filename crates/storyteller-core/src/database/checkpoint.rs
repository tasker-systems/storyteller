// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Checkpoint operations — periodic snapshots for fast recovery.
//!
//! See: `docs/technical/infrastructure-architecture.md`
//!
//! Checkpoints capture the full in-memory state (truth set, entity states,
//! scene context) at regular intervals. Recovery replays only events since
//! the last checkpoint.
