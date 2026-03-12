//! In-memory Storykeeper implementation — test and prototype backend.
//!
//! Wraps current engine behavior behind the Storykeeper trait surface.
//! Maintains all state in memory with no database dependency. This is
//! the bridge that lets the engine work identically while we build the
//! PostgreSQL-backed implementation.
//!
//! See: `docs/technical/storykeeper-api-contract.md`

use std::collections::HashMap;
use std::sync::Mutex;

use chrono::Utc;

use storyteller_core::errors::StorytellerResult;
use storyteller_core::traits::storykeeper::{
    BoundaryCheck, CheckpointId, CommandSourced, CommitResult, CompletedTurn, EntityRelevance,
    EntityWeightChange, GateProximity, SceneExitResult, SceneLoadResult, SessionContext,
    StorykeeperCommit, StorykeeperLifecycle, StorykeeperQuery,
};
use storyteller_core::types::entity::{EntityId, EntityRef};
use storyteller_core::types::event::NarrativeEvent;
use storyteller_core::types::message::PlayerInput;
use storyteller_core::types::narrator_context::{
    NarratorContextInput, PersistentPreamble, RetrievedContext, SceneJournal,
};
use storyteller_core::types::resolver::ResolverOutput;
use storyteller_core::types::scene::SceneId;

/// In-memory state for a single session.
#[derive(Debug, Default)]
struct SessionState {
    /// All committed turns in order.
    turns: Vec<CompletedTurn>,
    /// All events from all turns.
    events: Vec<NarrativeEvent>,
    /// Entity weight accumulations.
    entity_weights: HashMap<EntityId, f32>,
    /// Sourced commands awaiting processing.
    sourced_commands: Vec<PlayerInput>,
}

/// In-memory Storykeeper — all state lives in process.
///
/// Thread-safe via `Mutex` for Bevy Send + Sync requirements.
/// Not optimized for performance — this is a correctness bridge.
#[derive(Debug)]
pub struct InMemoryStorykeeper {
    /// Per-session state, keyed by session UUID.
    sessions: Mutex<HashMap<uuid::Uuid, SessionState>>,
}

impl InMemoryStorykeeper {
    /// Create a new empty in-memory Storykeeper.
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
        }
    }

    fn get_or_create_session(
        &self,
        session_id: uuid::Uuid,
    ) -> std::sync::MutexGuard<'_, HashMap<uuid::Uuid, SessionState>> {
        let mut sessions = self.sessions.lock().expect("session lock poisoned");
        sessions.entry(session_id).or_default();
        sessions
    }
}

impl Default for InMemoryStorykeeper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl StorykeeperQuery for InMemoryStorykeeper {
    async fn assemble_narrator_context(
        &self,
        session: &SessionContext,
        journal: &SceneJournal,
        resolver_output: &ResolverOutput,
        player_input: &str,
    ) -> StorytellerResult<NarratorContextInput> {
        // Minimal implementation — assembles context from in-memory state.
        // The real implementation will query PostgreSQL and AGE.
        let preamble = PersistentPreamble {
            narrator_identity: String::new(),
            anti_patterns: Vec::new(),
            setting_description: String::new(),
            cast_descriptions: Vec::new(),
            boundaries: Vec::new(),
            scene_direction: None,
            character_drives: Vec::new(),
            player_context: None,
        };

        let sessions = self.sessions.lock().expect("session lock poisoned");
        let retrieved = if let Some(state) = sessions.get(&session.session_id.0) {
            // Pull recent events as retrieved context
            state
                .events
                .iter()
                .rev()
                .take(5)
                .map(|e| RetrievedContext {
                    subject: format!("Event {}", e.id.0),
                    content: format!("{:?}", e.payload),
                    revealed: true,
                    emotional_context: None,
                    source_entities: Vec::new(),
                })
                .collect()
        } else {
            Vec::new()
        };

        Ok(NarratorContextInput {
            preamble,
            journal: journal.clone(),
            retrieved,
            resolver_output: resolver_output.clone(),
            player_input_summary: player_input.to_string(),
            estimated_tokens: 0,
        })
    }

    async fn query_entity_relevance(
        &self,
        entity: &EntityRef,
        session: &SessionContext,
    ) -> StorytellerResult<EntityRelevance> {
        let entity_id = entity.entity_id().unwrap_or_default();

        let sessions = self.sessions.lock().expect("session lock poisoned");
        let weight = sessions
            .get(&session.session_id.0)
            .and_then(|s| s.entity_weights.get(&entity_id))
            .copied()
            .unwrap_or(0.0);

        Ok(EntityRelevance {
            entity_id,
            relevance: weight.min(1.0),
            reason: "weight-based relevance".to_string(),
            context: Vec::new(),
        })
    }

    async fn query_gate_proximity(
        &self,
        _session: &SessionContext,
    ) -> StorytellerResult<Vec<GateProximity>> {
        // No gate tracking in the in-memory implementation yet.
        Ok(Vec::new())
    }

    fn check_information_boundary(&self, _entity_id: &EntityId, _fact: &str) -> BoundaryCheck {
        // In-memory implementation permits all — no information boundaries enforced.
        BoundaryCheck::Permitted
    }
}

#[async_trait::async_trait]
impl StorykeeperCommit for InMemoryStorykeeper {
    async fn source_command(
        &self,
        input: &PlayerInput,
        session: &SessionContext,
    ) -> StorytellerResult<CommandSourced> {
        let mut sessions = self.get_or_create_session(session.session_id.0);
        let state = sessions
            .get_mut(&session.session_id.0)
            .expect("just created");
        state.sourced_commands.push(input.clone());

        Ok(CommandSourced {
            turn_number: session.turn_number,
            sourced_at: Utc::now(),
        })
    }

    async fn commit_turn(
        &self,
        completed: &CompletedTurn,
        session: &SessionContext,
    ) -> StorytellerResult<CommitResult> {
        let mut sessions = self.get_or_create_session(session.session_id.0);
        let state = sessions
            .get_mut(&session.session_id.0)
            .expect("just created");

        // Record the turn
        state.turns.push(completed.clone());

        // Record events and accumulate entity weights
        let mut weight_changes = Vec::new();
        for event in &completed.events {
            state.events.push(event.clone());
        }

        // Simple weight accumulation: each event participation adds 0.1 weight
        for action in &completed.resolver_output.sequenced_actions {
            let entity_id = action.character_id;
            let previous = *state.entity_weights.get(&entity_id).unwrap_or(&0.0);
            let increment = 0.1 * action.outcomes.len() as f32;
            let new_weight = previous + increment;
            state.entity_weights.insert(entity_id, new_weight);

            weight_changes.push(EntityWeightChange {
                entity_id,
                previous_weight: previous,
                new_weight,
                reason: format!("{} action outcomes", action.outcomes.len()),
            });
        }

        Ok(CommitResult {
            events_committed: completed.events.clone(),
            weight_changes,
            gates_triggered: Vec::new(),
        })
    }
}

#[async_trait::async_trait]
impl StorykeeperLifecycle for InMemoryStorykeeper {
    async fn enter_scene(
        &self,
        scene_id: &SceneId,
        session: &SessionContext,
    ) -> StorytellerResult<SceneLoadResult> {
        // Ensure session state exists
        let _sessions = self.get_or_create_session(session.session_id.0);

        Ok(SceneLoadResult {
            scene_id: *scene_id,
            title: String::new(),
            cast: Vec::new(),
            retrieved_context: Vec::new(),
            journal: SceneJournal::new(*scene_id, 1200),
        })
    }

    async fn exit_scene(&self, _session: &SessionContext) -> StorytellerResult<SceneExitResult> {
        Ok(SceneExitResult {
            checkpoint_id: CheckpointId::new(),
            deferred_count: 0,
        })
    }

    async fn write_checkpoint(&self, _session: &SessionContext) -> StorytellerResult<CheckpointId> {
        // In-memory: no-op checkpoint, just return an ID
        Ok(CheckpointId::new())
    }

    async fn resume_from_checkpoint(
        &self,
        _checkpoint_id: &CheckpointId,
    ) -> StorytellerResult<SceneLoadResult> {
        // In-memory: cannot resume from checkpoint (state is ephemeral)
        Err(storyteller_core::StorytellerError::Config(
            "InMemoryStorykeeper does not support checkpoint resume".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use storyteller_core::types::event::{EventId, EventPriority};
    use storyteller_core::types::event_grammar::EventPayload;
    use storyteller_core::types::prediction::{ActionPrediction, ActionType};
    use storyteller_core::types::resolver::{
        ActionOutcome, ResolvedCharacterAction, SuccessDegree,
    };

    fn test_session() -> SessionContext {
        SessionContext {
            session_id: storyteller_core::traits::storykeeper::SessionId::new(),
            story_id: uuid::Uuid::now_v7(),
            player_entity_id: EntityId::new(),
            current_scene_id: Some(SceneId::new()),
            turn_number: 1,
        }
    }

    fn test_completed_turn() -> CompletedTurn {
        let entity_id = EntityId::new();
        CompletedTurn {
            turn_number: 1,
            player_input: PlayerInput {
                text: "I look around.".to_string(),
                turn_number: 1,
            },
            resolver_output: ResolverOutput {
                sequenced_actions: vec![ResolvedCharacterAction {
                    character_id: entity_id,
                    character_name: "Bramblehoof".to_string(),
                    outcomes: vec![ActionOutcome {
                        action: ActionPrediction {
                            description: "Looks around curiously".to_string(),
                            confidence: 0.9,
                            action_type: ActionType::Examine,
                            target: None,
                        },
                        success: SuccessDegree::FullSuccess,
                        consequences: vec!["Notices the fence is old".to_string()],
                        state_changes: vec![],
                    }],
                }],
                original_predictions: vec![],
                scene_dynamics: "A quiet moment of observation.".to_string(),
                conflicts: vec![],
                intent_statements: None,
            },
            events: vec![NarrativeEvent {
                id: EventId::new(),
                timestamp: Utc::now(),
                priority: EventPriority::Normal,
                payload: EventPayload::Untyped(serde_json::json!({"action": "observe"})),
            }],
        }
    }

    #[tokio::test]
    async fn source_command_records_input() {
        let sk = InMemoryStorykeeper::new();
        let session = test_session();
        let input = PlayerInput {
            text: "Hello".to_string(),
            turn_number: 1,
        };

        let result = sk.source_command(&input, &session).await.unwrap();
        assert_eq!(result.turn_number, 1);

        let sessions = sk.sessions.lock().unwrap();
        let state = sessions.get(&session.session_id.0).unwrap();
        assert_eq!(state.sourced_commands.len(), 1);
    }

    #[tokio::test]
    async fn commit_turn_records_events_and_weights() {
        let sk = InMemoryStorykeeper::new();
        let session = test_session();
        let turn = test_completed_turn();

        let result = sk.commit_turn(&turn, &session).await.unwrap();
        assert_eq!(result.events_committed.len(), 1);
        assert_eq!(result.weight_changes.len(), 1);
        assert!(result.weight_changes[0].new_weight > 0.0);
    }

    #[tokio::test]
    async fn entity_relevance_tracks_weight() {
        let sk = InMemoryStorykeeper::new();
        let session = test_session();
        let turn = test_completed_turn();
        let entity_id = turn.resolver_output.sequenced_actions[0].character_id;

        // Before any commits, relevance should be 0
        let relevance = sk
            .query_entity_relevance(&EntityRef::Resolved(entity_id), &session)
            .await
            .unwrap();
        assert!((relevance.relevance - 0.0).abs() < f32::EPSILON);

        // After commit, relevance should increase
        sk.commit_turn(&turn, &session).await.unwrap();
        let relevance = sk
            .query_entity_relevance(&EntityRef::Resolved(entity_id), &session)
            .await
            .unwrap();
        assert!(relevance.relevance > 0.0);
    }

    #[tokio::test]
    async fn enter_scene_creates_session_state() {
        let sk = InMemoryStorykeeper::new();
        let session = test_session();
        let scene_id = SceneId::new();

        let result = sk.enter_scene(&scene_id, &session).await.unwrap();
        assert_eq!(result.scene_id, scene_id);
    }

    #[tokio::test]
    async fn information_boundary_permits_all() {
        let sk = InMemoryStorykeeper::new();
        let entity_id = EntityId::new();
        assert_eq!(
            sk.check_information_boundary(&entity_id, "secret fact"),
            BoundaryCheck::Permitted,
        );
    }

    #[tokio::test]
    async fn checkpoint_resume_returns_error() {
        let sk = InMemoryStorykeeper::new();
        let checkpoint = CheckpointId::new();
        let result = sk.resume_from_checkpoint(&checkpoint).await;
        assert!(result.is_err());
    }
}
