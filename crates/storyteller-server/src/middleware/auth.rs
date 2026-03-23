// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Authentication middleware.
//!
//! Provides auth extraction and validation that deployment crates can
//! optionally apply. The auth strategy (JWT, session tokens, platform-managed)
//! depends on the deployment target.
