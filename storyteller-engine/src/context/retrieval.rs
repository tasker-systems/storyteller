//! Retrieved context assembly — Tier 3 of the Narrator's context.
//!
//! See: `docs/technical/narrator-architecture.md` § Retrieved Context
//!
//! For the prototype, Tier 3 is assembled from CharacterSheet data
//! (backstory, knows/does_not_know, relational context) rather than
//! graph queries. Full GraphRAG (PostgreSQL + AGE traversal) is deferred.
//!
//! Retrieval walks character sheets looking for relevant backstory and
//! relational context for referenced entities. Information boundary
//! enforcement ensures characters don't leak what they don't know.

use chrono::Utc;

use storyteller_core::traits::phase_observer::{PhaseEvent, PhaseEventDetail, PhaseObserver};
use storyteller_core::types::character::{CharacterSheet, SceneData};
use storyteller_core::types::entity::EntityId;
use storyteller_core::types::message::TurnPhaseKind;
use storyteller_core::types::narrator_context::RetrievedContext;

use super::tokens::estimate_tokens;

/// Retrieve context for referenced entities from character sheets.
///
/// For each referenced entity, this pulls relevant backstory, emotional
/// context, and relational information from character sheets. Information
/// boundaries are enforced — only information the characters would know
/// or that has been revealed is included.
///
/// Emits `ContextRetrieved` and `InformationBoundaryApplied` events.
pub fn retrieve_context(
    referenced_entities: &[EntityId],
    characters: &[&CharacterSheet],
    scene: &SceneData,
    observer: &dyn PhaseObserver,
) -> Vec<RetrievedContext> {
    let mut results = Vec::new();

    for &entity_id in referenced_entities {
        // Find the character sheet for this entity
        if let Some(sheet) = characters.iter().find(|c| c.entity_id == entity_id) {
            let mut items = retrieve_for_character(sheet, scene);

            // Information boundary: filter out items that reference
            // things the character explicitly does_not_know
            let available = items.len();
            items.retain(|item| !is_boundary_violation(item, sheet));
            let permitted = items.len();

            if available != permitted {
                observer.emit(PhaseEvent {
                    timestamp: Utc::now(),
                    turn_number: 0,
                    phase: TurnPhaseKind::ContextAssembly,
                    detail: PhaseEventDetail::InformationBoundaryApplied {
                        entity_id,
                        available,
                        permitted,
                    },
                });
            }

            results.extend(items);
        }
    }

    // Emit retrieval summary
    let total_tokens: u32 = results.iter().map(|r| estimate_tokens(&r.content)).sum();
    observer.emit(PhaseEvent {
        timestamp: Utc::now(),
        turn_number: 0,
        phase: TurnPhaseKind::ContextAssembly,
        detail: PhaseEventDetail::ContextRetrieved {
            entity_ids: referenced_entities.to_vec(),
            item_count: results.len(),
            estimated_tokens: total_tokens,
        },
    });

    results
}

/// Retrieve context items for a single character from their sheet.
fn retrieve_for_character(sheet: &CharacterSheet, scene: &SceneData) -> Vec<RetrievedContext> {
    let mut items = Vec::new();

    // Backstory context — brief summary of who this character is
    if !sheet.backstory.is_empty() {
        // Extract the first paragraph or first 200 chars as a summary
        let backstory_summary = sheet
            .backstory
            .find("\n\n")
            .map(|i| &sheet.backstory[..i])
            .unwrap_or_else(|| {
                if sheet.backstory.len() <= 200 {
                    &sheet.backstory
                } else {
                    &sheet.backstory[..200]
                }
            });

        items.push(RetrievedContext {
            subject: format!("{} — backstory", sheet.name),
            content: backstory_summary.to_string(),
            revealed: false,
            emotional_context: None,
            source_entities: vec![sheet.entity_id],
        });
    }

    // Knowledge — what this character knows (available for narration)
    for knowledge in &sheet.knows {
        items.push(RetrievedContext {
            subject: format!("{} knows", sheet.name),
            content: knowledge.clone(),
            revealed: true,
            emotional_context: None,
            source_entities: vec![sheet.entity_id],
        });
    }

    // Performance notes — emotional register guidance for the Narrator
    if !sheet.performance_notes.is_empty() {
        // First paragraph of performance notes
        let perf_summary = sheet
            .performance_notes
            .find("\n\n")
            .map(|i| &sheet.performance_notes[..i])
            .unwrap_or_else(|| {
                if sheet.performance_notes.len() <= 200 {
                    &sheet.performance_notes
                } else {
                    &sheet.performance_notes[..200]
                }
            });

        items.push(RetrievedContext {
            subject: format!("{} — character direction", sheet.name),
            content: perf_summary.to_string(),
            revealed: false,
            emotional_context: Some(
                sheet
                    .emotional_state
                    .mood_vector_notes
                    .first()
                    .cloned()
                    .unwrap_or_default(),
            ),
            source_entities: vec![sheet.entity_id],
        });
    }

    // Self-edge history pattern — adds depth for the Narrator
    if !sheet.self_edge.history_pattern.is_empty() {
        items.push(RetrievedContext {
            subject: format!("{} — pattern", sheet.name),
            content: sheet.self_edge.history_pattern.clone(),
            revealed: false,
            emotional_context: Some(sheet.self_edge.projection_content.clone()),
            source_entities: vec![sheet.entity_id],
        });
    }

    // Scene-specific stakes mentioning this character
    for stake in &scene.stakes {
        let name_lower = sheet.name.to_lowercase();
        if stake.to_lowercase().contains(&name_lower) {
            items.push(RetrievedContext {
                subject: format!("Scene stake — {}", sheet.name),
                content: stake.clone(),
                revealed: false,
                emotional_context: None,
                source_entities: vec![sheet.entity_id],
            });
        }
    }

    items
}

/// Check if a retrieved context item violates information boundaries.
///
/// An item violates boundaries if its content matches something in the
/// character's `does_not_know` list. This is a simple substring check
/// for the prototype; production would use semantic matching.
fn is_boundary_violation(item: &RetrievedContext, sheet: &CharacterSheet) -> bool {
    // Items flagged as revealed are always permitted
    if item.revealed {
        return false;
    }

    // Check if the content matches anything in does_not_know
    let content_lower = item.content.to_lowercase();
    for boundary in &sheet.does_not_know {
        // Extract keywords from the boundary description
        // (the parenthetical awareness notes are stripped)
        let boundary_text = boundary
            .find('(')
            .map(|i| &boundary[..i])
            .unwrap_or(boundary)
            .trim()
            .to_lowercase();

        // Check for significant keyword overlap
        let keywords: Vec<&str> = boundary_text
            .split_whitespace()
            .filter(|w| w.len() > 3) // skip short words
            .collect();

        let matches = keywords
            .iter()
            .filter(|kw| content_lower.contains(**kw))
            .count();

        // If more than half the significant keywords match, it's a boundary violation
        if keywords.len() >= 2 && matches >= keywords.len() / 2 {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use storyteller_core::traits::phase_observer::CollectingObserver;

    #[test]
    fn retrieve_for_known_character() {
        let scene = crate::workshop::the_flute_kept::scene();
        let bramblehoof = crate::workshop::the_flute_kept::bramblehoof();
        let pyotir = crate::workshop::the_flute_kept::pyotir();
        let characters: Vec<&CharacterSheet> = vec![&bramblehoof, &pyotir];

        let observer = CollectingObserver::new();
        let context = retrieve_context(&[bramblehoof.entity_id], &characters, &scene, &observer);

        // Should have backstory, knows items, performance notes, self-edge pattern, stakes
        assert!(
            context.len() >= 3,
            "Expected at least 3 context items for Bramblehoof, got {}",
            context.len()
        );

        // Should have a backstory item
        assert!(context.iter().any(|c| c.subject.contains("backstory")));

        // Should have knowledge items
        assert!(context.iter().any(|c| c.subject.contains("knows")));

        // Observer received events
        let events = observer.take_events();
        assert!(events
            .iter()
            .any(|e| matches!(e.detail, PhaseEventDetail::ContextRetrieved { .. })));
    }

    #[test]
    fn retrieve_for_both_characters() {
        let scene = crate::workshop::the_flute_kept::scene();
        let bramblehoof = crate::workshop::the_flute_kept::bramblehoof();
        let pyotir = crate::workshop::the_flute_kept::pyotir();
        let characters: Vec<&CharacterSheet> = vec![&bramblehoof, &pyotir];

        let observer = storyteller_core::traits::NoopObserver;
        let context = retrieve_context(
            &[bramblehoof.entity_id, pyotir.entity_id],
            &characters,
            &scene,
            &observer,
        );

        // Should have items for both characters
        assert!(context.iter().any(|c| c.subject.contains("Bramblehoof")));
        assert!(context.iter().any(|c| c.subject.contains("Pyotir")));
    }

    #[test]
    fn unknown_entity_returns_no_context() {
        let scene = crate::workshop::the_flute_kept::scene();
        let bramblehoof = crate::workshop::the_flute_kept::bramblehoof();
        let characters: Vec<&CharacterSheet> = vec![&bramblehoof];

        let observer = storyteller_core::traits::NoopObserver;
        let unknown_id = EntityId::new();
        let context = retrieve_context(&[unknown_id], &characters, &scene, &observer);

        // No matching character sheet — empty result
        assert!(context.is_empty());
    }

    #[test]
    fn information_boundary_filters_content() {
        let _scene = crate::workshop::the_flute_kept::scene();
        let bramblehoof = crate::workshop::the_flute_kept::bramblehoof();
        let pyotir = crate::workshop::the_flute_kept::pyotir();

        // Check that Pyotir's does_not_know includes ley line corruption
        assert!(pyotir.does_not_know.iter().any(|s| s.contains("ley line")));

        // Create a context item about ley lines — should be filtered for Pyotir
        let ley_line_item = RetrievedContext {
            subject: "World corruption".to_string(),
            content: "Systematic ley line corruption across the realm".to_string(),
            revealed: false,
            emotional_context: None,
            source_entities: vec![],
        };

        assert!(
            is_boundary_violation(&ley_line_item, &pyotir),
            "Pyotir should not see ley line information"
        );
        assert!(
            !is_boundary_violation(&ley_line_item, &bramblehoof),
            "Bramblehoof CAN see ley line information"
        );
    }

    #[test]
    fn revealed_items_bypass_boundary() {
        let pyotir = crate::workshop::the_flute_kept::pyotir();

        let revealed_item = RetrievedContext {
            subject: "Known fact".to_string(),
            content: "Something about ley line corruption that was revealed".to_string(),
            revealed: true,
            emotional_context: None,
            source_entities: vec![],
        };

        assert!(
            !is_boundary_violation(&revealed_item, &pyotir),
            "Revealed items should bypass information boundaries"
        );
    }
}
