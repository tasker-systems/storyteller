//! Validation for generated event classification training examples.
//!
//! Checks that generated examples are internally consistent:
//! entity spans are within text bounds, span text matches the annotated text,
//! event kinds are from the known vocabulary, and NER categories are valid.

use super::AnnotatedExample;

/// Valid event kind strings (matching `EventKind` variant names in storyteller-core).
const VALID_EVENT_KINDS: &[&str] = &[
    "ActionOccurrence",
    "SpatialChange",
    "SpeechAct",
    "EmotionalExpression",
    "InformationTransfer",
    "StateAssertion",
    "EnvironmentalChange",
    "RelationalShift",
];

/// Valid participant role strings.
const VALID_ROLES: &[&str] = &[
    "Actor",
    "Target",
    "Instrument",
    "Location",
    "Witness",
    "Subject",
];

/// Validate a single annotated example.
///
/// Returns `Ok(())` if valid, or `Err(reasons)` with a list of validation failures.
pub fn validate_example(example: &AnnotatedExample) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    // Text must not be empty
    if example.text.is_empty() {
        errors.push("text is empty".to_string());
    }

    // Must have at least one event kind
    if example.event_kinds.is_empty() {
        errors.push("no event kinds".to_string());
    }

    // All event kinds must be valid
    for kind in &example.event_kinds {
        if !VALID_EVENT_KINDS.contains(&kind.as_str()) {
            errors.push(format!("invalid event kind: {kind}"));
        }
    }

    // Validate entity annotations
    for (i, entity) in example.entities.iter().enumerate() {
        // Span bounds
        if entity.end > example.text.len() {
            errors.push(format!(
                "entity {i}: end ({}) exceeds text length ({})",
                entity.end,
                example.text.len()
            ));
            continue;
        }
        if entity.start >= entity.end {
            errors.push(format!(
                "entity {i}: start ({}) >= end ({})",
                entity.start, entity.end
            ));
            continue;
        }

        // Span text must match
        if example.text.is_char_boundary(entity.start) && example.text.is_char_boundary(entity.end)
        {
            let span_text = &example.text[entity.start..entity.end];
            if span_text != entity.text {
                errors.push(format!(
                    "entity {i}: span text {:?} != annotated text {:?}",
                    span_text, entity.text
                ));
            }
        } else {
            errors.push(format!(
                "entity {i}: span boundaries not on char boundaries"
            ));
        }

        // Role must be valid
        if !VALID_ROLES.contains(&entity.role.as_str()) {
            errors.push(format!("entity {i}: invalid role {:?}", entity.role));
        }
    }

    // Entities must not overlap
    let mut sorted: Vec<_> = example.entities.iter().collect();
    sorted.sort_by_key(|e| (e.start, e.end));
    for window in sorted.windows(2) {
        if window[0].end > window[1].start {
            errors.push(format!(
                "overlapping entities: {:?} and {:?}",
                window[0].text, window[1].text
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{EntityAnnotation, NerCategory, TextRegister};
    use super::*;

    fn valid_example() -> AnnotatedExample {
        AnnotatedExample {
            id: "test-id".to_string(),
            text: "I pick up the stone".to_string(),
            register: TextRegister::Player,
            event_kinds: vec!["ActionOccurrence".to_string()],
            action_type: Some("Perform".to_string()),
            entities: vec![
                EntityAnnotation {
                    start: 0,
                    end: 1,
                    text: "I".to_string(),
                    category: NerCategory::Character,
                    role: "Actor".to_string(),
                },
                EntityAnnotation {
                    start: 10,
                    end: 19,
                    text: "the stone".to_string(),
                    category: NerCategory::Object,
                    role: "Target".to_string(),
                },
            ],
        }
    }

    #[test]
    fn valid_example_passes() {
        assert!(validate_example(&valid_example()).is_ok());
    }

    #[test]
    fn empty_text_fails() {
        let mut ex = valid_example();
        ex.text = String::new();
        assert!(validate_example(&ex).is_err());
    }

    #[test]
    fn invalid_event_kind_fails() {
        let mut ex = valid_example();
        ex.event_kinds = vec!["MadeUpKind".to_string()];
        let err = validate_example(&ex).unwrap_err();
        assert!(err.iter().any(|e| e.contains("invalid event kind")));
    }

    #[test]
    fn span_out_of_bounds_fails() {
        let mut ex = valid_example();
        ex.entities[1].end = 100;
        let err = validate_example(&ex).unwrap_err();
        assert!(err.iter().any(|e| e.contains("exceeds text length")));
    }

    #[test]
    fn span_text_mismatch_fails() {
        let mut ex = valid_example();
        ex.entities[1].text = "wrong text".to_string();
        let err = validate_example(&ex).unwrap_err();
        assert!(err.iter().any(|e| e.contains("span text")));
    }

    #[test]
    fn invalid_role_fails() {
        let mut ex = valid_example();
        ex.entities[0].role = "InvalidRole".to_string();
        let err = validate_example(&ex).unwrap_err();
        assert!(err.iter().any(|e| e.contains("invalid role")));
    }

    #[test]
    fn overlapping_entities_fails() {
        let mut ex = valid_example();
        // Create overlapping entities
        ex.entities[1].start = 0;
        ex.entities[1].end = 5;
        ex.entities[1].text = "I pic".to_string();
        let err = validate_example(&ex).unwrap_err();
        assert!(err.iter().any(|e| e.contains("overlapping")));
    }
}
