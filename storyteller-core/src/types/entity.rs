//! Entity types — the unified representation for all story elements.
//!
//! See: `docs/technical/entity-model.md`, `docs/ticket-specs/event-system-foundations/entity-reference-model.md`
//!
//! Design decision: Everything is an Entity. Characters, presences, conditions,
//! and props share a single type with component configuration determining
//! their capabilities. Promotion and demotion between tiers is a lifecycle
//! operation, not a type change.
//!
//! The entity reference model handles the identity problem: when prose mentions
//! "the flowers," how does the system know which flowers? Entities are referenced
//! via `EntityRef` (resolved, unresolved, or implicit) and promoted through
//! relational weight accumulated from committed events.

use super::event::{EventId, TurnId};
use super::scene::SceneId;
use uuid::Uuid;

/// Unique identifier for an entity within a story session.
///
/// Uses UUID v7 (time-ordered) for efficient BTree indexing both in-process
/// and in PostgreSQL. Temporal ordering means IDs sort by creation time.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct EntityId(pub Uuid);

impl EntityId {
    /// Create a new time-ordered entity ID (UUID v7).
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for EntityId {
    fn default() -> Self {
        Self::new()
    }
}

/// How an entity came into existence in the story.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EntityOrigin {
    /// Authored by the story designer before play begins.
    Authored,
    /// Promoted from a lower tier during play (e.g., prop → presence).
    Promoted,
    /// Generated procedurally by the engine.
    Generated,
}

/// How an entity persists across scene boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PersistenceMode {
    /// Persists across all scenes (main characters, key locations).
    Permanent,
    /// Persists within the current scene only.
    SceneLocal,
    /// Created on demand, not persisted.
    Ephemeral,
}

// ===========================================================================
// Entity reference model — identity resolution and promotion
// ===========================================================================

/// A reference to an entity that may or may not have been resolved to
/// a tracked `EntityId`.
///
/// Events use `EntityRef` to identify participants. An event can reference
/// an entity before the system has decided whether to track it. Resolution
/// happens when enough relational context accumulates to either match an
/// existing tracked entity or promote a new one.
///
/// See: `docs/ticket-specs/event-system-foundations/entity-reference-model.md`
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum EntityRef {
    /// Resolved — this entity has an `EntityId` and is being tracked.
    Resolved(EntityId),

    /// Unresolved — the entity has been mentioned but not yet matched
    /// to a tracked entity. Carries enough context for later resolution.
    Unresolved {
        /// The mention text — how the entity was referred to.
        /// "the cup," "flowers," "a tall stranger."
        mention: String,
        /// Contextual information that may help resolve the reference.
        context: ReferentialContext,
    },

    /// Implicit — the entity was not mentioned but is implied by context.
    /// "She sat at the table" implies a chair; "the door opened" implies
    /// someone or something opened it.
    Implicit {
        /// What entity is implied.
        implied_entity: String,
        /// What implies it.
        implication_source: String,
    },
}

impl EntityRef {
    /// Returns the `EntityId` if this reference has been resolved.
    pub fn entity_id(&self) -> Option<EntityId> {
        match self {
            EntityRef::Resolved(id) => Some(*id),
            _ => None,
        }
    }

    /// Whether this reference has been resolved to a tracked entity.
    pub fn is_resolved(&self) -> bool {
        matches!(self, EntityRef::Resolved(_))
    }
}

/// Contextual information about an unresolved entity reference.
///
/// This is not a full entity description — it's the minimum context
/// needed for resolution. The context accumulates across events: the
/// first mention of "the cup" might have only spatial context; the
/// second mention might add a possessive context; the third might
/// add descriptive detail.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReferentialContext {
    /// Descriptive details mentioned — "chipped," "old," "silver."
    pub descriptors: Vec<String>,

    /// Spatial context — where this entity was mentioned.
    /// "on the table," "in her hand," "by the stream."
    pub spatial_context: Option<String>,

    /// Possessive context — who this entity belongs to.
    /// "Tanya's phone," "the Wolf's ears."
    /// Boxed to break the indirect recursion (EntityRef → ReferentialContext → EntityRef).
    pub possessor: Option<Box<EntityRef>>,

    /// Prior mentions in the current scene — event IDs where this entity
    /// was previously referenced. Used for anaphoric resolution ("she
    /// picked it up" — "it" refers to the most recent compatible mention).
    pub prior_mentions: Vec<EventId>,

    /// The scene in which this entity was first mentioned.
    pub first_mentioned_scene: SceneId,

    /// The turn in which this entity was first mentioned.
    /// Together with `first_mentioned_scene`, provides full provenance:
    /// which Turn -> which Scene. Enables efficient query scoping when
    /// resolving entity references.
    pub first_mentioned_turn: TurnId,
}

/// The entity's current position in the promotion lifecycle.
///
/// Entities progress upward through event participation and relational
/// weight accumulation. They can also be demoted when relationships
/// decay. The lifecycle extends the existing `PersistenceMode` with
/// pre-tracking tiers.
///
/// Ordering is meaningful: `Unmentioned < Mentioned < Referenced < Tracked < Persistent`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum PromotionTier {
    /// Not mentioned in any event. Does not exist in the system.
    Unmentioned,
    /// Mentioned in prose or events but with no relational implications.
    Mentioned,
    /// Participates in events with relational implications but has not
    /// accumulated enough weight for full tracking.
    Referenced,
    /// Has accumulated sufficient relational weight to earn an `EntityId`
    /// and a node in the relational graph. Equivalent to `PersistenceMode::SceneLocal`.
    Tracked,
    /// Persists across scene boundaries. Requires authored importance or
    /// sufficient cross-scene relational weight. Equivalent to `PersistenceMode::Permanent`.
    Persistent,
}

/// Accumulated relational weight for an entity reference, computed
/// from the events it has participated in.
///
/// Promotion decisions are derived from this weight — when it crosses
/// threshold values, the entity moves up in the promotion lifecycle.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RelationalWeight {
    /// The entity being tracked (may be unresolved).
    pub entity: EntityRef,

    /// Total weight from relational implications across all events.
    pub total_weight: f32,

    /// Number of distinct events this entity participates in.
    pub event_count: u32,

    /// Number of distinct other entities this entity has relationships with.
    pub relationship_count: u32,

    /// Weight from player-initiated events specifically. Player attention
    /// is a strong promotion signal.
    pub player_interaction_weight: f32,
}

/// A lightweight record of an unresolved entity mention in the ledger.
///
/// Stored alongside full `EventAtom` instances but indexed differently.
/// When an entity is promoted, the system queries this index to find
/// all prior mentions and resolve them.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UnresolvedMention {
    /// The event that contains this mention.
    pub event_id: EventId,
    /// The mention text.
    pub mention: String,
    /// The referential context at the time of mention.
    pub context: ReferentialContext,
    /// The scene in which the mention occurred.
    pub scene_id: SceneId,
    /// The turn in which the mention occurred.
    pub turn_id: TurnId,
    /// Index into the event's participants list.
    pub participant_index: usize,
}

/// Scene-level entity budget — how many entities the scene
/// can meaningfully track.
///
/// The budget is not a hard cap — it's a threshold above which
/// the system becomes more conservative about promotion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EntityBudget {
    /// Soft limit on tracked entities in a scene.
    pub soft_limit: u32,
    /// Current count of tracked entities.
    pub current_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_ref_resolved_returns_entity_id() {
        let id = EntityId::new();
        let entity_ref = EntityRef::Resolved(id);
        assert_eq!(entity_ref.entity_id(), Some(id));
        assert!(entity_ref.is_resolved());
    }

    #[test]
    fn entity_ref_unresolved_returns_none() {
        let entity_ref = EntityRef::Unresolved {
            mention: "the cup".to_string(),
            context: ReferentialContext {
                descriptors: vec!["chipped".to_string()],
                spatial_context: Some("on the table".to_string()),
                possessor: None,
                prior_mentions: vec![],
                first_mentioned_scene: SceneId::new(),
                first_mentioned_turn: TurnId::new(),
            },
        };
        assert_eq!(entity_ref.entity_id(), None);
        assert!(!entity_ref.is_resolved());
    }

    #[test]
    fn entity_ref_implicit_construction() {
        let entity_ref = EntityRef::Implicit {
            implied_entity: "chair".to_string(),
            implication_source: "She sat at the table".to_string(),
        };
        assert_eq!(entity_ref.entity_id(), None);
        assert!(!entity_ref.is_resolved());
    }

    #[test]
    fn referential_context_with_boxed_possessor() {
        let possessor = EntityRef::Resolved(EntityId::new());
        let context = ReferentialContext {
            descriptors: vec!["silver".to_string(), "small".to_string()],
            spatial_context: None,
            possessor: Some(Box::new(possessor)),
            prior_mentions: vec![EventId::new()],
            first_mentioned_scene: SceneId::new(),
            first_mentioned_turn: TurnId::new(),
        };
        assert!(context.possessor.is_some());
        assert!(context.possessor.as_ref().unwrap().is_resolved());
    }

    #[test]
    fn promotion_tier_ordering() {
        assert!(PromotionTier::Unmentioned < PromotionTier::Mentioned);
        assert!(PromotionTier::Mentioned < PromotionTier::Referenced);
        assert!(PromotionTier::Referenced < PromotionTier::Tracked);
        assert!(PromotionTier::Tracked < PromotionTier::Persistent);
    }

    #[test]
    fn relational_weight_construction() {
        let weight = RelationalWeight {
            entity: EntityRef::Resolved(EntityId::new()),
            total_weight: 1.5,
            event_count: 3,
            relationship_count: 2,
            player_interaction_weight: 0.5,
        };
        assert_eq!(weight.event_count, 3);
        assert!((weight.total_weight - 1.5).abs() < f32::EPSILON);
    }

    #[test]
    fn unresolved_mention_construction() {
        let mention = UnresolvedMention {
            event_id: EventId::new(),
            mention: "the flowers".to_string(),
            context: ReferentialContext {
                descriptors: vec!["pressed".to_string()],
                spatial_context: Some("in the book".to_string()),
                possessor: None,
                prior_mentions: vec![],
                first_mentioned_scene: SceneId::new(),
                first_mentioned_turn: TurnId::new(),
            },
            scene_id: SceneId::new(),
            turn_id: TurnId::new(),
            participant_index: 1,
        };
        assert_eq!(mention.participant_index, 1);
    }

    #[test]
    fn entity_budget_is_copy() {
        let budget = EntityBudget {
            soft_limit: 15,
            current_count: 8,
        };
        let budget2 = budget;
        assert_eq!(budget.soft_limit, budget2.soft_limit);
        assert_eq!(budget.current_count, budget2.current_count);
    }

    #[test]
    fn entity_ref_serde_roundtrip() {
        let entity_ref = EntityRef::Unresolved {
            mention: "the old cup".to_string(),
            context: ReferentialContext {
                descriptors: vec!["old".to_string(), "chipped".to_string()],
                spatial_context: Some("on the table".to_string()),
                possessor: Some(Box::new(EntityRef::Resolved(EntityId::new()))),
                prior_mentions: vec![EventId::new()],
                first_mentioned_scene: SceneId::new(),
                first_mentioned_turn: TurnId::new(),
            },
        };
        let json = serde_json::to_string(&entity_ref).expect("serialize");
        let deserialized: EntityRef = serde_json::from_str(&json).expect("deserialize");
        assert!(!deserialized.is_resolved());
        if let EntityRef::Unresolved { mention, context } = deserialized {
            assert_eq!(mention, "the old cup");
            assert_eq!(context.descriptors.len(), 2);
            assert!(context.possessor.is_some());
        } else {
            panic!("expected Unresolved variant");
        }
    }
}
