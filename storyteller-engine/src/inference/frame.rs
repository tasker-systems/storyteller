//! Psychological frame computation.
//!
//! See: `docs/foundation/power.md` (psychological frame concept)
//!
//! ML inference layer between relational data and Character Agent performance.
//! Reads substrate + topology + context → produces compressed frame (~200-400
//! tokens) for LLM character agents. Uses ort (ONNX Runtime) for inference.
//!
//! Frames are computed at scene entry and incrementally updated.
//! Compute isolation: frame computation runs on rayon/crossbeam thread pool,
//! separate from the tokio async runtime.
