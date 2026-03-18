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
/// Five variants. `AwaitingInput` is the rest state — no pipeline systems
/// run. The four active stages model the narrator-architecture.md pipeline
/// with commitment of the *previous* turn's provisional data triggered by
/// reception of new player input:
///
/// `AwaitingInput → CommittingPrevious → Enriching →
///  AssemblingContext → Rendering → AwaitingInput`
///
/// Commitment comes first because the player's response is what transforms
/// provisional ML/LLM outputs (Hypothesized/Rendered) into Committed data.
/// Only after the previous turn's data is committed does the new turn's
/// enrichment sub-pipeline begin (see `EnrichmentPhase`).
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
    /// All enrichment work: event classification, ML prediction,
    /// game system arbitration, intent synthesis. Internal sub-pipeline
    /// managed by EnrichmentPhase.
    Enriching,
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
            Self::CommittingPrevious => Self::Enriching,
            Self::Enriching => Self::AssemblingContext,
            Self::AssemblingContext => Self::Rendering,
            Self::Rendering => Self::AwaitingInput,
        }
    }
}

/// Sub-pipeline phases within the Enriching stage.
///
/// Managed by the enrichment system internally — not visible to the
/// top-level Bevy schedule. Extensible: new phases slot in without
/// changing TurnCycleStage.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum EnrichmentPhase {
    #[default]
    EventClassification,
    BehaviorPrediction,
    GameSystemArbitration,
    IntentSynthesis,
    Complete,
}

impl EnrichmentPhase {
    /// Advance to the next phase. `Complete` stays at `Complete`.
    pub fn next(self) -> Self {
        match self {
            Self::EventClassification => Self::BehaviorPrediction,
            Self::BehaviorPrediction => Self::GameSystemArbitration,
            Self::GameSystemArbitration => Self::IntentSynthesis,
            Self::IntentSynthesis => Self::Complete,
            Self::Complete => Self::Complete,
        }
    }

    /// Whether the enrichment sub-pipeline is complete.
    pub fn is_complete(self) -> bool {
        matches!(self, Self::Complete)
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
            TurnCycleStage::Enriching,
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
    fn committing_previous_precedes_enriching() {
        assert_eq!(
            TurnCycleStage::CommittingPrevious.next(),
            TurnCycleStage::Enriching
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
    fn enriching_precedes_assembling_context() {
        assert_eq!(
            TurnCycleStage::Enriching.next(),
            TurnCycleStage::AssemblingContext
        );
    }

    #[test]
    fn enrichment_phase_cycles_to_complete() {
        let mut phase = EnrichmentPhase::default();
        let expected = [
            EnrichmentPhase::BehaviorPrediction,
            EnrichmentPhase::GameSystemArbitration,
            EnrichmentPhase::IntentSynthesis,
            EnrichmentPhase::Complete,
        ];
        for &exp in &expected {
            phase = phase.next();
            assert_eq!(phase, exp);
        }
        assert!(phase.is_complete());
    }

    #[test]
    fn enrichment_complete_stays_complete() {
        assert_eq!(EnrichmentPhase::Complete.next(), EnrichmentPhase::Complete);
    }

    #[test]
    fn serde_roundtrip_committing_previous() {
        let stage = TurnCycleStage::CommittingPrevious;
        let json = serde_json::to_string(&stage).expect("serialize");
        let deserialized: TurnCycleStage = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(stage, deserialized);
    }

    #[test]
    fn serde_roundtrip_enriching() {
        let stage = TurnCycleStage::Enriching;
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
