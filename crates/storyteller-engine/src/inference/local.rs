// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Local LLM provider implementation (candle).
//!
//! See: `docs/technical/technical-stack.md`
//!
//! Implements `LlmProvider` for local model inference via candle.
//! Feature-gated behind `local-llm` (disabled by default).
//!
//! Future direction: `burn` as all-Rust replacement for the Python→ONNX path.
