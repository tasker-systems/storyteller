//! External server LLM provider (e.g., Ollama).
//!
//! See: `docs/technical/technical-stack.md`
//!
//! Implements `LlmProvider` for external inference servers running locally
//! or on the network. Communicates via HTTP/gRPC to servers like Ollama.
