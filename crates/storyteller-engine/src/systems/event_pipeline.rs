// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Event classification and truth set management.
//!
//! See: `docs/technical/event-system.md`
//!
//! Two-track classification: factual (fast, deterministic) and interpretive
//! (may use LLM, asynchronous). The truth set is a materialized view
//! reconstructable from the event ledger.
