//! Preamble construction — Tier 1 of the Narrator's context.
//!
//! See: `docs/technical/narrator-architecture.md` § Three-Tier Context
//!
//! The persistent preamble is constructed at scene entry and updated only
//! when the cast changes or scene constraints shift. It tells the Narrator
//! who it is, what the scene is, who is present, and what boundaries apply.

use chrono::Utc;

use storyteller_core::traits::phase_observer::{PhaseEvent, PhaseEventDetail, PhaseObserver};
use storyteller_core::types::character::{CharacterSheet, SceneData};
use storyteller_core::types::narrator_context::{CastDescription, PersistentPreamble};
use storyteller_core::types::turn_cycle::TurnCycleStage;

use super::tokens::estimate_tokens;

/// Build the Tier 1 persistent preamble from scene data and character sheets.
///
/// Extracts narrator voice, anti-patterns, setting, cast descriptions with
/// voice notes, and hard narrative boundaries. Emits a `PreambleBuilt`
/// event through the observer.
pub fn build_preamble(
    scene: &SceneData,
    characters: &[&CharacterSheet],
    observer: &dyn PhaseObserver,
) -> PersistentPreamble {
    // Narrator voice — hardcoded for the prototype (matches the existing
    // narrator system prompt style). In production this comes from a
    // voice configuration on the scene or story.
    let narrator_identity = String::from(
        "Literary fiction, present tense, close third person. \
         Compression: every sentence earns its place. \
         Sensory specificity: ground the reader in physical detail. \
         Subtext through physical detail: a gesture reveals more than dialogue. \
         Restraint: what you leave out matters as much as what you include.",
    );

    // Anti-patterns — inverted from the scene's evaluation criteria and
    // the existing narrator system prompt.
    let anti_patterns = vec![
        "Exclamation marks".to_string(),
        "Adverbs where a better verb would serve".to_string(),
        "Fantasy exposition or lore dumps".to_string(),
        "Telling the reader what characters think, feel, or realize".to_string(),
        "Re-rendering or summarizing what the player has already read".to_string(),
        "Inventing goodbyes, departures, or scene resolutions not in the facts".to_string(),
        "Breaking the fourth wall".to_string(),
    ];

    // Setting from scene data.
    let mut setting_description = scene.setting.description.clone();
    if !scene.setting.aesthetic_detail.is_empty() {
        setting_description.push_str("\n\n");
        setting_description.push_str(&scene.setting.aesthetic_detail);
    }

    // Cast descriptions — match character sheets to cast entries by name,
    // pulling voice notes from the sheet.
    let cast_descriptions: Vec<CastDescription> = scene
        .cast
        .iter()
        .map(|cast_entry| {
            let voice_note = characters
                .iter()
                .find(|c| c.name == cast_entry.name)
                .map(|c| c.voice.clone())
                .unwrap_or_default();

            CastDescription {
                entity_id: cast_entry.entity_id,
                name: cast_entry.name.clone(),
                role: cast_entry.role.clone(),
                voice_note,
            }
        })
        .collect();

    // Hard boundaries from scene constraints.
    let boundaries = scene.constraints.hard.clone();

    let preamble = PersistentPreamble {
        narrator_identity,
        anti_patterns,
        setting_description,
        cast_descriptions,
        boundaries,
    };

    // Emit observability event.
    let estimated_tokens = estimate_preamble_tokens(&preamble);
    observer.emit(PhaseEvent {
        timestamp: Utc::now(),
        turn_number: 0,
        stage: TurnCycleStage::AssemblingContext,
        detail: PhaseEventDetail::PreambleBuilt {
            cast_count: preamble.cast_descriptions.len(),
            boundary_count: preamble.boundaries.len(),
            estimated_tokens,
        },
    });

    preamble
}

/// Estimate token count for a preamble.
pub fn estimate_preamble_tokens(preamble: &PersistentPreamble) -> u32 {
    let mut total = estimate_tokens(&preamble.narrator_identity);
    for ap in &preamble.anti_patterns {
        total += estimate_tokens(ap);
    }
    total += estimate_tokens(&preamble.setting_description);
    for cast in &preamble.cast_descriptions {
        total += estimate_tokens(&cast.name);
        total += estimate_tokens(&cast.role);
        total += estimate_tokens(&cast.voice_note);
    }
    for boundary in &preamble.boundaries {
        total += estimate_tokens(boundary);
    }
    total
}

/// Render a preamble to a string suitable for the Narrator's system prompt.
pub fn render_preamble(preamble: &PersistentPreamble) -> String {
    let mut output = String::new();

    output.push_str("## Your Voice\n");
    output.push_str(&preamble.narrator_identity);
    output.push_str("\n\n");

    output.push_str("## Never Do\n");
    for ap in &preamble.anti_patterns {
        output.push_str("- ");
        output.push_str(ap);
        output.push('\n');
    }
    output.push('\n');

    output.push_str("## The Scene\n");
    output.push_str(&preamble.setting_description);
    output.push_str("\n\n");

    output.push_str("## Cast\n");
    for cast in &preamble.cast_descriptions {
        output.push_str(&format!("### {} — {}\n", cast.name, cast.role));
        if !cast.voice_note.is_empty() {
            output.push_str(&format!("Voice: {}\n", cast.voice_note));
        }
        output.push('\n');
    }

    if !preamble.boundaries.is_empty() {
        output.push_str("## Boundaries\n");
        for boundary in &preamble.boundaries {
            output.push_str("- ");
            output.push_str(boundary);
            output.push('\n');
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use storyteller_core::traits::phase_observer::CollectingObserver;

    #[test]
    fn preamble_from_workshop_scene() {
        let scene = crate::workshop::the_flute_kept::scene();
        let bramblehoof = crate::workshop::the_flute_kept::bramblehoof();
        let pyotir = crate::workshop::the_flute_kept::pyotir();
        let characters: Vec<&CharacterSheet> = vec![&bramblehoof, &pyotir];

        let observer = CollectingObserver::new();
        let preamble = build_preamble(&scene, &characters, &observer);

        // Cast
        assert_eq!(preamble.cast_descriptions.len(), 2);
        assert_eq!(preamble.cast_descriptions[0].name, "Bramblehoof");
        assert_eq!(preamble.cast_descriptions[1].name, "Pyotir");

        // Voice notes pulled from character sheets
        assert!(preamble.cast_descriptions[0]
            .voice_note
            .contains("metaphor"));
        assert!(preamble.cast_descriptions[1]
            .voice_note
            .contains("Measured"));

        // Boundaries from hard constraints
        assert_eq!(preamble.boundaries.len(), 3);
        assert!(preamble.boundaries[0].contains("Pyotir cannot leave"));

        // Setting includes aesthetic detail
        assert!(preamble.setting_description.contains("herb plants"));

        // Observer received an event
        let events = observer.take_events();
        assert_eq!(events.len(), 1);
        match &events[0].detail {
            PhaseEventDetail::PreambleBuilt {
                cast_count,
                boundary_count,
                estimated_tokens,
            } => {
                assert_eq!(*cast_count, 2);
                assert_eq!(*boundary_count, 3);
                assert!(*estimated_tokens > 0);
            }
            other => panic!("Expected PreambleBuilt, got {other:?}"),
        }
    }

    #[test]
    fn preamble_token_estimate_is_reasonable() {
        let scene = crate::workshop::the_flute_kept::scene();
        let bramblehoof = crate::workshop::the_flute_kept::bramblehoof();
        let pyotir = crate::workshop::the_flute_kept::pyotir();
        let characters: Vec<&CharacterSheet> = vec![&bramblehoof, &pyotir];

        let observer = storyteller_core::traits::NoopObserver;
        let preamble = build_preamble(&scene, &characters, &observer);
        let tokens = estimate_preamble_tokens(&preamble);

        // Architecture doc says ~600-800 tokens for Tier 1
        // With the workshop scene's detailed setting and two characters,
        // we expect something in that range (give generous bounds for heuristic)
        assert!(
            (200..=1200).contains(&tokens),
            "Expected 200-1200 tokens, got {tokens}"
        );
    }

    #[test]
    fn rendered_preamble_has_all_sections() {
        let scene = crate::workshop::the_flute_kept::scene();
        let bramblehoof = crate::workshop::the_flute_kept::bramblehoof();
        let pyotir = crate::workshop::the_flute_kept::pyotir();
        let characters: Vec<&CharacterSheet> = vec![&bramblehoof, &pyotir];

        let observer = storyteller_core::traits::NoopObserver;
        let preamble = build_preamble(&scene, &characters, &observer);
        let rendered = render_preamble(&preamble);

        assert!(rendered.contains("## Your Voice"));
        assert!(rendered.contains("## Never Do"));
        assert!(rendered.contains("## The Scene"));
        assert!(rendered.contains("## Cast"));
        assert!(rendered.contains("## Boundaries"));
        assert!(rendered.contains("Bramblehoof"));
        assert!(rendered.contains("Pyotir"));
        assert!(rendered.contains("metaphor"));
    }
}
