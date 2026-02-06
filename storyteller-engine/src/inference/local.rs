//! Local LLM provider implementation (candle).
//!
//! See: `docs/technical/technical-stack.md`
//!
//! Implements `LlmProvider` for local model inference via candle.
//! Feature-gated behind `local-llm` (disabled by default).
//!
//! Future direction: `burn` as all-Rust replacement for the Python→ONNX path.
