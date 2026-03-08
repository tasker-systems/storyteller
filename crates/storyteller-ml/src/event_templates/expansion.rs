//! Template expansion — fill templates with vocabulary, compute entity spans.
//!
//! Given a set of templates and a target count per event kind, this module
//! produces annotated examples by randomly sampling slot combinations and
//! tracking character offsets for entity annotations.

use std::collections::BTreeMap;

use rand::seq::IndexedRandom;
use rand::RngCore;

use super::templates::{EventTemplate, SlotContent};
use super::{AnnotatedExample, EntityAnnotation, TextRegister};

/// Segment of a parsed template pattern.
#[derive(Debug)]
enum Segment {
    /// Literal text (copied verbatim).
    Literal(String),
    /// A slot reference (filled from vocabulary).
    Slot(String),
}

/// Parse a template pattern into literal and slot segments.
///
/// Pattern format: `"literal {slot_name} literal {slot_name} literal"`
fn parse_pattern(pattern: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut rest = pattern;

    while let Some(open) = rest.find('{') {
        if open > 0 {
            segments.push(Segment::Literal(rest[..open].to_string()));
        }
        let close = rest[open..]
            .find('}')
            .expect("unclosed { in template pattern")
            + open;
        let name = rest[open + 1..close].to_string();
        segments.push(Segment::Slot(name));
        rest = &rest[close + 1..];
    }

    if !rest.is_empty() {
        segments.push(Segment::Literal(rest.to_string()));
    }

    segments
}

/// Fill a template with specific slot values and compute entity annotations.
///
/// Returns `(generated_text, entity_annotations)`.
fn fill_template(
    template: &EventTemplate,
    slot_values: &BTreeMap<String, String>,
) -> (String, Vec<EntityAnnotation>) {
    let segments = parse_pattern(&template.pattern);
    let mut text = String::new();
    let mut entities = Vec::new();

    for segment in &segments {
        match segment {
            Segment::Literal(lit) => {
                text.push_str(lit);
            }
            Segment::Slot(name) => {
                let value = &slot_values[name];
                let start = text.len();
                text.push_str(value);
                let end = text.len();

                // Check if this slot has an entity annotation
                if let Some(slot_def) = template.slots.iter().find(|s| &s.name == name) {
                    if let Some((category, role)) = &slot_def.entity {
                        entities.push(EntityAnnotation {
                            start,
                            end,
                            text: value.clone(),
                            category: *category,
                            role: role.clone(),
                        });
                    }
                }
            }
        }
    }

    (text, entities)
}

/// Sample a random value for a slot.
fn sample_slot_value(
    content: &SlotContent,
    register: TextRegister,
    rng: &mut dyn RngCore,
) -> String {
    match content {
        SlotContent::Words(words) => words.choose(rng).unwrap().clone(),
        SlotContent::Verbs(pairs) => {
            let pair = pairs.choose(rng).unwrap();
            match register {
                TextRegister::Player => pair.0.clone(),
                TextRegister::Narrator => pair.1.clone(),
            }
        }
    }
}

/// Expand a single template into multiple annotated examples via random sampling.
fn expand_template(
    template: &EventTemplate,
    count: usize,
    rng: &mut dyn RngCore,
) -> Vec<AnnotatedExample> {
    let mut examples = Vec::with_capacity(count);

    for _ in 0..count {
        // Sample a value for each slot
        let mut slot_values = BTreeMap::new();
        for slot in &template.slots {
            let value = sample_slot_value(&slot.content, template.register, rng);
            slot_values.insert(slot.name.clone(), value);
        }

        // Fill template and compute spans
        let (text, entities) = fill_template(template, &slot_values);

        examples.push(AnnotatedExample {
            id: uuid::Uuid::now_v7().to_string(),
            text,
            register: template.register,
            event_kinds: template.event_kinds.clone(),
            action_type: template.action_type.clone(),
            entities,
        });
    }

    examples
}

/// Expand all templates to produce a target count of examples per event kind.
///
/// Templates are grouped by their primary event kind (first in the list).
/// The target count is distributed evenly across templates for each kind.
/// Multi-label templates contribute to all their listed event kinds.
pub fn expand_all(
    templates: &[EventTemplate],
    count_per_kind: usize,
    rng: &mut dyn RngCore,
) -> Vec<AnnotatedExample> {
    // Group templates by primary event kind
    let mut by_kind: BTreeMap<String, Vec<&EventTemplate>> = BTreeMap::new();
    for template in templates {
        if let Some(primary) = template.event_kinds.first() {
            by_kind.entry(primary.clone()).or_default().push(template);
        }
    }

    let mut all_examples = Vec::new();

    for kind_templates in by_kind.values() {
        if kind_templates.is_empty() {
            continue;
        }

        // Distribute count evenly across templates, with remainder going to first
        let per_template = count_per_kind / kind_templates.len();
        let remainder = count_per_kind % kind_templates.len();

        for (i, template) in kind_templates.iter().enumerate() {
            let count = if i < remainder {
                per_template + 1
            } else {
                per_template
            };
            if count > 0 {
                all_examples.extend(expand_template(template, count, rng));
            }
        }
    }

    all_examples
}

#[cfg(test)]
mod tests {
    use rand::SeedableRng;

    use super::*;
    use crate::event_templates::templates::all_templates;

    #[test]
    fn parse_pattern_simple() {
        let segments = parse_pattern("{actor} {verb} {object}");
        assert_eq!(segments.len(), 5); // slot, lit, slot, lit, slot
    }

    #[test]
    fn parse_pattern_with_leading_literal() {
        let segments = parse_pattern("With {manner}, {character} {verb}");
        // "With " + slot + ", " + slot + " " + slot
        assert_eq!(segments.len(), 6);
        assert!(matches!(&segments[0], Segment::Literal(s) if s == "With "));
    }

    #[test]
    fn fill_template_computes_correct_spans() {
        use super::super::templates::{SlotContent, SlotDef};
        use super::super::NerCategory;

        let template = EventTemplate {
            event_kinds: vec!["ActionOccurrence".to_string()],
            action_type: Some("Perform".to_string()),
            register: TextRegister::Player,
            pattern: "{actor} {verb} {object}".to_string(),
            slots: vec![
                SlotDef {
                    name: "actor".to_string(),
                    content: SlotContent::Words(vec!["I".to_string()]),
                    entity: Some((NerCategory::Character, "Actor".to_string())),
                },
                SlotDef {
                    name: "verb".to_string(),
                    content: SlotContent::Words(vec!["pick up".to_string()]),
                    entity: None,
                },
                SlotDef {
                    name: "object".to_string(),
                    content: SlotContent::Words(vec!["the stone".to_string()]),
                    entity: Some((NerCategory::Object, "Target".to_string())),
                },
            ],
        };

        let mut values = BTreeMap::new();
        values.insert("actor".to_string(), "I".to_string());
        values.insert("verb".to_string(), "pick up".to_string());
        values.insert("object".to_string(), "the stone".to_string());

        let (text, entities) = fill_template(&template, &values);
        assert_eq!(text, "I pick up the stone");
        assert_eq!(entities.len(), 2);

        // "I" at position 0..1
        assert_eq!(entities[0].start, 0);
        assert_eq!(entities[0].end, 1);
        assert_eq!(entities[0].text, "I");
        assert_eq!(entities[0].category, NerCategory::Character);

        // "the stone" at position 10..19
        assert_eq!(entities[1].start, 10);
        assert_eq!(entities[1].end, 19);
        assert_eq!(entities[1].text, "the stone");
        assert_eq!(entities[1].category, NerCategory::Object);
    }

    #[test]
    fn expand_template_produces_requested_count() {
        let templates = all_templates();
        let template = &templates[0];
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let examples = expand_template(template, 10, &mut rng);
        assert_eq!(examples.len(), 10);
    }

    #[test]
    fn expand_all_distributes_across_kinds() {
        let templates = all_templates();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let examples = expand_all(&templates, 5, &mut rng);

        // Should have examples for multiple event kinds
        let mut kinds: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for ex in &examples {
            for k in &ex.event_kinds {
                kinds.insert(k.clone());
            }
        }
        assert!(
            kinds.len() >= 5,
            "expected at least 5 event kinds, got {}: {:?}",
            kinds.len(),
            kinds
        );
    }

    #[test]
    fn expanded_examples_have_valid_ids() {
        let templates = all_templates();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let examples = expand_all(&templates, 3, &mut rng);
        for ex in &examples {
            assert!(!ex.id.is_empty());
            // Should be valid UUID
            assert!(
                uuid::Uuid::parse_str(&ex.id).is_ok(),
                "invalid UUID: {}",
                ex.id
            );
        }
    }

    #[test]
    fn entity_text_matches_span_in_generated_text() {
        let templates = all_templates();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let examples = expand_all(&templates, 5, &mut rng);

        for ex in &examples {
            for entity in &ex.entities {
                let span_text = &ex.text[entity.start..entity.end];
                assert_eq!(
                    span_text, entity.text,
                    "span text mismatch in example: {:?}",
                    ex.text
                );
            }
        }
    }
}
