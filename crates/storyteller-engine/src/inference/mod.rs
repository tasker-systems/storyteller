// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! ML inference integration — frame computation, event classification, and LLM providers.
//!
//! See: `docs/technical/technical-stack.md`, `docs/foundation/power.md`
//!
//! The inference module bridges computational-predictive and agentic-generative
//! responsibilities. Frame computation (ort/ONNX) produces compressed
//! psychological frames; event classification (ort/ONNX + tokenizers) extracts
//! typed events and entities from natural language; LLM providers handle
//! natural language generation.

pub mod cloud;
pub mod event_classifier;
pub mod event_decomposition;
pub mod external;
pub mod frame;
pub mod intent_synthesis;
pub mod intention_generation;
pub mod structured;

#[cfg(feature = "local-llm")]
pub mod local;
