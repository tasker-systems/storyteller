//! Resolver types — deterministic rules engine replacing the Reconciler.
//!
//! See: `docs/technical/narrator-architecture.md`
//!
//! The Resolver replaces both the Reconciler and World Agent from the original
//! multi-agent architecture. It sequences character predictions, resolves
//! conflicts between overlapping actions, enforces world constraints, and
//! produces a deterministic output consumed by the Narrator's context assembly.
//!
//! Resolution uses hidden RPG-like mechanics: attributes, skills, graduated
//! success outcomes. The player never sees dice rolls — they see narrative
//! consequences.

use super::entity::EntityId;
use super::prediction::{ActionPrediction, CharacterPrediction};

// ---------------------------------------------------------------------------
// Success and outcome modeling
// ---------------------------------------------------------------------------

/// How well an action succeeded — graduated outcomes, not binary pass/fail.
///
/// Every outcome has narrative potential. Even full failure opens a door.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SuccessDegree {
    /// The action achieves what was intended.
    FullSuccess,
    /// The action partially achieves its goal, with complications.
    PartialSuccess,
    /// The action fails, and there are consequences.
    FailureWithConsequence,
    /// The action fails, but it reveals something or opens a new possibility.
    FailureWithOpportunity,
}

/// The outcome of a single resolved action.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActionOutcome {
    /// The original action that was attempted.
    pub action: ActionPrediction,
    /// How well it succeeded.
    pub success: SuccessDegree,
    /// Narrative consequences — what changed in the world or relationships.
    pub consequences: Vec<String>,
    /// State changes to apply after this turn (emotional shifts, relationship changes, etc.).
    pub state_changes: Vec<StateChange>,
}

/// A state change produced by action resolution.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum StateChange {
    /// An emotional shift for a character.
    EmotionalShift {
        character_id: EntityId,
        primary_id: String,
        intensity_change: f32,
    },
    /// A relationship substrate change between two entities.
    RelationalShift {
        source: EntityId,
        target: EntityId,
        dimension: String,
        change: f32,
    },
    /// An environmental or world-state change.
    WorldState { description: String },
}

// ---------------------------------------------------------------------------
// Conflict resolution
// ---------------------------------------------------------------------------

/// How a conflict between character actions was resolved.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConflictResolution {
    /// Human-readable description of what conflicted.
    pub description: String,
    /// How it was resolved.
    pub resolution: String,
    /// Who "won" the conflict, if applicable. None for mutual outcomes.
    pub winner: Option<EntityId>,
}

// ---------------------------------------------------------------------------
// Resolved character action — one character's complete resolved turn
// ---------------------------------------------------------------------------

/// A single character's resolved actions for this turn.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResolvedCharacterAction {
    /// Which character.
    pub character_id: EntityId,
    /// The character's name (for rendering).
    pub character_name: String,
    /// Resolved action outcomes in sequence.
    pub outcomes: Vec<ActionOutcome>,
}

// ---------------------------------------------------------------------------
// Resolver output — the complete turn resolution
// ---------------------------------------------------------------------------

/// The Resolver's complete output for a turn — consumed by the Narrator.
///
/// Replaces `ReconcilerOutput` from the multi-agent architecture. Contains
/// sequenced, resolved character actions plus scene-level dynamics.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResolverOutput {
    /// Resolved character actions in their final sequence (initiative order).
    pub sequenced_actions: Vec<ResolvedCharacterAction>,
    /// The original predictions that were resolved (for Narrator context).
    pub original_predictions: Vec<CharacterPrediction>,
    /// Scene-level dynamics observed during resolution.
    pub scene_dynamics: String,
    /// Any conflicts that were resolved this turn.
    pub conflicts: Vec<ConflictResolution>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::prediction::ActionType;

    #[test]
    fn resolver_output_is_constructible() {
        let output = ResolverOutput {
            sequenced_actions: vec![ResolvedCharacterAction {
                character_id: EntityId::new(),
                character_name: "Bramblehoof".to_string(),
                outcomes: vec![ActionOutcome {
                    action: ActionPrediction {
                        description: "Approaches the fence".to_string(),
                        confidence: 0.85,
                        action_type: ActionType::Move,
                        target: None,
                    },
                    success: SuccessDegree::FullSuccess,
                    consequences: vec!["Pyotir looks up from his work".to_string()],
                    state_changes: vec![],
                }],
            }],
            original_predictions: vec![],
            scene_dynamics: "A quiet arrival — the space between them is physical and temporal"
                .to_string(),
            conflicts: vec![],
        };
        assert_eq!(output.sequenced_actions.len(), 1);
        assert_eq!(
            output.sequenced_actions[0].outcomes[0].success,
            SuccessDegree::FullSuccess
        );
    }

    #[test]
    fn conflict_resolution_is_constructible() {
        let conflict = ConflictResolution {
            description: "Both characters reach for the flute at the same time".to_string(),
            resolution: "Pyotir's hand arrives first — it is his home, his flute".to_string(),
            winner: Some(EntityId::new()),
        };
        assert!(conflict.winner.is_some());
    }

    #[test]
    fn state_changes_cover_all_variants() {
        let changes: Vec<StateChange> = vec![
            StateChange::EmotionalShift {
                character_id: EntityId::new(),
                primary_id: "sadness".to_string(),
                intensity_change: 0.2,
            },
            StateChange::RelationalShift {
                source: EntityId::new(),
                target: EntityId::new(),
                dimension: "trust_reliability".to_string(),
                change: 0.1,
            },
            StateChange::WorldState {
                description: "The evening light deepens".to_string(),
            },
        ];
        assert_eq!(changes.len(), 3);
    }

    #[test]
    fn graduated_success_ordering() {
        // Verify all degrees exist and are distinct
        let degrees = [
            SuccessDegree::FullSuccess,
            SuccessDegree::PartialSuccess,
            SuccessDegree::FailureWithConsequence,
            SuccessDegree::FailureWithOpportunity,
        ];
        for i in 0..degrees.len() {
            for j in (i + 1)..degrees.len() {
                assert_ne!(degrees[i], degrees[j]);
            }
        }
    }
}
