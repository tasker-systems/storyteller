//! Reconciler agent — multi-character scene coordinator.
//!
//! See: `docs/foundation/system_architecture.md`, `docs/technical/agent-message-catalog.md`
//!
//! Sequences overlapping actions, resolves conflicts, surfaces dramatic
//! potential. Adds no content — only structures what Character Agents express.
//!
//! For the prototype (two characters), this is a deterministic Rust function.

use storyteller_core::types::message::{CharacterIntent, ReconcilerOutput};

/// Reconcile multiple character intents into a sequenced output.
///
/// For two characters: sequences both intents and adds a brief scene
/// dynamics note describing the interplay. For a single character,
/// passes through unchanged.
pub fn reconcile(intents: Vec<CharacterIntent>) -> ReconcilerOutput {
    if intents.is_empty() {
        return ReconcilerOutput {
            sequenced_intents: Vec::new(),
            scene_dynamics: String::new(),
        };
    }

    if intents.len() == 1 {
        return ReconcilerOutput {
            scene_dynamics: format!("{} acts alone in this beat.", intents[0].character_name),
            sequenced_intents: intents,
        };
    }

    let dynamics = describe_dynamics(&intents);

    ReconcilerOutput {
        sequenced_intents: intents,
        scene_dynamics: dynamics,
    }
}

/// Generate a brief dynamics note from the character intents.
fn describe_dynamics(intents: &[CharacterIntent]) -> String {
    let names: Vec<&str> = intents.iter().map(|i| i.character_name.as_str()).collect();

    // Look for dialogue in both intents (presence of quotation marks in action)
    let speakers: Vec<&str> = intents
        .iter()
        .filter(|i| {
            i.intent.contains('"') || i.intent.contains("says") || i.intent.contains("speaks")
        })
        .map(|i| i.character_name.as_str())
        .collect();

    // Look for physical actions
    let movers: Vec<&str> = intents
        .iter()
        .filter(|i| {
            let lower = i.intent.to_lowercase();
            lower.contains("step")
                || lower.contains("walk")
                || lower.contains("turn")
                || lower.contains("reach")
                || lower.contains("look")
                || lower.contains("nod")
                || lower.contains("hand")
                || lower.contains("move")
        })
        .map(|i| i.character_name.as_str())
        .collect();

    let mut notes = Vec::new();

    match speakers.len() {
        0 => notes.push("A beat of silence between them.".to_string()),
        1 => notes.push(format!("{} speaks; the other listens.", speakers[0])),
        _ => notes.push(format!("{} exchange words.", names.join(" and "))),
    }

    if movers.len() >= 2 {
        notes.push("Both characters are physically active in this beat.".to_string());
    }

    // Check for contrasting subtext
    let subtexts_differ = intents.len() >= 2
        && intents[0].emotional_subtext != intents[1].emotional_subtext
        && !intents[0].emotional_subtext.contains("[not provided]")
        && !intents[1].emotional_subtext.contains("[not provided]");

    if subtexts_differ {
        notes.push("Their inner states are pulling in different directions.".to_string());
    }

    notes.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use storyteller_core::types::entity::EntityId;

    fn make_intent(name: &str, intent: &str, subtext: &str) -> CharacterIntent {
        CharacterIntent {
            character_id: EntityId::new(),
            character_name: name.to_string(),
            intent: intent.to_string(),
            emotional_subtext: subtext.to_string(),
            internal_state: "thinking".to_string(),
        }
    }

    #[test]
    fn empty_intents_produces_empty_output() {
        let result = reconcile(Vec::new());
        assert!(result.sequenced_intents.is_empty());
    }

    #[test]
    fn single_intent_passes_through() {
        let intent = make_intent("Bramblehoof", "He steps forward.", "nervous");
        let result = reconcile(vec![intent]);
        assert_eq!(result.sequenced_intents.len(), 1);
        assert!(result.scene_dynamics.contains("Bramblehoof"));
    }

    #[test]
    fn two_intents_are_sequenced() {
        let a = make_intent(
            "Bramblehoof",
            "\"Hello,\" he says warmly.",
            "tightness in his chest",
        );
        let b = make_intent(
            "Pyotir",
            "He nods without looking up.",
            "a flash of recognition",
        );
        let result = reconcile(vec![a, b]);
        assert_eq!(result.sequenced_intents.len(), 2);
        assert!(!result.scene_dynamics.is_empty());
    }

    #[test]
    fn detects_contrasting_subtext() {
        let a = make_intent("Bramblehoof", "He reaches out.", "hope swelling");
        let b = make_intent("Pyotir", "He steps back.", "cold distance");
        let result = reconcile(vec![a, b]);
        assert!(result.scene_dynamics.contains("different directions"));
    }
}
