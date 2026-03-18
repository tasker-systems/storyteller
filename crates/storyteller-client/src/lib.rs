//! Typed gRPC client for the storyteller engine server.
//!
//! Provides [`StorytellerClient`] which wraps the generated tonic client stubs
//! with typed methods and maps proto responses to `storyteller-core` types.

pub mod proto {
    tonic::include_proto!("storyteller.v1");
}

mod client;
pub use client::{ClientConfig, ClientError, StorytellerClient};

// Re-export commonly used proto types for consumer convenience
pub use proto::{
    engine_event, CastMember, ComposeSceneRequest, DynamicPairing, EngineEvent,
    GetSceneStateRequest, PlayerCharacter, ResumeSessionRequest, SubmitInputRequest,
};
