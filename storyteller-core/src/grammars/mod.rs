//! Emotional grammar implementations.
//!
//! Each grammar defines a bounded vocabulary of primary emotions with
//! opposition structure and intensity gradients. Grammars are resolved
//! by ID at runtime.
//!
//! See: `docs/foundation/emotional-model.md`

pub mod plutchik_western;

use crate::traits::EmotionalGrammar;

pub use plutchik_western::PlutchikWestern;

/// Look up an emotional grammar by its ID.
///
/// Returns `None` for unrecognized IDs. In the future, this could be
/// replaced by a registry that supports runtime-registered grammars.
pub fn lookup(grammar_id: &str) -> Option<Box<dyn EmotionalGrammar>> {
    match grammar_id {
        PlutchikWestern::GRAMMAR_ID => Some(Box::new(PlutchikWestern::new())),
        _ => None,
    }
}
