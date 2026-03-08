//! Event classification training data generation.
//!
//! See: `docs/ticket-specs/event-system-foundations/phase-c-ml-classification-pipeline.md`
//!
//! Generates annotated text examples for training event classification and
//! entity extraction models. Follows the same pattern as [`super::matrix`]:
//! parameterized templates, combinatorial expansion, JSONL output.
//!
//! The key difference from character prediction training: this module produces
//! *text with annotations* (entity spans, event kind labels) rather than float
//! feature vectors. Entity annotation is programmatic — since we generate the
//! text from templates, we know exactly where the entities are.

pub mod expansion;
pub mod export;
pub mod templates;
pub mod validation;
pub mod vocabulary;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

// ===========================================================================
// Text register — player input vs narrator prose
// ===========================================================================

/// The register of generated text.
///
/// Player input is imperative/first-person ("I pick up the stone").
/// Narrator prose is literary/third-person past tense ("Sarah picked up the stone").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextRegister {
    /// First-person imperative/declarative (player input).
    Player,
    /// Third-person literary past tense (narrator prose).
    Narrator,
}

// ===========================================================================
// NER categories — entity types for extraction
// ===========================================================================

/// Named entity recognition category.
///
/// These categories define the entity extraction vocabulary for the NER model.
/// Each produces B (begin) and I (inside) BIO tags, plus the shared O (outside)
/// tag — 7 categories × 2 + 1 = 15 BIO labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NerCategory {
    /// A character or person — "Sarah", "the old man", "I".
    Character,
    /// A physical object — "the stone", "a sword", "the ancient book".
    Object,
    /// A place or spatial reference — "the riverbed", "the clearing".
    Location,
    /// A gesture or body language — "clenched fists", "turned away".
    Gesture,
    /// A sensory detail — "a distant howl", "the smell of smoke".
    Sensory,
    /// An abstract concept — "the truth", "the betrayal".
    Abstract,
    /// A group or collective — "the villagers", "the council".
    Collective,
}

// ===========================================================================
// Entity annotation — a span in generated text
// ===========================================================================

/// An entity annotation in generated text.
///
/// Character offsets (start inclusive, end exclusive) allow the Python
/// training script to convert these to BIO token labels using the
/// tokenizer's offset mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityAnnotation {
    /// Character offset start (inclusive) in the generated text.
    pub start: usize,
    /// Character offset end (exclusive) in the generated text.
    pub end: usize,
    /// The text of the entity mention.
    pub text: String,
    /// NER category.
    pub category: NerCategory,
    /// Participant role in the event (e.g. "Actor", "Target", "Location").
    pub role: String,
}

// ===========================================================================
// Annotated example — a single training data point
// ===========================================================================

/// A single annotated training example for event classification.
///
/// Each example contains generated text, event kind labels (multi-label),
/// and entity annotations with character-level spans.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotatedExample {
    /// Unique identifier (UUID v7).
    pub id: String,
    /// The generated text.
    pub text: String,
    /// Text register (player or narrator).
    pub register: TextRegister,
    /// Event kind labels — may contain multiple (multi-label classification).
    pub event_kinds: Vec<String>,
    /// Action type (for ActionOccurrence events).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_type: Option<String>,
    /// Entity annotations with character-level spans.
    pub entities: Vec<EntityAnnotation>,
}

/// Statistics from a training data generation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationManifest {
    /// Total examples generated (before validation).
    pub total_generated: usize,
    /// Total examples that passed validation.
    pub total_valid: usize,
    /// Total examples rejected by validation.
    pub total_rejected: usize,
    /// Examples per event kind.
    pub per_event_kind: BTreeMap<String, usize>,
    /// Examples per register.
    pub per_register: BTreeMap<String, usize>,
    /// Generation timestamp.
    pub generated_at: String,
    /// Random seed (if seeded).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
}

/// Generate annotated training examples for event classification.
///
/// Builds all templates, expands them combinatorially with random sampling,
/// validates generated examples, and returns valid examples with a manifest.
pub fn generate(
    count_per_kind: usize,
    seed: Option<u64>,
) -> (Vec<AnnotatedExample>, GenerationManifest) {
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    let mut rng: Box<dyn rand::RngCore> = match seed {
        Some(s) => Box::new(StdRng::seed_from_u64(s)),
        None => Box::new(StdRng::from_os_rng()),
    };

    let all_templates = templates::all_templates();
    let mut examples = expansion::expand_all(&all_templates, count_per_kind, &mut *rng);

    // Validate and partition
    let total_generated = examples.len();
    let mut valid = Vec::new();
    let mut rejected = 0usize;
    for example in examples.drain(..) {
        if validation::validate_example(&example).is_ok() {
            valid.push(example);
        } else {
            rejected += 1;
        }
    }

    // Build manifest
    let mut per_event_kind: BTreeMap<String, usize> = BTreeMap::new();
    let mut per_register: BTreeMap<String, usize> = BTreeMap::new();
    for ex in &valid {
        for kind in &ex.event_kinds {
            *per_event_kind.entry(kind.clone()).or_default() += 1;
        }
        let reg = match ex.register {
            TextRegister::Player => "player",
            TextRegister::Narrator => "narrator",
        };
        *per_register.entry(reg.to_string()).or_default() += 1;
    }

    let manifest = GenerationManifest {
        total_generated,
        total_valid: valid.len(),
        total_rejected: rejected,
        per_event_kind,
        per_register,
        generated_at: chrono::Utc::now().to_rfc3339(),
        seed,
    };

    (valid, manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_register_serde_roundtrip() {
        let player = TextRegister::Player;
        let json = serde_json::to_string(&player).unwrap();
        assert_eq!(json, "\"player\"");
        let narrator: TextRegister = serde_json::from_str("\"narrator\"").unwrap();
        assert_eq!(narrator, TextRegister::Narrator);
    }

    #[test]
    fn ner_category_serializes_screaming_snake() {
        let cat = NerCategory::Character;
        let json = serde_json::to_string(&cat).unwrap();
        assert_eq!(json, "\"CHARACTER\"");
    }

    #[test]
    fn entity_annotation_serde_roundtrip() {
        let ann = EntityAnnotation {
            start: 0,
            end: 5,
            text: "Sarah".to_string(),
            category: NerCategory::Character,
            role: "Actor".to_string(),
        };
        let json = serde_json::to_string(&ann).unwrap();
        let back: EntityAnnotation = serde_json::from_str(&json).unwrap();
        assert_eq!(back.start, 0);
        assert_eq!(back.end, 5);
        assert_eq!(back.text, "Sarah");
    }

    #[test]
    fn annotated_example_skips_none_action_type() {
        let ex = AnnotatedExample {
            id: "test".to_string(),
            text: "I wait".to_string(),
            register: TextRegister::Player,
            event_kinds: vec!["ActionOccurrence".to_string()],
            action_type: None,
            entities: vec![],
        };
        let json = serde_json::to_string(&ex).unwrap();
        assert!(!json.contains("action_type"));
    }

    #[test]
    fn generate_produces_examples_with_seed() {
        let (examples, manifest) = generate(10, Some(42));
        assert!(!examples.is_empty());
        assert_eq!(manifest.seed, Some(42));
        assert!(manifest.total_valid > 0);
        assert_eq!(
            manifest.total_valid + manifest.total_rejected,
            manifest.total_generated
        );
    }

    #[test]
    fn generate_covers_multiple_event_kinds() {
        let (_, manifest) = generate(20, Some(123));
        // Should have at least 3 distinct event kinds represented
        assert!(
            manifest.per_event_kind.len() >= 3,
            "expected at least 3 event kinds, got: {:?}",
            manifest.per_event_kind.keys().collect::<Vec<_>>()
        );
    }
}
