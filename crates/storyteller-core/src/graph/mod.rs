// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Apache AGE graph query operations.
//!
//! See: `docs/technical/infrastructure-architecture.md`, `docs/technical/technical-stack.md`
//!
//! All graph data (relational web, narrative graph, setting topology) lives in
//! PostgreSQL with Apache AGE providing openCypher query support. No separate
//! graph database needed.

pub mod narrative;
pub mod relational_web;
pub mod settings;
