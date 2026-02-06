//! ML inference integration — frame computation and LLM providers.
//!
//! See: `docs/technical/technical-stack.md`, `docs/foundation/power.md`
//!
//! The inference module bridges computational-predictive and agentic-generative
//! responsibilities. Frame computation (ort/ONNX) produces compressed
//! psychological frames; LLM providers handle natural language generation.

pub mod cloud;
pub mod external;
pub mod frame;

#[cfg(feature = "local-llm")]
pub mod local;
