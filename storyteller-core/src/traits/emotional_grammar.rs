//! Emotional grammar trait — pluggable vocabulary of primary emotions.
//!
//! See: `docs/foundation/emotional-model.md`
//!
//! An emotional grammar defines a bounded set of primary emotions with
//! opposition structure, intensity ranges, and composition rules. Different
//! grammars can model different cultural or entity-type emotional vocabularies.
//!
//! The first grammar is `plutchik_western` (Plutchik-derived, 8 primaries).
//! Future grammars: wu xing (TCM five-element), non-human, fey/mythic.

use crate::types::character::EmotionalState;

/// Definition of a primary emotion within a grammar's vocabulary.
///
/// This defines what a primary *is* in the grammar — not a character's
/// current intensity (that's [`EmotionalPrimary`](crate::types::character::EmotionalPrimary)).
#[derive(Debug, Clone)]
pub struct PrimaryDef {
    /// Identifier used in [`EmotionalPrimary::primary_id`](crate::types::character::EmotionalPrimary).
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// The opposite primary, if this grammar has opposition structure.
    pub opposite_id: Option<String>,
    /// Label at the low end of the intensity gradient. E.g. "serenity" for joy.
    pub low_intensity_label: String,
    /// Label at the high end. E.g. "ecstasy" for joy.
    pub high_intensity_label: String,
}

/// A pluggable emotional vocabulary with primaries, oppositions, and validation.
///
/// Implementations define the grammar's vocabulary and can validate that
/// a character's [`EmotionalState`] conforms to the grammar's rules.
///
/// Grammars are resolved by ID at runtime — [`CharacterSheet`](crate::types::character::CharacterSheet)
/// stores `grammar_id: String`, and the agent architecture looks up the
/// corresponding `dyn EmotionalGrammar` from a registry.
pub trait EmotionalGrammar: Send + Sync + std::fmt::Debug {
    /// Unique grammar identifier. E.g. `"plutchik_western"`.
    fn id(&self) -> &str;

    /// Human-readable name. E.g. `"Plutchik-derived Western"`.
    fn name(&self) -> &str;

    /// The set of primary emotions in this grammar.
    fn primaries(&self) -> &[PrimaryDef];

    /// Valid intensity range for primaries. Typically `(0.0, 1.0)`.
    fn intensity_range(&self) -> (f32, f32);

    /// Validate that an [`EmotionalState`] conforms to this grammar.
    ///
    /// Returns `Ok(())` if valid, or a list of validation errors.
    /// Checks: all primaries present, intensities in range, grammar ID matches.
    fn validate_state(&self, state: &EmotionalState) -> Result<(), Vec<String>>;
}
