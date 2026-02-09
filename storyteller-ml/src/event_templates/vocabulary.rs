//! Vocabulary lists for event classification training data generation.
//!
//! Organized by semantic category. Word lists are genre-appropriate for
//! fantasy narrative (the primary storyteller context). Each list provides
//! diverse entries for combinatorial expansion.
//!
//! Verb entries are `(imperative, past_tense)` pairs so templates can
//! select the correct form based on text register (player vs narrator).

/// Player-register actor — always first person.
pub const PLAYER_ACTORS: &[&str] = &["I"];

/// Character names for narrator-register templates.
/// Mix of specific names (from test content) and generic descriptions.
pub const CHARACTER_NAMES: &[&str] = &[
    "Sarah",
    "Tom",
    "Kate",
    "Adam",
    "Beth",
    "Bramblehoof",
    "Pyotir",
    "Illyana",
    "the woman",
    "the old man",
    "the stranger",
    "the child",
    "the guard",
    "the healer",
    "the merchant",
    "the elder",
    "the knight",
    "the hunter",
    "the shepherd",
    "the traveler",
];

/// Physical objects that can be interacted with.
pub const OBJECTS: &[&str] = &[
    "the stone",
    "the sword",
    "the key",
    "the book",
    "the lantern",
    "the flask",
    "the map",
    "the amulet",
    "the door",
    "the chest",
    "the cloak",
    "the staff",
    "the mirror",
    "the coin",
    "the feather",
    "the ring",
    "the scroll",
    "the bread",
    "the cup",
    "the bone",
    "the letter",
    "the pouch",
    "the charm",
    "the flute",
];

/// Spatial locations and landmarks.
pub const LOCATIONS: &[&str] = &[
    "the riverbed",
    "the clearing",
    "the doorway",
    "the bridge",
    "the cave",
    "the market",
    "the tower",
    "the garden",
    "the ridge",
    "the path",
    "the threshold",
    "the hearth",
    "the crossroads",
    "the shore",
    "the forest edge",
    "the village square",
    "the chapel",
    "the cellar",
    "the hillside",
    "the camp",
];

/// Physical action verbs: `(imperative, past_tense)`.
pub const PERFORM_VERBS: &[(&str, &str)] = &[
    ("pick up", "picked up"),
    ("grab", "grabbed"),
    ("take", "took"),
    ("push", "pushed"),
    ("pull", "pulled"),
    ("open", "opened"),
    ("close", "closed"),
    ("break", "broke"),
    ("light", "lit"),
    ("place", "placed"),
    ("throw", "threw"),
    ("drop", "dropped"),
    ("touch", "touched"),
    ("hold", "held"),
    ("lift", "lifted"),
    ("catch", "caught"),
    ("turn", "turned"),
    ("shake", "shook"),
    ("press", "pressed"),
    ("carry", "carried"),
    ("draw", "drew"),
    ("set down", "set down"),
    ("offer", "offered"),
    ("hide", "hid"),
    ("unfold", "unfolded"),
];

/// Observation/perception verbs: `(imperative, past_tense)`.
pub const EXAMINE_VERBS: &[(&str, &str)] = &[
    ("look at", "looked at"),
    ("examine", "examined"),
    ("study", "studied"),
    ("inspect", "inspected"),
    ("peer at", "peered at"),
    ("watch", "watched"),
    ("observe", "observed"),
    ("notice", "noticed"),
    ("gaze at", "gazed at"),
    ("survey", "surveyed"),
    ("regard", "regarded"),
    ("scrutinize", "scrutinized"),
    ("consider", "considered"),
    ("search", "searched"),
    ("check", "checked"),
];

/// Movement verbs: `(imperative, past_tense)`.
/// The destination follows the verb directly (no preposition in the pair).
pub const MOVE_VERBS: &[(&str, &str)] = &[
    ("walk toward", "walked toward"),
    ("run to", "ran to"),
    ("step toward", "stepped toward"),
    ("crawl to", "crawled to"),
    ("climb toward", "climbed toward"),
    ("approach", "approached"),
    ("head toward", "headed toward"),
    ("move to", "moved to"),
    ("venture toward", "ventured toward"),
    ("retreat from", "retreated from"),
    ("flee from", "fled from"),
    ("leave", "left"),
    ("enter", "entered"),
    ("return to", "returned to"),
    ("cross to", "crossed to"),
];

/// Past-tense speech verbs for narrator attribution.
pub const SPEECH_VERBS_PAST: &[&str] = &[
    "said",
    "whispered",
    "shouted",
    "muttered",
    "asked",
    "declared",
    "murmured",
    "called out",
    "replied",
    "answered",
    "breathed",
    "hissed",
    "pleaded",
    "insisted",
];

/// Manner adverbs.
pub const ADVERBS: &[&str] = &[
    "carefully",
    "quickly",
    "slowly",
    "quietly",
    "gently",
    "firmly",
    "hesitantly",
    "urgently",
    "deliberately",
    "nervously",
    "cautiously",
    "eagerly",
    "silently",
    "gracefully",
    "roughly",
    "tenderly",
    "wearily",
    "fiercely",
    "stubbornly",
    "abruptly",
];

/// Prepositional manner phrases (used after "with" or standalone).
pub const MANNER_PHRASES: &[&str] = &[
    "with trembling hands",
    "with great care",
    "in silence",
    "with a heavy heart",
    "without hesitation",
    "with practiced ease",
    "in desperation",
    "with quiet resolve",
    "with shaking fingers",
    "through clenched teeth",
    "with a steadying breath",
    "with visible reluctance",
];

/// Conversational topics.
pub const TOPICS: &[&str] = &[
    "the path ahead",
    "the danger",
    "the curse",
    "the old ways",
    "the missing child",
    "the prophecy",
    "the hidden passage",
    "the covenant",
    "the war",
    "the harvest",
    "the remedy",
    "the boundary",
    "the fair folk",
    "the dead",
    "the crossing",
];

/// Short dialogue lines for speech act templates.
pub const DIALOGUE_LINES: &[&str] = &[
    "Where are you going?",
    "You shouldn't be here.",
    "I don't understand.",
    "Tell me the truth.",
    "What happened?",
    "We need to leave.",
    "I remember now.",
    "It's not what you think.",
    "Help me.",
    "There's no time.",
    "I'm sorry.",
    "Promise me.",
    "Do you trust me?",
    "Listen carefully.",
    "Something is wrong.",
];

/// Emotion adjectives.
pub const EMOTION_ADJS: &[&str] = &[
    "sorrowful",
    "angry",
    "fearful",
    "joyful",
    "surprised",
    "disgusted",
    "hopeful",
    "anxious",
    "bitter",
    "tender",
    "ashamed",
    "defiant",
    "weary",
    "desperate",
    "serene",
];

/// Abstract concepts for information transfer and relational contexts.
pub const ABSTRACT_CONCEPTS: &[&str] = &[
    "the truth",
    "the betrayal",
    "the covenant",
    "the memory",
    "the loss",
    "the hope",
    "the danger",
    "the secret",
    "the sacrifice",
    "the debt",
    "the promise",
    "the burden",
    "the past",
    "the choice",
];

/// Sensory details for perception and environmental events.
pub const SENSORY_DETAILS: &[&str] = &[
    "a distant howl",
    "the smell of smoke",
    "a cold wind",
    "a sudden silence",
    "the crack of a branch",
    "the rustle of leaves",
    "a faint glow",
    "the taste of iron",
    "a tremor in the earth",
    "the sound of water",
    "a flicker of shadow",
    "birdsong",
];

/// State descriptions for state assertion templates.
pub const STATES: &[&str] = &[
    "sitting alone",
    "standing guard",
    "sleeping fitfully",
    "hiding in the shadows",
    "watching the horizon",
    "waiting by the fire",
    "trembling",
    "pale and drawn",
    "armed and ready",
    "lost in thought",
];

/// Environmental changes (complete clauses for narrator templates).
pub const ENVIRONMENT_CHANGES: &[&str] = &[
    "The wind shifted",
    "Rain began to fall",
    "The light dimmed",
    "Snow started falling",
    "The ground trembled",
    "Thunder rumbled in the distance",
    "The air grew cold",
    "Fog rolled in from the river",
    "The sun broke through the clouds",
    "Darkness settled over the land",
];

/// Collective nouns.
pub const COLLECTIVES: &[&str] = &[
    "the villagers",
    "the council",
    "the soldiers",
    "the refugees",
    "the pilgrims",
    "the children",
    "the elders",
    "the healers",
    "the merchants",
    "the guards",
];

/// Gesture/body language descriptions.
pub const GESTURES: &[&str] = &[
    "clenched fists",
    "turned away",
    "trembled visibly",
    "stiffened",
    "lowered eyes",
    "set jaw",
    "crossed arms",
    "bowed head",
    "wrung hands",
    "stepped back",
    "covered mouth",
    "bit lip",
];

// ===========================================================================
// Helper functions to convert static slices to owned Vecs
// ===========================================================================

/// Convert a static string slice to owned strings.
pub fn to_strings(source: &[&str]) -> Vec<String> {
    source.iter().map(|s| (*s).to_string()).collect()
}

/// Convert static verb pairs to owned pairs.
pub fn to_verb_pairs(source: &[(&str, &str)]) -> Vec<(String, String)> {
    source
        .iter()
        .map(|(a, b)| ((*a).to_string(), (*b).to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_vocab_lists_are_nonempty() {
        assert!(!PLAYER_ACTORS.is_empty());
        assert!(!CHARACTER_NAMES.is_empty());
        assert!(!OBJECTS.is_empty());
        assert!(!LOCATIONS.is_empty());
        assert!(!PERFORM_VERBS.is_empty());
        assert!(!EXAMINE_VERBS.is_empty());
        assert!(!MOVE_VERBS.is_empty());
        assert!(!SPEECH_VERBS_PAST.is_empty());
        assert!(!ADVERBS.is_empty());
        assert!(!MANNER_PHRASES.is_empty());
        assert!(!TOPICS.is_empty());
        assert!(!DIALOGUE_LINES.is_empty());
        assert!(!EMOTION_ADJS.is_empty());
        assert!(!ABSTRACT_CONCEPTS.is_empty());
        assert!(!SENSORY_DETAILS.is_empty());
        assert!(!STATES.is_empty());
        assert!(!ENVIRONMENT_CHANGES.is_empty());
        assert!(!COLLECTIVES.is_empty());
        assert!(!GESTURES.is_empty());
    }

    #[test]
    fn verb_pairs_have_both_forms() {
        for (imp, past) in PERFORM_VERBS {
            assert!(!imp.is_empty(), "imperative should not be empty");
            assert!(!past.is_empty(), "past tense should not be empty");
        }
    }

    #[test]
    fn to_strings_converts_correctly() {
        let owned = to_strings(&["a", "b"]);
        assert_eq!(owned, vec!["a".to_string(), "b".to_string()]);
    }
}
