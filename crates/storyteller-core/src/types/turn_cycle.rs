//! Turn cycle pipeline types — the state machine driving the narrator-centric pipeline.
//!
//! See: `docs/technical/turn-cycle-architecture.md`
//!
//! `TurnCycleStage` is held as a Bevy Resource. Systems use `run_if` conditions
//! to gate on the current stage. Each stage system advances to the next when
//! its work completes.

/// Which stage of the turn pipeline is currently active.
///
/// Held as a Bevy Resource. Systems use `run_if` conditions to gate
/// on the current stage. Each stage system advances to the next when
/// its work completes.
///
/// Eight variants. `AwaitingInput` is the rest state — no pipeline systems
/// run. The seven active stages model the narrator-architecture.md pipeline
/// with commitment of the *previous* turn's provisional data triggered by
/// reception of new player input:
///
/// `AwaitingInput → CommittingPrevious → Classifying → Predicting →
///  Resolving → AssemblingContext → Rendering → AwaitingInput`
///
/// Commitment comes first because the player's response is what transforms
/// provisional ML/LLM outputs (Hypothesized/Rendered) into Committed data.
/// Only after the previous turn's data is committed does the new turn's
/// classification begin.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum TurnCycleStage {
    /// Waiting for player input. No pipeline systems run.
    #[default]
    AwaitingInput,
    /// Player input received. Commit the previous turn's provisional
    /// data (predictions, rendering) to the event ledger and truth set.
    /// On the first turn of a scene, this is a no-op.
    CommittingPrevious,
    /// Event classifier processing raw input → ClassificationOutput.
    Classifying,
    /// ML character prediction running in parallel across cast.
    Predicting,
    /// Rules engine resolving predictions → ResolverOutput.
    Resolving,
    /// Storykeeper assembling three-tier Narrator context.
    AssemblingContext,
    /// Narrator LLM call in progress (async bridge).
    Rendering,
}

impl TurnCycleStage {
    /// Advance to the next stage in the pipeline.
    ///
    /// `Rendering` wraps around to `AwaitingInput`, completing the cycle.
    pub fn next(self) -> Self {
        match self {
            Self::AwaitingInput => Self::CommittingPrevious,
            Self::CommittingPrevious => Self::Classifying,
            Self::Classifying => Self::Predicting,
            Self::Predicting => Self::Resolving,
            Self::Resolving => Self::AssemblingContext,
            Self::AssemblingContext => Self::Rendering,
            Self::Rendering => Self::AwaitingInput,
        }
    }
}

/// Simplified entity category for participant role assignment.
///
/// Lives in core (not in storyteller-ml) so that `assign_participant_roles`
/// can operate without an ML dependency. Maps from `NerCategory` at the
/// call site in the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EntityCategory {
    /// A character or person.
    Character,
    /// An object or item.
    Object,
    /// A place or location.
    Location,
    /// Any other entity category.
    Other,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_awaiting_input() {
        assert_eq!(TurnCycleStage::default(), TurnCycleStage::AwaitingInput);
    }

    #[test]
    fn next_cycles_through_all_stages() {
        let mut stage = TurnCycleStage::AwaitingInput;
        let expected = [
            TurnCycleStage::CommittingPrevious,
            TurnCycleStage::Classifying,
            TurnCycleStage::Predicting,
            TurnCycleStage::Resolving,
            TurnCycleStage::AssemblingContext,
            TurnCycleStage::Rendering,
            TurnCycleStage::AwaitingInput,
        ];

        for &exp in &expected {
            stage = stage.next();
            assert_eq!(stage, exp);
        }
    }

    #[test]
    fn rendering_wraps_to_awaiting_input() {
        assert_eq!(
            TurnCycleStage::Rendering.next(),
            TurnCycleStage::AwaitingInput
        );
    }

    #[test]
    fn committing_previous_precedes_classifying() {
        assert_eq!(
            TurnCycleStage::CommittingPrevious.next(),
            TurnCycleStage::Classifying
        );
    }

    #[test]
    fn awaiting_input_starts_with_commit() {
        assert_eq!(
            TurnCycleStage::AwaitingInput.next(),
            TurnCycleStage::CommittingPrevious
        );
    }

    #[test]
    fn serde_roundtrip() {
        let stage = TurnCycleStage::Predicting;
        let json = serde_json::to_string(&stage).expect("serialize");
        let deserialized: TurnCycleStage = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(stage, deserialized);
    }

    #[test]
    fn serde_roundtrip_committing_previous() {
        let stage = TurnCycleStage::CommittingPrevious;
        let json = serde_json::to_string(&stage).expect("serialize");
        let deserialized: TurnCycleStage = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(stage, deserialized);
    }

    #[test]
    fn entity_category_all_variants_distinct() {
        let categories = [
            EntityCategory::Character,
            EntityCategory::Object,
            EntityCategory::Location,
            EntityCategory::Other,
        ];
        for i in 0..categories.len() {
            for j in (i + 1)..categories.len() {
                assert_ne!(categories[i], categories[j]);
            }
        }
    }
}
