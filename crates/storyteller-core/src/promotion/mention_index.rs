//! Mention index — in-memory index of unresolved entity mentions for retroactive promotion.
//!
//! When an entity accumulates enough relational weight to be promoted (gaining an
//! `EntityId`), the system walks this index to find all prior mentions that should
//! be resolved to the new entity. Resolution records are created without mutating
//! the original `UnresolvedMention` — preserving ledger immutability.

use std::collections::BTreeMap;

use crate::types::entity::{EntityId, UnresolvedMention};
use crate::types::event::{EventId, TurnId};

use super::weight::normalize_mention;

/// In-memory index of unresolved entity mentions.
///
/// When an entity accumulates enough relational weight to be promoted
/// (gaining an `EntityId`), the system walks this index to find all prior
/// mentions that should be resolved to the new entity.
///
/// Keyed by normalized mention text for efficient lookup.
#[derive(Debug, Clone, Default)]
pub struct MentionIndex {
    entries: BTreeMap<String, Vec<UnresolvedMention>>,
}

impl MentionIndex {
    /// Create an empty mention index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert an unresolved mention into the index.
    ///
    /// The mention is keyed by its normalized text (lowercase, stripped articles).
    pub fn insert(&mut self, mention: UnresolvedMention) {
        let key = normalize_mention(&mention.mention);
        self.entries.entry(key).or_default().push(mention);
    }

    /// Look up mentions by normalized text.
    ///
    /// Returns a slice of all mentions matching the normalized key.
    pub fn lookup(&self, mention: &str) -> &[UnresolvedMention] {
        let key = normalize_mention(mention);
        self.entries.get(&key).map_or(&[], |v| v.as_slice())
    }

    /// Remove and return all mentions matching the normalized text.
    pub fn remove(&mut self, mention: &str) -> Vec<UnresolvedMention> {
        let key = normalize_mention(mention);
        self.entries.remove(&key).unwrap_or_default()
    }

    /// Total number of mentions across all keys.
    pub fn len(&self) -> usize {
        self.entries.values().map(|v| v.len()).sum()
    }

    /// Whether the index contains no mentions.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty() || self.entries.values().all(|v| v.is_empty())
    }
}

/// A record that an unresolved mention has been resolved to a tracked entity.
///
/// Created during retroactive promotion. Does not mutate the original
/// `UnresolvedMention` — preserves ledger immutability.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResolutionRecord {
    /// The event that contained the original unresolved mention.
    pub event_id: EventId,
    /// The participant index within the event's participant list.
    pub participant_index: usize,
    /// The original mention text.
    pub original_mention: String,
    /// The entity ID this mention was resolved to.
    pub resolved_to: EntityId,
    /// The turn in which the mention occurred.
    pub mention_turn: TurnId,
}

/// Retroactively resolve prior mentions of a newly promoted entity.
///
/// When an entity accumulates enough relational weight to earn an `EntityId`,
/// this function finds all prior unresolved mentions that match and creates
/// resolution records. The original mentions in the ledger are not mutated.
///
/// Returns the resolution records (to be persisted) and removes the resolved
/// mentions from the index.
pub fn retroactively_promote(
    entity_id: EntityId,
    mention_text: &str,
    index: &mut MentionIndex,
) -> Vec<ResolutionRecord> {
    let mentions = index.remove(mention_text);

    mentions
        .into_iter()
        .map(|m| ResolutionRecord {
            event_id: m.event_id,
            participant_index: m.participant_index,
            original_mention: m.mention,
            resolved_to: entity_id,
            mention_turn: m.turn_id,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::types::entity::ReferentialContext;
    use crate::types::scene::SceneId;

    use super::*;

    fn make_mention(
        mention: &str,
        event_id: EventId,
        participant_index: usize,
    ) -> UnresolvedMention {
        let turn_id = TurnId::new();
        UnresolvedMention {
            event_id,
            mention: mention.to_string(),
            context: ReferentialContext {
                descriptors: vec![],
                spatial_context: None,
                possessor: None,
                prior_mentions: vec![],
                first_mentioned_scene: SceneId::new(),
                first_mentioned_turn: turn_id,
            },
            scene_id: SceneId::new(),
            turn_id,
            participant_index,
        }
    }

    // -----------------------------------------------------------------------
    // MentionIndex basics
    // -----------------------------------------------------------------------

    #[test]
    fn insert_and_lookup() {
        let mut index = MentionIndex::new();
        let event_id = EventId::new();
        index.insert(make_mention("the cup", event_id, 0));

        let results = index.lookup("cup");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].event_id, event_id);
    }

    #[test]
    fn multiple_mentions_same_text() {
        let mut index = MentionIndex::new();
        let e1 = EventId::new();
        let e2 = EventId::new();
        index.insert(make_mention("cup", e1, 0));
        index.insert(make_mention("the Cup", e2, 1));

        let results = index.lookup("cup");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn remove_clears_entries() {
        let mut index = MentionIndex::new();
        index.insert(make_mention("cup", EventId::new(), 0));
        index.insert(make_mention("the cup", EventId::new(), 1));

        let removed = index.remove("cup");
        assert_eq!(removed.len(), 2);
        assert!(index.lookup("cup").is_empty());
    }

    #[test]
    fn normalize_key_matches() {
        let mut index = MentionIndex::new();
        index.insert(make_mention("The Cup", EventId::new(), 0));

        // Lookup with different casing and article
        assert_eq!(index.lookup("cup").len(), 1);
        assert_eq!(index.lookup("the cup").len(), 1);
        assert_eq!(index.lookup("The Cup").len(), 1);
    }

    #[test]
    fn len_and_is_empty() {
        let mut index = MentionIndex::new();
        assert!(index.is_empty());
        assert_eq!(index.len(), 0);

        index.insert(make_mention("cup", EventId::new(), 0));
        index.insert(make_mention("stone", EventId::new(), 1));
        assert!(!index.is_empty());
        assert_eq!(index.len(), 2);
    }

    #[test]
    fn default_is_empty() {
        let index = MentionIndex::default();
        assert!(index.is_empty());
    }

    // -----------------------------------------------------------------------
    // Retroactive promotion
    // -----------------------------------------------------------------------

    #[test]
    fn retroactive_promotion_creates_records() {
        let mut index = MentionIndex::new();
        let e1 = EventId::new();
        let e2 = EventId::new();
        index.insert(make_mention("the cup", e1, 0));
        index.insert(make_mention("cup", e2, 2));

        let entity_id = EntityId::new();
        let records = retroactively_promote(entity_id, "cup", &mut index);

        assert_eq!(records.len(), 2);
        assert!(records.iter().all(|r| r.resolved_to == entity_id));
        assert!(index.lookup("cup").is_empty());
    }

    #[test]
    fn retroactive_promotion_removes_from_index() {
        let mut index = MentionIndex::new();
        index.insert(make_mention("cup", EventId::new(), 0));
        index.insert(make_mention("stone", EventId::new(), 1));

        let entity_id = EntityId::new();
        retroactively_promote(entity_id, "cup", &mut index);

        assert!(index.lookup("cup").is_empty());
        assert_eq!(index.lookup("stone").len(), 1);
    }

    #[test]
    fn retroactive_promotion_nonexistent_returns_empty() {
        let mut index = MentionIndex::new();
        let records = retroactively_promote(EntityId::new(), "cup", &mut index);
        assert!(records.is_empty());
    }

    #[test]
    fn resolution_record_preserves_original_mention() {
        let mut index = MentionIndex::new();
        let event_id = EventId::new();
        index.insert(make_mention("The Old Cup", event_id, 3));

        let entity_id = EntityId::new();
        let records = retroactively_promote(entity_id, "old cup", &mut index);

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].original_mention, "The Old Cup");
        assert_eq!(records[0].event_id, event_id);
        assert_eq!(records[0].participant_index, 3);
        assert_eq!(records[0].resolved_to, entity_id);
    }
}
