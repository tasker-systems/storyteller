// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Storykeeper persistence traits — the domain facade for narrative state.
//!
//! See: `docs/technical/storykeeper-api-contract.md`
//!
//! The Storykeeper is the guardian of complete narrative state. These traits
//! define three interfaces:
//! - [`StorykeeperQuery`] — reads: "what matters right now?"
//! - [`StorykeeperCommit`] — writes: "what happened this turn?"
//! - [`StorykeeperLifecycle`] — transitions: scene entry/exit, checkpoints
//!
//! Two implementations are planned:
//! - `InMemoryStorykeeper` — wraps current engine behavior for tests/prototype
//! - `PostgresStorykeeper` — sqlx queries against the real schema (future)
//!
//! The engine interacts exclusively through these traits via a Bevy Resource
//! wrapping `Arc<dyn Storykeeper>`.

use crate::errors::StorytellerResult;
use crate::types::character::CharacterSheet;
use crate::types::entity::{EntityId, EntityRef};
use crate::types::event::NarrativeEvent;
use crate::types::message::PlayerInput;
use crate::types::narrator_context::{NarratorContextInput, RetrievedContext, SceneJournal};
use crate::types::resolver::ResolverOutput;
use crate::types::scene::SceneId;

use uuid::Uuid;

// ---------------------------------------------------------------------------
// Supporting domain types
// ---------------------------------------------------------------------------

/// Identifies a player session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct SessionId(pub Uuid);

impl SessionId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

/// Identifies a checkpoint for crash recovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CheckpointId(pub Uuid);

impl CheckpointId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for CheckpointId {
    fn default() -> Self {
        Self::new()
    }
}

/// Session context — identifies the current player session and scene position.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionContext {
    pub session_id: SessionId,
    pub story_id: Uuid,
    pub player_entity_id: EntityId,
    pub current_scene_id: Option<SceneId>,
    pub turn_number: u32,
}

/// Scene data loaded at scene entry — everything needed to run a scene.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SceneLoadResult {
    /// The scene's identity and metadata.
    pub scene_id: SceneId,
    /// Scene title.
    pub title: String,
    /// Cast of characters present in this scene.
    pub cast: Vec<CharacterSheet>,
    /// Retrieved context items for initial scene state.
    pub retrieved_context: Vec<RetrievedContext>,
    /// Scene journal (empty for new scenes, populated for resumed scenes).
    pub journal: SceneJournal,
}

/// Result of committing a completed turn.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommitResult {
    /// Events written to the ledger this turn.
    pub events_committed: Vec<NarrativeEvent>,
    /// Entity weight changes applied.
    pub weight_changes: Vec<EntityWeightChange>,
    /// Whether any information gates were triggered.
    pub gates_triggered: Vec<String>,
}

/// A change in an entity's relational weight from turn commitment.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EntityWeightChange {
    pub entity_id: EntityId,
    pub previous_weight: f32,
    pub new_weight: f32,
    pub reason: String,
}

/// Result of a scene exit — what was flushed and what was deferred.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SceneExitResult {
    /// Checkpoint ID for the state snapshot written at scene exit.
    pub checkpoint_id: CheckpointId,
    /// Number of deferred cascade items queued for background processing.
    pub deferred_count: usize,
}

/// A completed turn ready for commitment.
///
/// Bundles the player input, classification, predictions, and resolver output
/// that together constitute what happened this turn.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompletedTurn {
    pub turn_number: u32,
    pub player_input: PlayerInput,
    pub resolver_output: ResolverOutput,
    pub events: Vec<NarrativeEvent>,
}

/// How relevant an entity is to the current scene context.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EntityRelevance {
    pub entity_id: EntityId,
    /// Relevance score in [0.0, 1.0].
    pub relevance: f32,
    /// Why this entity is relevant (for context assembly).
    pub reason: String,
    /// Retrieved context items about this entity.
    pub context: Vec<RetrievedContext>,
}

/// A gate condition and how close it is to triggering.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GateProximity {
    /// Human-readable gate name.
    pub gate_name: String,
    /// The scene this gate controls access to.
    pub target_scene_id: SceneId,
    /// Fraction of conditions satisfied, in [0.0, 1.0].
    pub satisfaction: f32,
    /// Which conditions are still unmet.
    pub unmet_conditions: Vec<String>,
}

/// Whether an entity is permitted to know a specific fact.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BoundaryCheck {
    /// The entity may know this.
    Permitted,
    /// The entity must not know this — it is guarded by information boundaries.
    Withheld,
    /// The entity has partial knowledge — some aspects are revealed.
    PartiallyRevealed { visible_aspects: Vec<String> },
}

/// Confirmation that player input was durably stored before processing.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommandSourced {
    pub turn_number: u32,
    pub sourced_at: chrono::DateTime<chrono::Utc>,
}

// ---------------------------------------------------------------------------
// Trait: StorykeeperQuery — reads
// ---------------------------------------------------------------------------

/// The read interface — "what matters right now?"
///
/// Used by the Narrator (context assembly), prediction pipeline, trigger
/// evaluation, and domain data queries. All queries respect information
/// boundaries natively.
#[async_trait::async_trait]
pub trait StorykeeperQuery: Send + Sync + std::fmt::Debug {
    /// Assemble the full three-tier context for the Narrator.
    async fn assemble_narrator_context(
        &self,
        session: &SessionContext,
        journal: &SceneJournal,
        resolver_output: &ResolverOutput,
        player_input: &str,
    ) -> StorytellerResult<NarratorContextInput>;

    /// What is relevant about this entity right now?
    async fn query_entity_relevance(
        &self,
        entity: &EntityRef,
        session: &SessionContext,
    ) -> StorytellerResult<EntityRelevance>;

    /// Which gates are close to triggering?
    async fn query_gate_proximity(
        &self,
        session: &SessionContext,
    ) -> StorytellerResult<Vec<GateProximity>>;

    /// Is this entity permitted to know this fact?
    fn check_information_boundary(&self, entity_id: &EntityId, fact: &str) -> BoundaryCheck;
}

// ---------------------------------------------------------------------------
// Trait: StorykeeperCommit — writes
// ---------------------------------------------------------------------------

/// The write interface — "what happened this turn?"
///
/// Orchestrates the domain transaction: events → ledger, entity weights,
/// relational cascade, information state, truth set — all updated as one
/// coherent operation.
#[async_trait::async_trait]
pub trait StorykeeperCommit: Send + Sync + std::fmt::Debug {
    /// Durably store player input before processing begins (command sourcing).
    async fn source_command(
        &self,
        input: &PlayerInput,
        session: &SessionContext,
    ) -> StorytellerResult<CommandSourced>;

    /// Process a completed turn into durable state changes.
    async fn commit_turn(
        &self,
        completed: &CompletedTurn,
        session: &SessionContext,
    ) -> StorytellerResult<CommitResult>;
}

// ---------------------------------------------------------------------------
// Trait: StorykeeperLifecycle — scene transitions and sessions
// ---------------------------------------------------------------------------

/// Scene transition and session lifecycle management.
///
/// Manages the cold-to-hot state transitions at scene boundaries,
/// checkpointing for crash recovery, and session lifecycle.
#[async_trait::async_trait]
pub trait StorykeeperLifecycle: Send + Sync + std::fmt::Debug {
    /// Load everything needed for a scene.
    async fn enter_scene(
        &self,
        scene_id: &SceneId,
        session: &SessionContext,
    ) -> StorytellerResult<SceneLoadResult>;

    /// Flush accumulated state and process deferred effects.
    async fn exit_scene(&self, session: &SessionContext) -> StorytellerResult<SceneExitResult>;

    /// Snapshot state for crash recovery.
    async fn write_checkpoint(&self, session: &SessionContext) -> StorytellerResult<CheckpointId>;

    /// Restore from checkpoint + replay ledger delta.
    async fn resume_from_checkpoint(
        &self,
        checkpoint_id: &CheckpointId,
    ) -> StorytellerResult<SceneLoadResult>;
}

// ---------------------------------------------------------------------------
// Combined trait — convenience for Bevy Resource wrapping
// ---------------------------------------------------------------------------

/// Combined Storykeeper trait for use as a Bevy Resource.
///
/// The engine wraps `Arc<dyn Storykeeper>` as a Resource, giving all systems
/// access to query, commit, and lifecycle operations through a single handle.
pub trait Storykeeper: StorykeeperQuery + StorykeeperCommit + StorykeeperLifecycle {}

/// Blanket implementation: anything implementing all three traits is a Storykeeper.
impl<T: StorykeeperQuery + StorykeeperCommit + StorykeeperLifecycle> Storykeeper for T {}
