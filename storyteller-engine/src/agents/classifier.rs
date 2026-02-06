//! Classifier agents — NL input to typed events.
//!
//! See: `docs/technical/event-system.md` (Stage 1 and Stage 3 classification)
//!
//! Pre-processing layer that classifies player input before it reaches
//! the main agent pipeline. Stage 1 is fast factual classification;
//! Stage 3 is interpretive classification that may use LLM inference.
