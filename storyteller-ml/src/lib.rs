//! ML training data generation and feature encoding for the storyteller engine.
//!
//! See: `docs/ticket-specs/storyteller-ml-foundations/_overview.md`
//!
//! This crate owns the boundary between the storyteller type system and the ML
//! pipeline. It provides:
//!
//! - **Feature schema**: canonical encoding of `CharacterSheet`, emotional state,
//!   relational edges, and scene context into fixed-size feature vectors — the
//!   shared contract between Rust inference and Python training.
//! - **Combinatorial matrix**: archetype templates, relational dynamics, and scene
//!   profiles that combine to generate diverse training scenarios.
//! - **Training data generation**: the pipeline that produces labeled examples
//!   (feature vector → structured prediction) via LLM-generated intents.
//! - **Coherence validation**: programmatic checks that ensure generated training
//!   data respects tensor constraints, emotional consistency, and awareness discipline.
//! - **Event templates**: annotated text generation for training event classification
//!   and entity extraction models (Phase C ML pipeline).
//!
//! The ML model itself is trained in Python (PyTorch) and exported to ONNX.
//! Inference runs in `storyteller-engine` via `ort`. This crate produces the
//! training data and defines the encoding contract that both sides share.

pub mod event_templates;
pub mod feature_schema;
pub mod matrix;
