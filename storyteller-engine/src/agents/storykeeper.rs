//! Storykeeper agent — guardian of the complete narrative state.
//!
//! See: `docs/foundation/system_architecture.md`
//!
//! Holds the complete narrative graph, information ledger, character tensors,
//! and relationship web. Filters what downstream agents may know.
//! Guards mystery and revelation.
//!
//! For the prototype, this is a deterministic Rust function — not an LLM agent.
//! Information boundaries are known at compile time for hardcoded scenes.

use storyteller_core::types::character::{CharacterSheet, SceneData};
use storyteller_core::types::message::{PlayerInput, StorykeeperDirective};

/// Produce per-character directives from player input and scene state.
///
/// For each character, creates a `StorykeeperDirective` with:
/// - `visible_context`: the most recent narration (what both characters perceived)
/// - `filtered_input`: player input framed for this character's perspective
/// - `guidance`: scene-phase hint based on turn number
pub fn produce_directives(
    player_input: &PlayerInput,
    scene: &SceneData,
    characters: &[&CharacterSheet],
    recent_narration: Option<&str>,
) -> Vec<StorykeeperDirective> {
    let guidance = scene_phase_guidance(player_input.turn_number, scene);

    characters
        .iter()
        .map(|sheet| {
            let visible_context = recent_narration
                .unwrap_or("The scene is just beginning.")
                .to_string();

            let filtered_input = format!(
                "In the scene: {}. You are {}, {}.",
                player_input.text,
                sheet.name,
                character_situation_note(sheet, scene),
            );

            StorykeeperDirective {
                character_id: sheet.entity_id,
                visible_context,
                filtered_input,
                guidance: guidance.clone(),
            }
        })
        .collect()
}

/// A brief situation note for each character based on their role in the scene.
fn character_situation_note(sheet: &CharacterSheet, scene: &SceneData) -> String {
    scene
        .cast
        .iter()
        .find(|c| c.entity_id == sheet.entity_id)
        .map(|c| c.role.clone())
        .unwrap_or_else(|| "present in this scene".to_string())
}

/// Scene-phase guidance based on turn number and scene arc.
fn scene_phase_guidance(turn: u32, scene: &SceneData) -> String {
    // Map turn number to rough phase in the emotional arc.
    // The arc has ~7 phases; we distribute across turns.
    let phase_index = match turn {
        0..=2 => 0,
        3..=4 => 1,
        5..=6 => 2,
        7..=8 => 3,
        9..=10 => 4,
        11..=12 => 5,
        _ => 6,
    };

    scene
        .emotional_arc
        .get(phase_index)
        .cloned()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn produces_one_directive_per_character() {
        let scene = crate::workshop::the_flute_kept::scene();
        let bramble = crate::workshop::the_flute_kept::bramblehoof();
        let pyotir = crate::workshop::the_flute_kept::pyotir();

        let input = PlayerInput {
            text: "Bramblehoof approaches the fence.".to_string(),
            turn_number: 1,
        };

        let directives = produce_directives(&input, &scene, &[&bramble, &pyotir], None);
        assert_eq!(directives.len(), 2);
        assert_eq!(directives[0].character_id, bramble.entity_id);
        assert_eq!(directives[1].character_id, pyotir.entity_id);
    }

    #[test]
    fn filtered_input_includes_character_name() {
        let scene = crate::workshop::the_flute_kept::scene();
        let bramble = crate::workshop::the_flute_kept::bramblehoof();

        let input = PlayerInput {
            text: "The satyr walks up the path.".to_string(),
            turn_number: 1,
        };

        let directives = produce_directives(&input, &scene, &[&bramble], None);
        assert!(directives[0].filtered_input.contains("Bramblehoof"));
    }

    #[test]
    fn visible_context_uses_narration_when_available() {
        let scene = crate::workshop::the_flute_kept::scene();
        let bramble = crate::workshop::the_flute_kept::bramblehoof();

        let input = PlayerInput {
            text: "test".to_string(),
            turn_number: 2,
        };

        let directives = produce_directives(
            &input,
            &scene,
            &[&bramble],
            Some("The light falls across the rows."),
        );
        assert!(directives[0].visible_context.contains("light falls"));
    }

    #[test]
    fn guidance_changes_with_turn_number() {
        let scene = crate::workshop::the_flute_kept::scene();
        let bramble = crate::workshop::the_flute_kept::bramblehoof();

        let early = produce_directives(
            &PlayerInput {
                text: "x".to_string(),
                turn_number: 1,
            },
            &scene,
            &[&bramble],
            None,
        );
        let late = produce_directives(
            &PlayerInput {
                text: "x".to_string(),
                turn_number: 10,
            },
            &scene,
            &[&bramble],
            None,
        );

        // Early and late guidance should differ
        assert_ne!(early[0].guidance, late[0].guidance);
    }
}
