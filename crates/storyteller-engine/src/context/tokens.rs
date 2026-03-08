//! Token estimation heuristics.
//!
//! For the prototype, we use a rough word-count heuristic rather than a
//! real tokenizer. This is intentionally naive — production will use
//! tiktoken-rs or the model's actual tokenizer. The heuristic is:
//! `word_count / 0.75` (English text averages ~0.75 words per token).

/// Estimate the token count for a string using word-count heuristic.
///
/// This is a rough approximation: `ceil(word_count / 0.75)`.
/// Good enough for budget tracking in the prototype; production
/// should use a real tokenizer.
pub fn estimate_tokens(text: &str) -> u32 {
    let word_count = text.split_whitespace().count();
    // ~0.75 words per token → tokens = words / 0.75 = words * 4 / 3
    (word_count * 4).div_ceil(3) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_string_is_zero_tokens() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn single_word() {
        // 1 word → ceil(1/0.75) = 2 tokens
        assert!(estimate_tokens("hello") >= 1);
    }

    #[test]
    fn typical_sentence() {
        let text = "The Wolf's ear flicks — a small involuntary motion.";
        let tokens = estimate_tokens(text);
        // 9 words → ~12 tokens, reasonable range
        assert!((8..=20).contains(&tokens));
    }

    #[test]
    fn longer_passage() {
        let text = "Literary fiction, present tense, close third person. \
            Your reference is Marilynne Robinson, not Dungeons and Dragons. \
            Compression: every sentence earns its place.";
        let tokens = estimate_tokens(text);
        // ~25 words → ~33 tokens
        assert!((20..=50).contains(&tokens));
    }
}
