// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Agent implementations.
//!
//! See: `docs/technical/narrator-architecture.md`
//!
//! In the narrator-centric architecture, the Narrator is the ONLY LLM agent.
//! Character behavior is predicted by ML models, and world constraints are
//! enforced by a deterministic rules engine (Resolver). Agents are Bevy
//! systems — lightweight functions, not microservices.

pub mod classifier;
pub mod narrator;
pub mod world;

pub use narrator::NarratorAgent;
