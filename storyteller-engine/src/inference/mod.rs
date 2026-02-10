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
pub mod external;
pub mod frame;

#[cfg(feature = "local-llm")]
pub mod local;
