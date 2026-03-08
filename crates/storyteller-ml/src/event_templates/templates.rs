//! Template definitions for event classification training data.
//!
//! Each template maps a pattern string with `{slot}` placeholders to
//! event kind labels and entity annotations. Templates are organized
//! by EventKind and TextRegister.

use super::vocabulary::{self, to_strings, to_verb_pairs};
use super::{NerCategory, TextRegister};

/// How a slot's value is selected during expansion.
#[derive(Debug, Clone)]
pub enum SlotContent {
    /// Simple string list — pick one at random.
    Words(Vec<String>),
    /// Verb pair list `(imperative, past)` — template register determines form.
    Verbs(Vec<(String, String)>),
}

/// A slot definition within a template pattern.
#[derive(Debug, Clone)]
pub struct SlotDef {
    /// Slot name, matching `{name}` in the pattern.
    pub name: String,
    /// Available values for this slot.
    pub content: SlotContent,
    /// If this slot produces an entity annotation: `(NerCategory, role_string)`.
    pub entity: Option<(NerCategory, String)>,
}

/// A template for generating annotated training examples.
#[derive(Debug, Clone)]
pub struct EventTemplate {
    /// Event kind labels this template produces (multi-label).
    pub event_kinds: Vec<String>,
    /// Action type (for `ActionOccurrence` events).
    pub action_type: Option<String>,
    /// Text register.
    pub register: TextRegister,
    /// Pattern string with `{slot_name}` placeholders.
    pub pattern: String,
    /// Slot definitions.
    pub slots: Vec<SlotDef>,
}

// ===========================================================================
// Slot construction helpers
// ===========================================================================

fn entity_slot(name: &str, content: SlotContent, category: NerCategory, role: &str) -> SlotDef {
    SlotDef {
        name: name.to_string(),
        content,
        entity: Some((category, role.to_string())),
    }
}

fn plain_slot(name: &str, content: SlotContent) -> SlotDef {
    SlotDef {
        name: name.to_string(),
        content,
        entity: None,
    }
}

fn words(source: &[&str]) -> SlotContent {
    SlotContent::Words(to_strings(source))
}

fn verbs(source: &[(&str, &str)]) -> SlotContent {
    SlotContent::Verbs(to_verb_pairs(source))
}

// ===========================================================================
// Template construction helpers
// ===========================================================================

fn template(
    event_kinds: &[&str],
    action_type: Option<&str>,
    register: TextRegister,
    pattern: &str,
    slots: Vec<SlotDef>,
) -> EventTemplate {
    EventTemplate {
        event_kinds: event_kinds.iter().map(|s| (*s).to_string()).collect(),
        action_type: action_type.map(|s| s.to_string()),
        register,
        pattern: pattern.to_string(),
        slots,
    }
}

// ===========================================================================
// All templates
// ===========================================================================

/// Returns all event classification templates across all EventKinds and registers.
pub fn all_templates() -> Vec<EventTemplate> {
    let mut templates = Vec::new();
    templates.extend(action_perform_templates());
    templates.extend(action_examine_templates());
    templates.extend(action_wait_resist_templates());
    templates.extend(spatial_change_templates());
    templates.extend(speech_act_templates());
    templates.extend(emotional_expression_templates());
    templates.extend(information_transfer_templates());
    templates.extend(state_assertion_templates());
    templates.extend(environmental_change_templates());
    templates.extend(relational_shift_templates());
    templates.extend(multi_label_templates());
    templates
}

// ===========================================================================
// ActionOccurrence — Perform
// ===========================================================================

fn action_perform_templates() -> Vec<EventTemplate> {
    use vocabulary::*;
    use TextRegister::*;
    vec![
        // Player: "I pick up the stone"
        template(
            &["ActionOccurrence"],
            Some("Perform"),
            Player,
            "{actor} {verb} {object}",
            vec![
                entity_slot(
                    "actor",
                    words(PLAYER_ACTORS),
                    NerCategory::Character,
                    "Actor",
                ),
                plain_slot("verb", verbs(PERFORM_VERBS)),
                entity_slot("object", words(OBJECTS), NerCategory::Object, "Target"),
            ],
        ),
        // Player: "I try to pick up the stone"
        template(
            &["ActionOccurrence"],
            Some("Perform"),
            Player,
            "{actor} try to {verb} {object}",
            vec![
                entity_slot(
                    "actor",
                    words(PLAYER_ACTORS),
                    NerCategory::Character,
                    "Actor",
                ),
                plain_slot("verb", verbs(PERFORM_VERBS)),
                entity_slot("object", words(OBJECTS), NerCategory::Object, "Target"),
            ],
        ),
        // Player: "I pick up the stone carefully"
        template(
            &["ActionOccurrence"],
            Some("Perform"),
            Player,
            "{actor} {verb} {object} {adverb}",
            vec![
                entity_slot(
                    "actor",
                    words(PLAYER_ACTORS),
                    NerCategory::Character,
                    "Actor",
                ),
                plain_slot("verb", verbs(PERFORM_VERBS)),
                entity_slot("object", words(OBJECTS), NerCategory::Object, "Target"),
                plain_slot("adverb", words(ADVERBS)),
            ],
        ),
        // Narrator: "Sarah picked up the stone"
        template(
            &["ActionOccurrence"],
            Some("Perform"),
            Narrator,
            "{character} {verb} {object}",
            vec![
                entity_slot(
                    "character",
                    words(CHARACTER_NAMES),
                    NerCategory::Character,
                    "Actor",
                ),
                plain_slot("verb", verbs(PERFORM_VERBS)),
                entity_slot("object", words(OBJECTS), NerCategory::Object, "Target"),
            ],
        ),
        // Narrator: "Sarah picked up the stone carefully"
        template(
            &["ActionOccurrence"],
            Some("Perform"),
            Narrator,
            "{character} {verb} {object} {adverb}",
            vec![
                entity_slot(
                    "character",
                    words(CHARACTER_NAMES),
                    NerCategory::Character,
                    "Actor",
                ),
                plain_slot("verb", verbs(PERFORM_VERBS)),
                entity_slot("object", words(OBJECTS), NerCategory::Object, "Target"),
                plain_slot("adverb", words(ADVERBS)),
            ],
        ),
        // Narrator: "With trembling hands, Sarah picked up the stone"
        template(
            &["ActionOccurrence"],
            Some("Perform"),
            Narrator,
            "{manner}, {character} {verb} {object}",
            vec![
                plain_slot("manner", words(MANNER_PHRASES)),
                entity_slot(
                    "character",
                    words(CHARACTER_NAMES),
                    NerCategory::Character,
                    "Actor",
                ),
                plain_slot("verb", verbs(PERFORM_VERBS)),
                entity_slot("object", words(OBJECTS), NerCategory::Object, "Target"),
            ],
        ),
    ]
}

// ===========================================================================
// ActionOccurrence — Examine
// ===========================================================================

fn action_examine_templates() -> Vec<EventTemplate> {
    use vocabulary::*;
    use TextRegister::*;
    vec![
        // Player: "I look at the stone"
        template(
            &["ActionOccurrence"],
            Some("Examine"),
            Player,
            "{actor} {verb} {object}",
            vec![
                entity_slot(
                    "actor",
                    words(PLAYER_ACTORS),
                    NerCategory::Character,
                    "Actor",
                ),
                plain_slot("verb", verbs(EXAMINE_VERBS)),
                entity_slot("object", words(OBJECTS), NerCategory::Object, "Target"),
            ],
        ),
        // Player: "I look at the stone carefully"
        template(
            &["ActionOccurrence"],
            Some("Examine"),
            Player,
            "{actor} {verb} {object} {adverb}",
            vec![
                entity_slot(
                    "actor",
                    words(PLAYER_ACTORS),
                    NerCategory::Character,
                    "Actor",
                ),
                plain_slot("verb", verbs(EXAMINE_VERBS)),
                entity_slot("object", words(OBJECTS), NerCategory::Object, "Target"),
                plain_slot("adverb", words(ADVERBS)),
            ],
        ),
        // Narrator: "Sarah examined the stone"
        template(
            &["ActionOccurrence"],
            Some("Examine"),
            Narrator,
            "{character} {verb} {object}",
            vec![
                entity_slot(
                    "character",
                    words(CHARACTER_NAMES),
                    NerCategory::Character,
                    "Actor",
                ),
                plain_slot("verb", verbs(EXAMINE_VERBS)),
                entity_slot("object", words(OBJECTS), NerCategory::Object, "Target"),
            ],
        ),
        // Narrator: "Sarah's gaze settled on the stone"
        template(
            &["ActionOccurrence"],
            Some("Examine"),
            Narrator,
            "{character}'s gaze settled on {object}",
            vec![
                entity_slot(
                    "character",
                    words(CHARACTER_NAMES),
                    NerCategory::Character,
                    "Actor",
                ),
                entity_slot("object", words(OBJECTS), NerCategory::Object, "Target"),
            ],
        ),
    ]
}

// ===========================================================================
// ActionOccurrence — Wait, Resist
// ===========================================================================

fn action_wait_resist_templates() -> Vec<EventTemplate> {
    use vocabulary::*;
    use TextRegister::*;
    vec![
        // Player: "I wait"
        template(
            &["ActionOccurrence"],
            Some("Wait"),
            Player,
            "{actor} wait {adverb}",
            vec![
                entity_slot(
                    "actor",
                    words(PLAYER_ACTORS),
                    NerCategory::Character,
                    "Actor",
                ),
                plain_slot("adverb", words(ADVERBS)),
            ],
        ),
        // Player: "I hold my ground"
        template(
            &["ActionOccurrence"],
            Some("Wait"),
            Player,
            "{actor} hold my ground",
            vec![entity_slot(
                "actor",
                words(PLAYER_ACTORS),
                NerCategory::Character,
                "Actor",
            )],
        ),
        // Narrator: "Sarah waited in silence"
        template(
            &["ActionOccurrence"],
            Some("Wait"),
            Narrator,
            "{character} waited {adverb}",
            vec![
                entity_slot(
                    "character",
                    words(CHARACTER_NAMES),
                    NerCategory::Character,
                    "Actor",
                ),
                plain_slot("adverb", words(ADVERBS)),
            ],
        ),
        // Player: "I refuse to give up the stone"
        template(
            &["ActionOccurrence"],
            Some("Resist"),
            Player,
            "{actor} refuse to give up {object}",
            vec![
                entity_slot(
                    "actor",
                    words(PLAYER_ACTORS),
                    NerCategory::Character,
                    "Actor",
                ),
                entity_slot("object", words(OBJECTS), NerCategory::Object, "Target"),
            ],
        ),
        // Narrator: "Sarah resisted the truth"
        template(
            &["ActionOccurrence"],
            Some("Resist"),
            Narrator,
            "{character} resisted {abstract}",
            vec![
                entity_slot(
                    "character",
                    words(CHARACTER_NAMES),
                    NerCategory::Character,
                    "Actor",
                ),
                entity_slot(
                    "abstract",
                    words(ABSTRACT_CONCEPTS),
                    NerCategory::Abstract,
                    "Target",
                ),
            ],
        ),
    ]
}

// ===========================================================================
// SpatialChange
// ===========================================================================

fn spatial_change_templates() -> Vec<EventTemplate> {
    use vocabulary::*;
    use TextRegister::*;
    vec![
        // Player: "I walk toward the bridge"
        template(
            &["SpatialChange"],
            None,
            Player,
            "{actor} {verb} {location}",
            vec![
                entity_slot(
                    "actor",
                    words(PLAYER_ACTORS),
                    NerCategory::Character,
                    "Actor",
                ),
                plain_slot("verb", verbs(MOVE_VERBS)),
                entity_slot(
                    "location",
                    words(LOCATIONS),
                    NerCategory::Location,
                    "Location",
                ),
            ],
        ),
        // Player: "I head for the bridge quickly"
        template(
            &["SpatialChange"],
            None,
            Player,
            "{actor} {verb} {location} {adverb}",
            vec![
                entity_slot(
                    "actor",
                    words(PLAYER_ACTORS),
                    NerCategory::Character,
                    "Actor",
                ),
                plain_slot("verb", verbs(MOVE_VERBS)),
                entity_slot(
                    "location",
                    words(LOCATIONS),
                    NerCategory::Location,
                    "Location",
                ),
                plain_slot("adverb", words(ADVERBS)),
            ],
        ),
        // Narrator: "Sarah walked toward the bridge"
        template(
            &["SpatialChange"],
            None,
            Narrator,
            "{character} {verb} {location}",
            vec![
                entity_slot(
                    "character",
                    words(CHARACTER_NAMES),
                    NerCategory::Character,
                    "Actor",
                ),
                plain_slot("verb", verbs(MOVE_VERBS)),
                entity_slot(
                    "location",
                    words(LOCATIONS),
                    NerCategory::Location,
                    "Location",
                ),
            ],
        ),
        // Narrator: "Sarah left the clearing behind"
        template(
            &["SpatialChange"],
            None,
            Narrator,
            "{character} left {location} behind",
            vec![
                entity_slot(
                    "character",
                    words(CHARACTER_NAMES),
                    NerCategory::Character,
                    "Actor",
                ),
                entity_slot(
                    "location",
                    words(LOCATIONS),
                    NerCategory::Location,
                    "Location",
                ),
            ],
        ),
    ]
}

// ===========================================================================
// SpeechAct
// ===========================================================================

fn speech_act_templates() -> Vec<EventTemplate> {
    use vocabulary::*;
    use TextRegister::*;
    vec![
        // Player: "I ask Adam about the curse"
        template(
            &["SpeechAct"],
            None,
            Player,
            "{actor} ask {character} about {topic}",
            vec![
                entity_slot(
                    "actor",
                    words(PLAYER_ACTORS),
                    NerCategory::Character,
                    "Actor",
                ),
                entity_slot(
                    "character",
                    words(CHARACTER_NAMES),
                    NerCategory::Character,
                    "Target",
                ),
                entity_slot("topic", words(TOPICS), NerCategory::Abstract, "Subject"),
            ],
        ),
        // Player: "I tell Adam the truth"
        template(
            &["SpeechAct"],
            None,
            Player,
            "{actor} tell {character} about {abstract}",
            vec![
                entity_slot(
                    "actor",
                    words(PLAYER_ACTORS),
                    NerCategory::Character,
                    "Actor",
                ),
                entity_slot(
                    "character",
                    words(CHARACTER_NAMES),
                    NerCategory::Character,
                    "Target",
                ),
                entity_slot(
                    "abstract",
                    words(ABSTRACT_CONCEPTS),
                    NerCategory::Abstract,
                    "Subject",
                ),
            ],
        ),
        // Player: "I say 'Where are you going?'"
        template(
            &["SpeechAct"],
            None,
            Player,
            "{actor} say '{dialogue}'",
            vec![
                entity_slot(
                    "actor",
                    words(PLAYER_ACTORS),
                    NerCategory::Character,
                    "Actor",
                ),
                plain_slot("dialogue", words(DIALOGUE_LINES)),
            ],
        ),
        // Narrator: "'Where are you going?' Sarah said quietly"
        template(
            &["SpeechAct"],
            None,
            Narrator,
            "'{dialogue},' {character} {speech_verb} {adverb}",
            vec![
                plain_slot("dialogue", words(DIALOGUE_LINES)),
                entity_slot(
                    "character",
                    words(CHARACTER_NAMES),
                    NerCategory::Character,
                    "Actor",
                ),
                plain_slot("speech_verb", words(SPEECH_VERBS_PAST)),
                plain_slot("adverb", words(ADVERBS)),
            ],
        ),
        // Narrator: "Sarah asked Adam about the curse"
        template(
            &["SpeechAct"],
            None,
            Narrator,
            "{character} asked {character2} about {topic}",
            vec![
                entity_slot(
                    "character",
                    words(CHARACTER_NAMES),
                    NerCategory::Character,
                    "Actor",
                ),
                entity_slot(
                    "character2",
                    words(CHARACTER_NAMES),
                    NerCategory::Character,
                    "Target",
                ),
                entity_slot("topic", words(TOPICS), NerCategory::Abstract, "Subject"),
            ],
        ),
    ]
}

// ===========================================================================
// EmotionalExpression
// ===========================================================================

fn emotional_expression_templates() -> Vec<EventTemplate> {
    use vocabulary::*;
    use TextRegister::*;
    vec![
        // Player: "I can't hold back tears" (rare from players)
        template(
            &["EmotionalExpression"],
            None,
            Player,
            "{actor} feel {emotion_adj}",
            vec![
                entity_slot(
                    "actor",
                    words(PLAYER_ACTORS),
                    NerCategory::Character,
                    "Actor",
                ),
                plain_slot("emotion_adj", words(EMOTION_ADJS)),
            ],
        ),
        // Narrator: "A sorrowful look crossed Sarah's face"
        template(
            &["EmotionalExpression"],
            None,
            Narrator,
            "A {emotion_adj} look crossed {character}'s face",
            vec![
                plain_slot("emotion_adj", words(EMOTION_ADJS)),
                entity_slot(
                    "character",
                    words(CHARACTER_NAMES),
                    NerCategory::Character,
                    "Actor",
                ),
            ],
        ),
        // Narrator: "Tears welled in Sarah's eyes"
        template(
            &["EmotionalExpression"],
            None,
            Narrator,
            "Tears welled in {character}'s eyes",
            vec![entity_slot(
                "character",
                words(CHARACTER_NAMES),
                NerCategory::Character,
                "Actor",
            )],
        ),
        // Narrator: "Sarah trembled visibly"
        template(
            &["EmotionalExpression"],
            None,
            Narrator,
            "{character} {gesture}",
            vec![
                entity_slot(
                    "character",
                    words(CHARACTER_NAMES),
                    NerCategory::Character,
                    "Actor",
                ),
                entity_slot(
                    "gesture",
                    words(GESTURES),
                    NerCategory::Gesture,
                    "Instrument",
                ),
            ],
        ),
    ]
}

// ===========================================================================
// InformationTransfer
// ===========================================================================

fn information_transfer_templates() -> Vec<EventTemplate> {
    use vocabulary::*;
    use TextRegister::*;
    vec![
        // Player: "I show Adam the map"
        template(
            &["InformationTransfer"],
            None,
            Player,
            "{actor} show {character} {object}",
            vec![
                entity_slot(
                    "actor",
                    words(PLAYER_ACTORS),
                    NerCategory::Character,
                    "Actor",
                ),
                entity_slot(
                    "character",
                    words(CHARACTER_NAMES),
                    NerCategory::Character,
                    "Target",
                ),
                entity_slot("object", words(OBJECTS), NerCategory::Object, "Instrument"),
            ],
        ),
        // Player: "I tell Adam about the hidden passage"
        template(
            &["InformationTransfer"],
            None,
            Player,
            "{actor} reveal {abstract} to {character}",
            vec![
                entity_slot(
                    "actor",
                    words(PLAYER_ACTORS),
                    NerCategory::Character,
                    "Actor",
                ),
                entity_slot(
                    "abstract",
                    words(ABSTRACT_CONCEPTS),
                    NerCategory::Abstract,
                    "Subject",
                ),
                entity_slot(
                    "character",
                    words(CHARACTER_NAMES),
                    NerCategory::Character,
                    "Target",
                ),
            ],
        ),
        // Narrator: "Sarah revealed the secret to Adam"
        template(
            &["InformationTransfer"],
            None,
            Narrator,
            "{character} revealed {abstract} to {character2}",
            vec![
                entity_slot(
                    "character",
                    words(CHARACTER_NAMES),
                    NerCategory::Character,
                    "Actor",
                ),
                entity_slot(
                    "abstract",
                    words(ABSTRACT_CONCEPTS),
                    NerCategory::Abstract,
                    "Subject",
                ),
                entity_slot(
                    "character2",
                    words(CHARACTER_NAMES),
                    NerCategory::Character,
                    "Target",
                ),
            ],
        ),
        // Narrator: "Sarah shared the truth with Adam"
        template(
            &["InformationTransfer"],
            None,
            Narrator,
            "{character} shared {abstract} with {character2}",
            vec![
                entity_slot(
                    "character",
                    words(CHARACTER_NAMES),
                    NerCategory::Character,
                    "Actor",
                ),
                entity_slot(
                    "abstract",
                    words(ABSTRACT_CONCEPTS),
                    NerCategory::Abstract,
                    "Subject",
                ),
                entity_slot(
                    "character2",
                    words(CHARACTER_NAMES),
                    NerCategory::Character,
                    "Target",
                ),
            ],
        ),
    ]
}

// ===========================================================================
// StateAssertion
// ===========================================================================

fn state_assertion_templates() -> Vec<EventTemplate> {
    use vocabulary::*;
    use TextRegister::*;
    vec![
        // Narrator: "Sarah was sitting alone"
        template(
            &["StateAssertion"],
            None,
            Narrator,
            "{character} was {state}",
            vec![
                entity_slot(
                    "character",
                    words(CHARACTER_NAMES),
                    NerCategory::Character,
                    "Actor",
                ),
                plain_slot("state", words(STATES)),
            ],
        ),
        // Narrator: "The stone lay beside the hearth"
        template(
            &["StateAssertion"],
            None,
            Narrator,
            "{object} lay beside {location}",
            vec![
                entity_slot("object", words(OBJECTS), NerCategory::Object, "Target"),
                entity_slot(
                    "location",
                    words(LOCATIONS),
                    NerCategory::Location,
                    "Location",
                ),
            ],
        ),
        // Player: "I am standing by the bridge"
        template(
            &["StateAssertion"],
            None,
            Player,
            "{actor} am {state}",
            vec![
                entity_slot(
                    "actor",
                    words(PLAYER_ACTORS),
                    NerCategory::Character,
                    "Actor",
                ),
                plain_slot("state", words(STATES)),
            ],
        ),
    ]
}

// ===========================================================================
// EnvironmentalChange
// ===========================================================================

fn environmental_change_templates() -> Vec<EventTemplate> {
    use vocabulary::*;
    use TextRegister::*;
    vec![
        // Narrator: standalone environmental changes
        template(
            &["EnvironmentalChange"],
            None,
            Narrator,
            "{env_change}",
            vec![plain_slot("env_change", words(ENVIRONMENT_CHANGES))],
        ),
        // Narrator: "A cold wind swept across the clearing"
        template(
            &["EnvironmentalChange"],
            None,
            Narrator,
            "{sensory} filled {location}",
            vec![
                entity_slot(
                    "sensory",
                    words(SENSORY_DETAILS),
                    NerCategory::Sensory,
                    "Subject",
                ),
                entity_slot(
                    "location",
                    words(LOCATIONS),
                    NerCategory::Location,
                    "Location",
                ),
            ],
        ),
        // Narrator: environmental + location
        template(
            &["EnvironmentalChange"],
            None,
            Narrator,
            "The air grew still around {location}",
            vec![entity_slot(
                "location",
                words(LOCATIONS),
                NerCategory::Location,
                "Location",
            )],
        ),
    ]
}

// ===========================================================================
// RelationalShift (interpretive — usually narrator)
// ===========================================================================

fn relational_shift_templates() -> Vec<EventTemplate> {
    use vocabulary::*;
    use TextRegister::*;
    vec![
        // Narrator: "Something changed between Sarah and Adam"
        template(
            &["RelationalShift"],
            None,
            Narrator,
            "Something changed between {character} and {character2}",
            vec![
                entity_slot(
                    "character",
                    words(CHARACTER_NAMES),
                    NerCategory::Character,
                    "Actor",
                ),
                entity_slot(
                    "character2",
                    words(CHARACTER_NAMES),
                    NerCategory::Character,
                    "Target",
                ),
            ],
        ),
        // Narrator: "Sarah turned away from Adam"
        template(
            &["RelationalShift"],
            None,
            Narrator,
            "{character} turned away from {character2}",
            vec![
                entity_slot(
                    "character",
                    words(CHARACTER_NAMES),
                    NerCategory::Character,
                    "Actor",
                ),
                entity_slot(
                    "character2",
                    words(CHARACTER_NAMES),
                    NerCategory::Character,
                    "Target",
                ),
            ],
        ),
    ]
}

// ===========================================================================
// Multi-label templates — single text, multiple EventKinds
// ===========================================================================

fn multi_label_templates() -> Vec<EventTemplate> {
    use vocabulary::*;
    use TextRegister::*;
    vec![
        // "I grab the sword and run for the door" → ActionOccurrence + SpatialChange
        template(
            &["ActionOccurrence", "SpatialChange"],
            Some("Perform"),
            Player,
            "{actor} {verb} {object} and run for {location}",
            vec![
                entity_slot(
                    "actor",
                    words(PLAYER_ACTORS),
                    NerCategory::Character,
                    "Actor",
                ),
                plain_slot("verb", verbs(PERFORM_VERBS)),
                entity_slot("object", words(OBJECTS), NerCategory::Object, "Target"),
                entity_slot(
                    "location",
                    words(LOCATIONS),
                    NerCategory::Location,
                    "Location",
                ),
            ],
        ),
        // "I tell Adam about the hidden passage while heading toward the bridge"
        // → SpeechAct + SpatialChange
        template(
            &["SpeechAct", "SpatialChange"],
            None,
            Player,
            "{actor} tell {character} about {topic} while heading toward {location}",
            vec![
                entity_slot(
                    "actor",
                    words(PLAYER_ACTORS),
                    NerCategory::Character,
                    "Actor",
                ),
                entity_slot(
                    "character",
                    words(CHARACTER_NAMES),
                    NerCategory::Character,
                    "Target",
                ),
                entity_slot("topic", words(TOPICS), NerCategory::Abstract, "Subject"),
                entity_slot(
                    "location",
                    words(LOCATIONS),
                    NerCategory::Location,
                    "Location",
                ),
            ],
        ),
        // Narrator: "Sarah stormed out of the clearing" → SpatialChange + EmotionalExpression
        template(
            &["SpatialChange", "EmotionalExpression"],
            None,
            Narrator,
            "{character} stormed out of {location}",
            vec![
                entity_slot(
                    "character",
                    words(CHARACTER_NAMES),
                    NerCategory::Character,
                    "Actor",
                ),
                entity_slot(
                    "location",
                    words(LOCATIONS),
                    NerCategory::Location,
                    "Location",
                ),
            ],
        ),
        // Narrator: "Sarah shared the secret, her voice trembling"
        // → InformationTransfer + EmotionalExpression
        template(
            &["InformationTransfer", "EmotionalExpression"],
            None,
            Narrator,
            "{character} shared {abstract}, voice trembling",
            vec![
                entity_slot(
                    "character",
                    words(CHARACTER_NAMES),
                    NerCategory::Character,
                    "Actor",
                ),
                entity_slot(
                    "abstract",
                    words(ABSTRACT_CONCEPTS),
                    NerCategory::Abstract,
                    "Subject",
                ),
            ],
        ),
        // Player: "I show Adam the map and ask about the path ahead"
        // → InformationTransfer + SpeechAct
        template(
            &["InformationTransfer", "SpeechAct"],
            None,
            Player,
            "{actor} show {character} {object} and ask about {topic}",
            vec![
                entity_slot(
                    "actor",
                    words(PLAYER_ACTORS),
                    NerCategory::Character,
                    "Actor",
                ),
                entity_slot(
                    "character",
                    words(CHARACTER_NAMES),
                    NerCategory::Character,
                    "Target",
                ),
                entity_slot("object", words(OBJECTS), NerCategory::Object, "Instrument"),
                entity_slot("topic", words(TOPICS), NerCategory::Abstract, "Subject"),
            ],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_templates_is_nonempty() {
        let templates = all_templates();
        assert!(
            templates.len() >= 30,
            "expected at least 30 templates, got {}",
            templates.len()
        );
    }

    #[test]
    fn all_templates_have_valid_event_kinds() {
        let valid_kinds = [
            "ActionOccurrence",
            "SpatialChange",
            "SpeechAct",
            "EmotionalExpression",
            "InformationTransfer",
            "StateAssertion",
            "EnvironmentalChange",
            "RelationalShift",
        ];
        for template in all_templates() {
            for kind in &template.event_kinds {
                assert!(
                    valid_kinds.contains(&kind.as_str()),
                    "invalid event kind: {kind}"
                );
            }
        }
    }

    #[test]
    fn all_templates_have_matching_slots() {
        for template in all_templates() {
            for slot in &template.slots {
                let placeholder = format!("{{{}}}", slot.name);
                assert!(
                    template.pattern.contains(&placeholder),
                    "template pattern {:?} missing slot placeholder {placeholder}",
                    template.pattern
                );
            }
        }
    }

    #[test]
    fn all_slots_have_nonempty_choices() {
        for template in all_templates() {
            for slot in &template.slots {
                let count = match &slot.content {
                    SlotContent::Words(w) => w.len(),
                    SlotContent::Verbs(v) => v.len(),
                };
                assert!(
                    count > 0,
                    "slot {:?} in template {:?} has no choices",
                    slot.name,
                    template.pattern
                );
            }
        }
    }

    #[test]
    fn multi_label_templates_have_multiple_kinds() {
        let templates = multi_label_templates();
        for t in &templates {
            assert!(
                t.event_kinds.len() >= 2,
                "multi-label template should have >=2 kinds: {:?}",
                t.event_kinds
            );
        }
    }

    #[test]
    fn both_registers_represented() {
        let templates = all_templates();
        let has_player = templates.iter().any(|t| t.register == TextRegister::Player);
        let has_narrator = templates
            .iter()
            .any(|t| t.register == TextRegister::Narrator);
        assert!(has_player, "should have player-register templates");
        assert!(has_narrator, "should have narrator-register templates");
    }
}
