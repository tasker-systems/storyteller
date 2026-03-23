// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Session state management — player session lifecycle.
//!
//! See: `docs/technical/infrastructure-architecture.md`
//!
//! Tracks active sessions, handles reconnection, and manages
//! session-to-instance affinity.
