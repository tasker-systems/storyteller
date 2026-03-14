//! Likeness pass — scores and selects lexicon fragments for active goals.
//!
//! Three steps: dimensional match, tensor affinity scoring, diversity sampling.
//!
//! See: `docs/plans/2026-03-11-scene-goals-and-character-intentions-design.md`

use rand::seq::SliceRandom;
use rand::Rng;

use super::descriptors::{Goal, LexiconEntry};
use super::goals::{CharacterGoal, FragmentRegister, GoalFragment, SceneGoal};

/// Scene context for the likeness pass.
#[derive(Debug)]
pub struct LikenessContext<'a> {
    pub genre_id: &'a str,
    pub profile_id: &'a str,
    pub archetype_ids: Vec<&'a str>,
    pub dynamic_ids: Vec<&'a str>,
}

/// Score a single fragment against the scene context.
///
/// Returns a score in [0.0, 1.0] based on dimensional match:
/// - Each matching dimension (archetype, profile, dynamic) adds to the score
/// - `null` dimensions are wildcards and contribute a base score
fn score_fragment(entry: &LexiconEntry, ctx: &LikenessContext<'_>) -> f64 {
    let mut score = 0.0;
    let mut max_score = 0.0;

    // Archetype match
    max_score += 1.0;
    match &entry.dimensional_context.archetypes {
        None => score += 0.5, // wildcard: partial credit
        Some(archetypes) => {
            if ctx
                .archetype_ids
                .iter()
                .any(|a| archetypes.iter().any(|ea| ea == a))
            {
                score += 1.0;
            }
        }
    }

    // Profile match
    max_score += 1.0;
    match &entry.dimensional_context.profiles {
        None => score += 0.5,
        Some(profiles) => {
            if profiles.iter().any(|p| p == ctx.profile_id) {
                score += 1.0;
            }
        }
    }

    // Dynamic match
    max_score += 1.0;
    match &entry.dimensional_context.dynamics {
        None => score += 0.5,
        Some(dynamics) => {
            if ctx
                .dynamic_ids
                .iter()
                .any(|d| dynamics.iter().any(|ed| ed == d))
            {
                score += 1.0;
            }
        }
    }

    if max_score > 0.0 {
        score / max_score
    } else {
        0.0
    }
}

fn parse_register(s: &str) -> FragmentRegister {
    match s {
        "atmospheric" => FragmentRegister::Atmospheric,
        "transitional" => FragmentRegister::Transitional,
        _ => FragmentRegister::CharacterSignal,
    }
}

/// Select fragments for a goal using the likeness pass.
///
/// Scores all lexicon entries, then samples with diversity:
/// - Up to 3 character_signal fragments
/// - Up to 2 atmospheric fragments
/// - Up to 1 transitional fragment
pub fn select_fragments<R: Rng>(
    goal: &Goal,
    ctx: &LikenessContext<'_>,
    rng: &mut R,
) -> Vec<GoalFragment> {
    if goal.lexicon.is_empty() {
        return Vec::new();
    }

    let mut scored: Vec<(&LexiconEntry, f64)> = goal
        .lexicon
        .iter()
        .map(|e| (e, score_fragment(e, ctx)))
        .filter(|(_, s)| *s > 0.0)
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut character_signals = Vec::new();
    let mut atmospherics = Vec::new();
    let mut transitionals = Vec::new();

    // Weighted shuffle: take top candidates with some randomness
    // Shuffle within score tiers (±0.1) for diversity
    scored.shuffle(rng);
    scored.sort_by(|a, b| {
        let a_tier = (a.1 * 10.0).round() as i32;
        let b_tier = (b.1 * 10.0).round() as i32;
        b_tier.cmp(&a_tier)
    });

    for (entry, _score) in &scored {
        let register = parse_register(&entry.register);
        let fragment = GoalFragment {
            text: entry.fragment.clone(),
            register: register.clone(),
        };

        match register {
            FragmentRegister::CharacterSignal if character_signals.len() < 3 => {
                character_signals.push(fragment);
            }
            FragmentRegister::Atmospheric if atmospherics.len() < 2 => {
                atmospherics.push(fragment);
            }
            FragmentRegister::Transitional if transitionals.is_empty() => {
                transitionals.push(fragment);
            }
            _ => {}
        }
    }

    let mut result = Vec::new();
    result.extend(character_signals);
    result.extend(atmospherics);
    result.extend(transitionals);
    result
}

/// Populate fragments on scene goals.
pub fn populate_scene_goal_fragments<R: Rng>(
    scene_goals: &mut [SceneGoal],
    goal_defs: &[Goal],
    ctx: &LikenessContext<'_>,
    rng: &mut R,
) {
    for sg in scene_goals.iter_mut() {
        if let Some(def) = goal_defs.iter().find(|g| g.id == sg.goal_id) {
            sg.fragments = select_fragments(def, ctx, rng);
        }
    }
}

/// Populate fragments on character goals.
pub fn populate_character_goal_fragments<R: Rng>(
    character_goals: &mut [CharacterGoal],
    goal_defs: &[Goal],
    ctx: &LikenessContext<'_>,
    rng: &mut R,
) {
    for cg in character_goals.iter_mut() {
        if let Some(def) = goal_defs.iter().find(|g| g.id == cg.goal_id) {
            cg.fragments = select_fragments(def, ctx, rng);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::descriptors::DimensionalContext;
    use super::*;

    fn make_entry(
        fragment: &str,
        register: &str,
        archetypes: Option<Vec<&str>>,
        profiles: Option<Vec<&str>>,
    ) -> LexiconEntry {
        LexiconEntry {
            fragment: fragment.to_string(),
            register: register.to_string(),
            dimensional_context: DimensionalContext {
                archetypes: archetypes.map(|v| v.into_iter().map(String::from).collect()),
                profiles: profiles.map(|v| v.into_iter().map(String::from).collect()),
                dynamics: None,
                valence: Vec::new(),
            },
        }
    }

    fn test_ctx() -> LikenessContext<'static> {
        LikenessContext {
            genre_id: "cozy_ghost_story",
            profile_id: "quiet_reunion",
            archetype_ids: vec!["stoic_survivor"],
            dynamic_ids: vec!["strangers_shared_grief"],
        }
    }

    #[test]
    fn exact_match_scores_higher_than_wildcard() {
        let exact = make_entry(
            "exact",
            "character_signal",
            Some(vec!["stoic_survivor"]),
            Some(vec!["quiet_reunion"]),
        );
        let wildcard = make_entry("wild", "character_signal", None, None);
        let ctx = test_ctx();

        let exact_score = score_fragment(&exact, &ctx);
        let wild_score = score_fragment(&wildcard, &ctx);
        assert!(
            exact_score > wild_score,
            "exact {exact_score} should beat wildcard {wild_score}"
        );
    }

    #[test]
    fn non_matching_scores_zero() {
        let entry = make_entry(
            "miss",
            "character_signal",
            Some(vec!["byronic_hero"]),
            Some(vec!["farewell_scene"]),
        );
        let ctx = test_ctx();
        let s = score_fragment(&entry, &ctx);
        // One dimension (dynamic) is None → 0.5/3, others are non-matching → 0
        // Total: 0.5/3 ≈ 0.17
        assert!(s > 0.0, "wildcard dynamic should give partial score");
        assert!(
            s < 0.5,
            "non-matching archetypes+profiles should keep score low"
        );
    }

    #[test]
    fn empty_lexicon_returns_empty_fragments() {
        let goal = Goal {
            id: "test".to_string(),
            entity_id: String::new(),
            description: String::new(),
            category: "revelation".to_string(),
            visibility: "Signaled".to_string(),
            valence: "heavy".to_string(),
            lexicon: Vec::new(),
        };
        let ctx = test_ctx();
        let mut rng = rand::rng();
        let fragments = select_fragments(&goal, &ctx, &mut rng);
        assert!(fragments.is_empty());
    }

    #[test]
    fn respects_register_budgets() {
        let entries: Vec<LexiconEntry> = (0..10)
            .map(|i| make_entry(&format!("signal_{i}"), "character_signal", None, None))
            .chain((0..5).map(|i| make_entry(&format!("atmo_{i}"), "atmospheric", None, None)))
            .chain((0..3).map(|i| make_entry(&format!("trans_{i}"), "transitional", None, None)))
            .collect();

        let goal = Goal {
            id: "test".to_string(),
            entity_id: String::new(),
            description: String::new(),
            category: "revelation".to_string(),
            visibility: "Signaled".to_string(),
            valence: "heavy".to_string(),
            lexicon: entries,
        };
        let ctx = test_ctx();
        let mut rng = rand::rng();
        let fragments = select_fragments(&goal, &ctx, &mut rng);

        let signals = fragments
            .iter()
            .filter(|f| f.register == FragmentRegister::CharacterSignal)
            .count();
        let atmos = fragments
            .iter()
            .filter(|f| f.register == FragmentRegister::Atmospheric)
            .count();
        let trans = fragments
            .iter()
            .filter(|f| f.register == FragmentRegister::Transitional)
            .count();

        assert!(signals <= 3, "max 3 character_signal, got {signals}");
        assert!(atmos <= 2, "max 2 atmospheric, got {atmos}");
        assert!(trans <= 1, "max 1 transitional, got {trans}");
    }
}
