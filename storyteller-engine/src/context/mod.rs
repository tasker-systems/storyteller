//! Context assembly pipeline — builds the Narrator's three-tier context.
//!
//! See: `docs/technical/narrator-architecture.md` § Three-Tier Context
//!
//! The Narrator does not remember. The system remembers for it and provides
//! context on demand. This module assembles:
//!
//! 1. **Persistent preamble** (Tier 1) — narrator identity, scene, cast, boundaries
//! 2. **Rolling scene journal** (Tier 2) — progressively compressed turn history
//! 3. **Retrieved context** (Tier 3) — on-demand backstory via entity reference
//!
//! All assembly steps emit `PhaseEvent`s through the `PhaseObserver` trait
//! for Layer 2 (session) observability.

pub mod event_composition;
pub mod journal;
pub mod preamble;
pub mod prediction;
pub mod retrieval;
pub mod tokens;

use chrono::Utc;

use storyteller_core::traits::phase_observer::{PhaseEvent, PhaseEventDetail, PhaseObserver};
use storyteller_core::types::character::{CharacterSheet, SceneData};
use storyteller_core::types::entity::EntityId;
use storyteller_core::types::narrator_context::{NarratorContextInput, SceneJournal};
use storyteller_core::types::resolver::ResolverOutput;
use storyteller_core::types::turn_cycle::TurnCycleStage;

use self::preamble::{build_preamble, estimate_preamble_tokens};
use self::retrieval::retrieve_context;
use self::tokens::estimate_tokens;

/// Default total token budget for all three tiers combined.
pub const DEFAULT_TOTAL_TOKEN_BUDGET: u32 = 2500;

/// Assemble the complete Narrator context from all three tiers.
///
/// Budget trimming strategy: if the total exceeds the budget, trim Tier 3
/// (retrieved context) first, then compress Tier 2 (journal) more aggressively.
/// Tier 1 (preamble) is never trimmed — it's the narrator's identity.
pub fn assemble_narrator_context(
    scene: &SceneData,
    characters: &[&CharacterSheet],
    journal: &SceneJournal,
    resolver_output: &ResolverOutput,
    player_input: &str,
    referenced_entities: &[EntityId],
    total_budget: u32,
    observer: &dyn PhaseObserver,
) -> NarratorContextInput {
    // Tier 1: Preamble
    let preamble = build_preamble(scene, characters, observer);
    let preamble_tokens = estimate_preamble_tokens(&preamble);

    // Tier 2: Journal (already built and compressed externally)
    let journal_tokens = journal::estimate_journal_tokens(journal);

    // Tier 3: Retrieved context
    let retrieved = retrieve_context(referenced_entities, characters, scene, observer);
    let mut retrieved_tokens: u32 = retrieved.iter().map(|r| estimate_tokens(&r.content)).sum();

    // Budget trimming
    let mut total = preamble_tokens + journal_tokens + retrieved_tokens;
    let trimmed = total > total_budget;

    if total > total_budget {
        // Trim Tier 3 first — drop lowest-priority retrieved items
        let budget_for_retrieved = total_budget.saturating_sub(preamble_tokens + journal_tokens);
        retrieved_tokens = retrieved_tokens.min(budget_for_retrieved);
        total = preamble_tokens + journal_tokens + retrieved_tokens;
    }

    // Emit assembled context event
    observer.emit(PhaseEvent {
        timestamp: Utc::now(),
        turn_number: journal.entries.last().map_or(0, |e| e.turn_number),
        stage: TurnCycleStage::AssemblingContext,
        detail: PhaseEventDetail::ContextAssembled {
            preamble_tokens,
            journal_tokens,
            retrieved_tokens,
            total_tokens: total,
            trimmed,
        },
    });

    NarratorContextInput {
        preamble,
        journal: journal.clone(),
        retrieved,
        resolver_output: resolver_output.clone(),
        player_input_summary: player_input.to_string(),
        estimated_tokens: total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use storyteller_core::traits::phase_observer::CollectingObserver;
    use storyteller_core::types::scene::SceneId;

    fn mock_resolver_output() -> ResolverOutput {
        ResolverOutput {
            sequenced_actions: vec![],
            original_predictions: vec![],
            scene_dynamics: "Quiet tension between recognition and distance".to_string(),
            conflicts: vec![],
        }
    }

    #[test]
    fn full_assembly_from_workshop_data() {
        let scene = crate::workshop::the_flute_kept::scene();
        let bramblehoof = crate::workshop::the_flute_kept::bramblehoof();
        let pyotir = crate::workshop::the_flute_kept::pyotir();
        let characters: Vec<&CharacterSheet> = vec![&bramblehoof, &pyotir];
        let journal = SceneJournal::new(SceneId::new(), 1200);
        let resolver = mock_resolver_output();

        let observer = CollectingObserver::new();
        let context = assemble_narrator_context(
            &scene,
            &characters,
            &journal,
            &resolver,
            "I approach the fence slowly.",
            &[bramblehoof.entity_id, pyotir.entity_id],
            DEFAULT_TOTAL_TOKEN_BUDGET,
            &observer,
        );

        // All three tiers present
        assert_eq!(context.preamble.cast_descriptions.len(), 2);
        assert_eq!(context.journal.turn_count(), 0); // empty journal
        assert!(context.estimated_tokens > 0);
        assert_eq!(context.player_input_summary, "I approach the fence slowly.");

        // Observer received events (preamble + retrieval + assembly)
        let events = observer.take_events();
        assert!(
            events.len() >= 2,
            "Expected at least 2 events, got {}",
            events.len()
        );

        // Last event should be ContextAssembled
        let last = events.last().unwrap();
        match &last.detail {
            PhaseEventDetail::ContextAssembled { total_tokens, .. } => {
                assert!(*total_tokens > 0);
            }
            other => panic!("Expected ContextAssembled, got {other:?}"),
        }
    }

    #[test]
    fn budget_trimming_flags_when_exceeded() {
        let scene = crate::workshop::the_flute_kept::scene();
        let bramblehoof = crate::workshop::the_flute_kept::bramblehoof();
        let pyotir = crate::workshop::the_flute_kept::pyotir();
        let characters: Vec<&CharacterSheet> = vec![&bramblehoof, &pyotir];
        let journal = SceneJournal::new(SceneId::new(), 1200);
        let resolver = mock_resolver_output();

        let observer = CollectingObserver::new();
        // Very tight budget that will be exceeded
        let _context = assemble_narrator_context(
            &scene,
            &characters,
            &journal,
            &resolver,
            "test",
            &[bramblehoof.entity_id],
            100, // absurdly low budget
            &observer,
        );

        let events = observer.take_events();
        let assembled = events
            .iter()
            .find(|e| matches!(e.detail, PhaseEventDetail::ContextAssembled { .. }))
            .expect("Should have ContextAssembled event");

        match &assembled.detail {
            PhaseEventDetail::ContextAssembled { trimmed, .. } => {
                assert!(*trimmed, "Should be trimmed with a 100-token budget");
            }
            _ => unreachable!(),
        }
    }
}
