//! Capability lexicon — pre-seeded natural language mappings for authored capabilities.
//!
//! See: `docs/plans/2026-03-09-event-classification-and-action-arbitration-design.md`
//!
//! At story authoring time, each authored capability (e.g., "swordsmanship")
//! is expanded into synonyms, action verbs, implied objects, and idiomatic
//! phrases. At runtime, capability matching is fast string/token lookup
//! against these pre-computed sets.

use std::collections::BTreeMap;

/// A pre-seeded mapping from authored capability to natural language terms.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LexiconEntry {
    /// The authored capability name.
    pub capability: String,
    /// Direct synonyms: "swordsmanship" -> ["fencing", "blade work"].
    pub synonyms: Vec<String>,
    /// Action verbs: "swordsmanship" -> ["slash", "parry", "thrust"].
    pub action_verbs: Vec<String>,
    /// Implied objects: "swordsmanship" -> ["rapier", "sword", "blade"].
    pub implied_objects: Vec<String>,
    /// Multi-hop phrases: "swordsmanship" -> ["crossed swords", "steel rang"].
    pub idiomatic_phrases: Vec<String>,
}

impl LexiconEntry {
    /// Check if a single token matches any term in this entry.
    pub fn matches_token(&self, token: &str) -> bool {
        let lower = token.to_lowercase();
        self.synonyms.iter().any(|s| s.to_lowercase() == lower)
            || self.action_verbs.iter().any(|v| v.to_lowercase() == lower)
            || self
                .implied_objects
                .iter()
                .any(|o| o.to_lowercase() == lower)
    }

    /// Check if any term from this entry appears in the given text.
    pub fn matches_text(&self, text: &str) -> bool {
        let lower = text.to_lowercase();
        self.synonyms
            .iter()
            .any(|s| lower.contains(&s.to_lowercase()))
            || self
                .action_verbs
                .iter()
                .any(|v| lower.contains(&v.to_lowercase()))
            || self
                .implied_objects
                .iter()
                .any(|o| lower.contains(&o.to_lowercase()))
            || self
                .idiomatic_phrases
                .iter()
                .any(|p| lower.contains(&p.to_lowercase()))
    }
}

/// Collection of capability lexicon entries for a story's game design system.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CapabilityLexicon {
    /// Maps capability name to its lexicon entry.
    pub entries: BTreeMap<String, LexiconEntry>,
}

impl CapabilityLexicon {
    /// Create an empty lexicon.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a lexicon entry.
    pub fn add(&mut self, entry: LexiconEntry) {
        self.entries.insert(entry.capability.clone(), entry);
    }

    /// Find all capabilities that match tokens in the given text.
    pub fn match_text(&self, text: &str) -> Vec<String> {
        self.entries
            .values()
            .filter(|entry| entry.matches_text(text))
            .map(|entry| entry.capability.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexicon_entry_matches_synonym() {
        let entry = LexiconEntry {
            capability: "swordsmanship".to_string(),
            synonyms: vec!["fencing".to_string(), "blade work".to_string()],
            action_verbs: vec!["slash".to_string(), "parry".to_string()],
            implied_objects: vec!["rapier".to_string(), "sword".to_string()],
            idiomatic_phrases: vec!["crossed swords".to_string()],
        };
        assert!(entry.matches_token("fencing"));
        assert!(entry.matches_token("slash"));
        assert!(entry.matches_token("rapier"));
        assert!(!entry.matches_token("cooking"));
    }

    #[test]
    fn lexicon_matches_against_text() {
        let mut lexicon = CapabilityLexicon::new();
        lexicon.add(LexiconEntry {
            capability: "swordsmanship".to_string(),
            synonyms: vec![],
            action_verbs: vec!["slash".to_string()],
            implied_objects: vec!["rapier".to_string()],
            idiomatic_phrases: vec![],
        });
        let matches = lexicon.match_text("I dive for the rapier and slash at his hand");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0], "swordsmanship");
    }

    #[test]
    fn lexicon_returns_empty_for_no_match() {
        let lexicon = CapabilityLexicon::new();
        let matches = lexicon.match_text("I walk through the meadow");
        assert!(matches.is_empty());
    }

    #[test]
    fn lexicon_serializes() {
        let mut lexicon = CapabilityLexicon::new();
        lexicon.add(LexiconEntry {
            capability: "archery".to_string(),
            synonyms: vec!["bowmanship".to_string()],
            action_verbs: vec!["shoot".to_string(), "aim".to_string()],
            implied_objects: vec!["bow".to_string(), "arrow".to_string()],
            idiomatic_phrases: vec![],
        });
        let json = serde_json::to_string(&lexicon).unwrap();
        let roundtrip: CapabilityLexicon = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip.entries.len(), 1);
    }
}
