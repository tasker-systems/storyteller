//! Label constants for event classification and entity extraction models.
//!
//! Shared encoding contract between the Python training pipeline
//! (`training/event_classifier/src/event_classifier/schema.py`) and
//! Rust inference (`storyteller-engine/src/inference/event_classifier.rs`).
//!
//! Same pattern as [`super::feature_schema`]: both sides agree on indices
//! and string labels. Any change here must be mirrored in `schema.py`.

use crate::event_templates::NerCategory;

// ===========================================================================
// Event classification labels (sequence-level, multi-label)
// ===========================================================================

/// The 8 classifiable EventKinds, in label-index order.
///
/// `SceneLifecycle` and `EntityLifecycle` are system-generated events,
/// not extracted from text by the classifier.
pub const EVENT_KIND_LABELS: [&str; 8] = [
    "StateAssertion",
    "ActionOccurrence",
    "SpatialChange",
    "EmotionalExpression",
    "InformationTransfer",
    "SpeechAct",
    "RelationalShift",
    "EnvironmentalChange",
];

/// Number of classifiable event kinds.
pub const NUM_EVENT_KINDS: usize = EVENT_KIND_LABELS.len();

// ===========================================================================
// NER labels (token-level, BIO tagging)
// ===========================================================================

/// 15 BIO labels: O + 7 categories x (B + I).
///
/// Order matches Python `schema.py`: O first, then B-X / I-X pairs
/// for each `NerCategory` variant in declaration order.
pub const BIO_LABELS: [&str; 15] = [
    "O",
    "B-CHARACTER",
    "I-CHARACTER",
    "B-OBJECT",
    "I-OBJECT",
    "B-LOCATION",
    "I-LOCATION",
    "B-GESTURE",
    "I-GESTURE",
    "B-SENSORY",
    "I-SENSORY",
    "B-ABSTRACT",
    "I-ABSTRACT",
    "B-COLLECTIVE",
    "I-COLLECTIVE",
];

/// Number of BIO labels (O + 7 categories x 2).
pub const NUM_BIO_LABELS: usize = BIO_LABELS.len();

/// Maximum sequence length for tokenized input.
pub const MAX_SEQ_LENGTH: usize = 128;

// ===========================================================================
// BIO label parsing
// ===========================================================================

/// Parse a BIO label string into its `NerCategory`, if it is a B- or I- tag.
///
/// Returns `None` for "O" (outside) or unrecognized labels.
pub fn bio_label_to_category(label: &str) -> Option<NerCategory> {
    // Strip B- or I- prefix, then match the category suffix.
    let suffix = label
        .strip_prefix("B-")
        .or_else(|| label.strip_prefix("I-"))?;
    match suffix {
        "CHARACTER" => Some(NerCategory::Character),
        "OBJECT" => Some(NerCategory::Object),
        "LOCATION" => Some(NerCategory::Location),
        "GESTURE" => Some(NerCategory::Gesture),
        "SENSORY" => Some(NerCategory::Sensory),
        "ABSTRACT" => Some(NerCategory::Abstract),
        "COLLECTIVE" => Some(NerCategory::Collective),
        _ => None,
    }
}

/// Returns `true` if the label is a B- (begin) tag.
pub fn is_begin_tag(label: &str) -> bool {
    label.starts_with("B-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_counts_match() {
        assert_eq!(NUM_EVENT_KINDS, 8);
        assert_eq!(NUM_BIO_LABELS, 15);
        // 15 = 1 (O) + 7 categories * 2 (B + I)
        assert_eq!(NUM_BIO_LABELS, 1 + 7 * 2);
    }

    #[test]
    fn bio_labels_start_with_o() {
        assert_eq!(BIO_LABELS[0], "O");
    }

    #[test]
    fn bio_labels_alternate_b_i() {
        // After O, labels should alternate B-X, I-X for each category.
        for chunk in BIO_LABELS[1..].chunks(2) {
            assert!(
                chunk[0].starts_with("B-"),
                "expected B- tag, got {}",
                chunk[0]
            );
            assert!(
                chunk[1].starts_with("I-"),
                "expected I- tag, got {}",
                chunk[1]
            );
            // Same category suffix
            let b_suffix = chunk[0].strip_prefix("B-").unwrap();
            let i_suffix = chunk[1].strip_prefix("I-").unwrap();
            assert_eq!(b_suffix, i_suffix);
        }
    }

    #[test]
    fn bio_label_to_category_parses_b_tags() {
        assert_eq!(
            bio_label_to_category("B-CHARACTER"),
            Some(NerCategory::Character)
        );
        assert_eq!(bio_label_to_category("B-OBJECT"), Some(NerCategory::Object));
        assert_eq!(
            bio_label_to_category("B-LOCATION"),
            Some(NerCategory::Location)
        );
        assert_eq!(
            bio_label_to_category("B-GESTURE"),
            Some(NerCategory::Gesture)
        );
        assert_eq!(
            bio_label_to_category("B-SENSORY"),
            Some(NerCategory::Sensory)
        );
        assert_eq!(
            bio_label_to_category("B-ABSTRACT"),
            Some(NerCategory::Abstract)
        );
        assert_eq!(
            bio_label_to_category("B-COLLECTIVE"),
            Some(NerCategory::Collective)
        );
    }

    #[test]
    fn bio_label_to_category_parses_i_tags() {
        assert_eq!(
            bio_label_to_category("I-CHARACTER"),
            Some(NerCategory::Character)
        );
        assert_eq!(
            bio_label_to_category("I-LOCATION"),
            Some(NerCategory::Location)
        );
    }

    #[test]
    fn bio_label_to_category_returns_none_for_o() {
        assert_eq!(bio_label_to_category("O"), None);
    }

    #[test]
    fn bio_label_to_category_returns_none_for_unknown() {
        assert_eq!(bio_label_to_category("B-UNKNOWN"), None);
        assert_eq!(bio_label_to_category("garbage"), None);
        assert_eq!(bio_label_to_category(""), None);
    }

    #[test]
    fn all_bio_labels_round_trip_through_category() {
        // Every B-/I- label in BIO_LABELS should parse to Some(category).
        for label in &BIO_LABELS[1..] {
            assert!(
                bio_label_to_category(label).is_some(),
                "BIO_LABELS entry {label} should parse to a NerCategory"
            );
        }
    }

    #[test]
    fn is_begin_tag_works() {
        assert!(is_begin_tag("B-CHARACTER"));
        assert!(!is_begin_tag("I-CHARACTER"));
        assert!(!is_begin_tag("O"));
    }

    #[test]
    fn event_kind_labels_match_python_schema() {
        // Verify the exact order matches schema.py EVENT_KINDS list.
        assert_eq!(EVENT_KIND_LABELS[0], "StateAssertion");
        assert_eq!(EVENT_KIND_LABELS[1], "ActionOccurrence");
        assert_eq!(EVENT_KIND_LABELS[2], "SpatialChange");
        assert_eq!(EVENT_KIND_LABELS[3], "EmotionalExpression");
        assert_eq!(EVENT_KIND_LABELS[4], "InformationTransfer");
        assert_eq!(EVENT_KIND_LABELS[5], "SpeechAct");
        assert_eq!(EVENT_KIND_LABELS[6], "RelationalShift");
        assert_eq!(EVENT_KIND_LABELS[7], "EnvironmentalChange");
    }
}
