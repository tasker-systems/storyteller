//! Entity reference resolution — matching unresolved mentions to tracked entities.
//!
//! When the system encounters an unresolved entity reference (e.g., "the cup"),
//! it tries to resolve it against known tracked entities using four strategies
//! in order: possessive, spatial, anaphoric, descriptive.
//!
//! Resolution is conservative: ambiguous results (multiple candidates with equal
//! confidence) return `None` rather than guessing.

use crate::types::entity::{EntityId, EntityRef};
use crate::types::scene::SceneId;

use super::weight::normalize_mention;

/// A tracked entity — has an `EntityId` and enough context for resolution.
#[derive(Debug, Clone)]
pub struct TrackedEntity {
    /// The entity's unique identifier.
    pub id: EntityId,
    /// Canonical name or mention text.
    pub canonical_name: String,
    /// Known descriptors (accumulated from prior events).
    pub descriptors: Vec<String>,
    /// Current scene if present.
    pub current_scene: Option<SceneId>,
    /// Entity that possesses this one (if any).
    pub possessor: Option<EntityId>,
}

/// Scene-level context for resolution.
#[derive(Debug, Clone)]
pub struct SceneResolutionContext {
    /// Entities present in the current scene.
    pub scene_cast: Vec<TrackedEntity>,
    /// The current scene ID.
    pub scene_id: SceneId,
}

/// Attempt to resolve an unresolved entity reference against known entities.
///
/// Tries four strategies in order: possessive, spatial, anaphoric, descriptive.
/// Returns `None` if no strategy produces a confident match or if the result
/// is ambiguous (multiple candidates with equal confidence).
///
/// For `Resolved` refs, returns the entity ID directly. For `Implicit` refs,
/// returns `None` (implicit entities are not resolvable via this path).
pub fn resolve_entity_ref(
    entity_ref: &EntityRef,
    context: &SceneResolutionContext,
) -> Option<EntityId> {
    match entity_ref {
        EntityRef::Resolved(id) => Some(*id),
        EntityRef::Implicit { .. } => None,
        EntityRef::Unresolved {
            mention,
            context: ref_context,
        } => {
            // Try possessive resolution
            if let Some(possessor_ref) = &ref_context.possessor {
                if let Some(result) = resolve_possessive(possessor_ref, context) {
                    return Some(result);
                }
            }

            // Try spatial resolution
            if let Some(spatial) = &ref_context.spatial_context {
                if let Some(result) = resolve_spatial(mention, spatial, context) {
                    return Some(result);
                }
            }

            // Try anaphoric resolution (prior mentions)
            if !ref_context.prior_mentions.is_empty() {
                if let Some(result) = resolve_anaphoric(ref_context, context) {
                    return Some(result);
                }
            }

            // Try descriptive resolution
            resolve_descriptive(mention, &ref_context.descriptors, context)
        }
    }
}

/// Possessive resolution: find an entity owned by the possessor.
///
/// If the possessor is resolved, look for tracked entities in the scene
/// that are possessed by that entity.
fn resolve_possessive(
    possessor_ref: &EntityRef,
    context: &SceneResolutionContext,
) -> Option<EntityId> {
    let possessor_id = possessor_ref.entity_id()?;

    let candidates: Vec<&TrackedEntity> = context
        .scene_cast
        .iter()
        .filter(|e| e.possessor == Some(possessor_id))
        .collect();

    if candidates.len() == 1 {
        Some(candidates[0].id)
    } else {
        None // Ambiguous or no match
    }
}

/// Spatial resolution: narrow candidates by mention text among scene entities.
///
/// Spatial context is used as a filter — we match entities that are in the
/// scene and whose canonical name matches the mention.
fn resolve_spatial(
    mention: &str,
    _spatial_context: &str,
    context: &SceneResolutionContext,
) -> Option<EntityId> {
    let normalized = normalize_mention(mention);
    let candidates: Vec<&TrackedEntity> = context
        .scene_cast
        .iter()
        .filter(|e| normalize_mention(&e.canonical_name) == normalized)
        .collect();

    if candidates.len() == 1 {
        Some(candidates[0].id)
    } else {
        None
    }
}

/// Anaphoric resolution: use prior mentions to find previously resolved entities.
///
/// If any prior mention's event involved a resolved entity with a matching
/// canonical name, use that entity. This handles "she picked it up" where
/// "it" refers to the most recently mentioned compatible entity.
fn resolve_anaphoric(
    ref_context: &crate::types::entity::ReferentialContext,
    context: &SceneResolutionContext,
) -> Option<EntityId> {
    // For anaphoric resolution, we check if any entity in the scene cast
    // has a prior_mention event ID that matches one in the reference context.
    // This is a simplified approach — the full implementation would query
    // the event ledger to find what entity was mentioned in that event.
    //
    // For now, we use the last prior mention's event ID and look for a
    // scene cast entity that was involved in that event. Since we don't
    // have event data here, we return None — anaphoric resolution requires
    // ledger access which is Phase D work.
    let _ = ref_context;
    let _ = context;
    None
}

/// Descriptive resolution: match descriptors against tracked entity descriptors.
///
/// Counts descriptor overlap between the reference and each tracked entity.
/// Returns a match only if exactly one entity has the highest overlap score,
/// and that score is greater than zero.
fn resolve_descriptive(
    mention: &str,
    descriptors: &[String],
    context: &SceneResolutionContext,
) -> Option<EntityId> {
    if descriptors.is_empty() && mention.is_empty() {
        return None;
    }

    let normalized_mention = normalize_mention(mention);

    // First, try matching by canonical name
    let name_matches: Vec<&TrackedEntity> = context
        .scene_cast
        .iter()
        .filter(|e| normalize_mention(&e.canonical_name) == normalized_mention)
        .collect();

    if name_matches.len() == 1 {
        return Some(name_matches[0].id);
    }

    // If no unique name match, try descriptor overlap
    if descriptors.is_empty() {
        return None;
    }

    let normalized_descriptors: Vec<String> =
        descriptors.iter().map(|d| d.to_lowercase()).collect();

    let mut best_score = 0;
    let mut best_id = None;
    let mut ambiguous = false;

    for entity in &context.scene_cast {
        let score = entity
            .descriptors
            .iter()
            .filter(|d| normalized_descriptors.contains(&d.to_lowercase()))
            .count();

        if score > best_score {
            best_score = score;
            best_id = Some(entity.id);
            ambiguous = false;
        } else if score == best_score && score > 0 {
            ambiguous = true;
        }
    }

    if ambiguous || best_score == 0 {
        None
    } else {
        best_id
    }
}

#[cfg(test)]
mod tests {
    use crate::types::entity::ReferentialContext;
    use crate::types::event::{EventId, TurnId};

    use super::*;

    fn make_scene_context(entities: Vec<TrackedEntity>) -> SceneResolutionContext {
        SceneResolutionContext {
            scene_cast: entities,
            scene_id: SceneId::new(),
        }
    }

    fn make_unresolved(
        mention: &str,
        descriptors: Vec<&str>,
        possessor: Option<EntityRef>,
        spatial: Option<&str>,
        prior_mentions: Vec<EventId>,
    ) -> EntityRef {
        EntityRef::Unresolved {
            mention: mention.to_string(),
            context: ReferentialContext {
                descriptors: descriptors.iter().map(|s| s.to_string()).collect(),
                spatial_context: spatial.map(|s| s.to_string()),
                possessor: possessor.map(Box::new),
                prior_mentions,
                first_mentioned_scene: SceneId::new(),
                first_mentioned_turn: TurnId::new(),
            },
        }
    }

    // -----------------------------------------------------------------------
    // Resolved passthrough
    // -----------------------------------------------------------------------

    #[test]
    fn resolved_ref_passes_through() {
        let id = EntityId::new();
        let entity_ref = EntityRef::Resolved(id);
        let context = make_scene_context(vec![]);
        assert_eq!(resolve_entity_ref(&entity_ref, &context), Some(id));
    }

    // -----------------------------------------------------------------------
    // Implicit returns None
    // -----------------------------------------------------------------------

    #[test]
    fn implicit_ref_returns_none() {
        let entity_ref = EntityRef::Implicit {
            implied_entity: "chair".to_string(),
            implication_source: "sat down".to_string(),
        };
        let context = make_scene_context(vec![]);
        assert_eq!(resolve_entity_ref(&entity_ref, &context), None);
    }

    // -----------------------------------------------------------------------
    // Possessive resolution
    // -----------------------------------------------------------------------

    #[test]
    fn possessive_resolution_finds_owned_entity() {
        let owner_id = EntityId::new();
        let cup_id = EntityId::new();

        let context = make_scene_context(vec![
            TrackedEntity {
                id: owner_id,
                canonical_name: "Tanya".to_string(),
                descriptors: vec![],
                current_scene: None,
                possessor: None,
            },
            TrackedEntity {
                id: cup_id,
                canonical_name: "cup".to_string(),
                descriptors: vec!["chipped".to_string()],
                current_scene: None,
                possessor: Some(owner_id),
            },
        ]);

        let entity_ref = make_unresolved(
            "cup",
            vec!["chipped"],
            Some(EntityRef::Resolved(owner_id)),
            None,
            vec![],
        );
        assert_eq!(resolve_entity_ref(&entity_ref, &context), Some(cup_id));
    }

    // -----------------------------------------------------------------------
    // Spatial resolution
    // -----------------------------------------------------------------------

    #[test]
    fn spatial_resolution_narrows_to_scene() {
        let stone_id = EntityId::new();

        let context = make_scene_context(vec![TrackedEntity {
            id: stone_id,
            canonical_name: "stone".to_string(),
            descriptors: vec!["smooth".to_string()],
            current_scene: None,
            possessor: None,
        }]);

        let entity_ref = make_unresolved("the stone", vec![], None, Some("by the stream"), vec![]);
        assert_eq!(resolve_entity_ref(&entity_ref, &context), Some(stone_id));
    }

    // -----------------------------------------------------------------------
    // Descriptive resolution
    // -----------------------------------------------------------------------

    #[test]
    fn descriptive_resolution_matches_by_descriptors() {
        let cup_id = EntityId::new();
        let bowl_id = EntityId::new();

        let context = make_scene_context(vec![
            TrackedEntity {
                id: cup_id,
                canonical_name: "vessel".to_string(),
                descriptors: vec!["chipped".to_string(), "old".to_string()],
                current_scene: None,
                possessor: None,
            },
            TrackedEntity {
                id: bowl_id,
                canonical_name: "container".to_string(),
                descriptors: vec!["wooden".to_string(), "large".to_string()],
                current_scene: None,
                possessor: None,
            },
        ]);

        let entity_ref = make_unresolved("thing", vec!["chipped", "old"], None, None, vec![]);
        assert_eq!(resolve_entity_ref(&entity_ref, &context), Some(cup_id));
    }

    #[test]
    fn ambiguous_descriptors_return_none() {
        let cup_id = EntityId::new();
        let bowl_id = EntityId::new();

        let context = make_scene_context(vec![
            TrackedEntity {
                id: cup_id,
                canonical_name: "cup".to_string(),
                descriptors: vec!["old".to_string()],
                current_scene: None,
                possessor: None,
            },
            TrackedEntity {
                id: bowl_id,
                canonical_name: "bowl".to_string(),
                descriptors: vec!["old".to_string()],
                current_scene: None,
                possessor: None,
            },
        ]);

        // "old" matches both — ambiguous
        let entity_ref = make_unresolved("thing", vec!["old"], None, None, vec![]);
        assert_eq!(resolve_entity_ref(&entity_ref, &context), None);
    }

    // -----------------------------------------------------------------------
    // Empty scene cast
    // -----------------------------------------------------------------------

    #[test]
    fn empty_scene_cast_returns_none() {
        let context = make_scene_context(vec![]);
        let entity_ref = make_unresolved("the cup", vec!["chipped"], None, None, vec![]);
        assert_eq!(resolve_entity_ref(&entity_ref, &context), None);
    }

    // -----------------------------------------------------------------------
    // Strategy ordering
    // -----------------------------------------------------------------------

    #[test]
    fn first_unambiguous_strategy_wins() {
        let owner_id = EntityId::new();
        let cup_id = EntityId::new();
        let other_cup_id = EntityId::new();

        // Two cups in the scene, but only one owned by owner
        let context = make_scene_context(vec![
            TrackedEntity {
                id: owner_id,
                canonical_name: "Tanya".to_string(),
                descriptors: vec![],
                current_scene: None,
                possessor: None,
            },
            TrackedEntity {
                id: cup_id,
                canonical_name: "cup".to_string(),
                descriptors: vec!["chipped".to_string()],
                current_scene: None,
                possessor: Some(owner_id),
            },
            TrackedEntity {
                id: other_cup_id,
                canonical_name: "cup".to_string(),
                descriptors: vec!["silver".to_string()],
                current_scene: None,
                possessor: None,
            },
        ]);

        // Possessive resolution should win even though descriptive would be ambiguous
        let entity_ref = make_unresolved(
            "cup",
            vec![],
            Some(EntityRef::Resolved(owner_id)),
            None,
            vec![],
        );
        assert_eq!(resolve_entity_ref(&entity_ref, &context), Some(cup_id));
    }

    // -----------------------------------------------------------------------
    // Name-based resolution
    // -----------------------------------------------------------------------

    #[test]
    fn descriptive_matches_by_canonical_name() {
        let sarah_id = EntityId::new();

        let context = make_scene_context(vec![TrackedEntity {
            id: sarah_id,
            canonical_name: "Sarah".to_string(),
            descriptors: vec![],
            current_scene: None,
            possessor: None,
        }]);

        let entity_ref = make_unresolved("Sarah", vec![], None, None, vec![]);
        assert_eq!(resolve_entity_ref(&entity_ref, &context), Some(sarah_id));
    }

    #[test]
    fn no_matching_strategy_returns_none() {
        let stone_id = EntityId::new();

        let context = make_scene_context(vec![TrackedEntity {
            id: stone_id,
            canonical_name: "stone".to_string(),
            descriptors: vec!["smooth".to_string()],
            current_scene: None,
            possessor: None,
        }]);

        // Mention "flower" doesn't match "stone" by name or descriptors
        let entity_ref = make_unresolved("flower", vec!["blue"], None, None, vec![]);
        assert_eq!(resolve_entity_ref(&entity_ref, &context), None);
    }
}
