# Scene Goals and Character Intentions Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the narrator concrete, goal-directed character intentions at scene setup so characters have agency and scenes have dramaturgical direction, replacing the current pattern of reactive narrator drift.

**Architecture:** Author a goal vocabulary descriptor (`goals.json`) and tag existing descriptors (profiles, archetypes, dynamics) with goal references. At composition time, a two-pass set intersection determines active scene and character goals. A likeness pass selects behavioral lexicon fragments, and a single LLM call (qwen2.5:14b-instruct via Ollama) generates concrete situational intentions grounded in physical scene elements. These intentions are injected into the narrator preamble and persisted with the session.

**Tech Stack:** Rust (storyteller-core, storyteller-engine, storyteller-workshop), Python (tools/training), JSON descriptors (storyteller-data), Ollama (qwen2.5:14b-instruct)

**Spec:** `docs/plans/2026-03-11-scene-goals-and-character-intentions-design.md`

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `storyteller-data/training-data/descriptors/goals.json` | Create | Goal vocabulary — named goals with category, visibility, valence, empty lexicon |
| `storyteller-data/training-data/descriptors/profiles.json` | Modify | Add `scene_goals` array to each profile |
| `storyteller-data/training-data/descriptors/archetypes.json` | Modify | Add `pursuable_goals` array to each archetype |
| `storyteller-data/training-data/descriptors/dynamics.json` | Modify | Add `enabled_goals` and `blocked_goals` arrays to each dynamic |
| `crates/storyteller-engine/src/scene_composer/descriptors.rs` | Modify | Add `Goal` struct, goal fields on `Profile`/`Archetype`/`Dynamic`, load `goals.json` into `DescriptorSet` |
| `crates/storyteller-engine/src/scene_composer/goals.rs` | Create | Goal types (`SceneGoal`, `CharacterGoal`, `ComposedGoals`), two-pass set intersection, coherence/affinity check |
| `crates/storyteller-engine/src/scene_composer/likeness.rs` | Create | Likeness pass — dimensional match scoring, tensor affinity, diversity sampling, fragment selection |
| `crates/storyteller-engine/src/scene_composer/compose.rs` | Modify | Call goal intersection + likeness pass during `compose()`, include `ComposedGoals` in output |
| `crates/storyteller-engine/src/scene_composer/mod.rs` | Modify | Add `pub mod goals; pub mod likeness;`, update re-exports |
| `crates/storyteller-engine/src/inference/intention_generation.rs` | Create | Composition-time LLM call — build prompt from goals + fragments + scene context, parse structured JSON output, validation |
| `crates/storyteller-core/src/types/narrator_context.rs` | Modify | Add `SceneDirection`, `CharacterDrive` structs, add optional goal fields to `PersistentPreamble` |
| `crates/storyteller-engine/src/context/preamble.rs` | Modify | Accept `ComposedGoals` in `build_preamble()`, render Scene Direction / Character Drives / Player Context sections |
| `crates/storyteller-workshop/src-tauri/src/session.rs` | Modify | Add `save_goals()` / `load_goals()` for session `goals.json` persistence |
| `crates/storyteller-workshop/src-tauri/src/commands.rs` | Modify | Call intention generation after composition, pass goals to `build_preamble()`, persist goals, load on resume |
| `tools/training/src/goal_lexicon/` | Create | Python build-time lexicon enrichment pipeline |

---

## Chunk 1: Goal Vocabulary and Descriptor Integration

### Task 0: Author the goal vocabulary descriptor

**Files:**
- Create: `storyteller-data/training-data/descriptors/goals.json`

This task creates the initial goal vocabulary. Start with ~30 goals across all 8 categories, covering the three current genres (Cozy Ghost Story, Low Fantasy / Folklore, Sci-Fi Noir).

- [ ] **Step 1: Create goals.json**

Create the file with the full goal vocabulary. Each goal needs: `id`, `description`, `category`, `visibility`, `valence`, and an empty `lexicon` array. The lexicon will be populated by the build-time enrichment pipeline in a later task.

```json
{
  "goals": [
    {
      "id": "confront_shared_loss",
      "description": "Characters face a loss they both carry, moving toward acknowledgment rather than avoidance",
      "category": "revelation",
      "visibility": "Signaled",
      "valence": "heavy",
      "lexicon": []
    },
    {
      "id": "reveal_hidden_truth",
      "description": "A concealed fact surfaces through action or confession, changing what characters know about each other",
      "category": "revelation",
      "visibility": "Hidden",
      "valence": "tense",
      "lexicon": []
    },
    {
      "id": "discover_secret_identity",
      "description": "One character's true nature or role is exposed or deduced by another",
      "category": "revelation",
      "visibility": "Hidden",
      "valence": "tense",
      "lexicon": []
    },
    {
      "id": "approach_threshold_of_trust",
      "description": "Characters move from guarded interaction toward genuine openness, testing whether vulnerability is safe",
      "category": "relational_shift",
      "visibility": "Signaled",
      "valence": "warm",
      "lexicon": []
    },
    {
      "id": "test_loyalty",
      "description": "One character creates or encounters a situation that reveals whether another can be relied upon",
      "category": "relational_shift",
      "visibility": "Hidden",
      "valence": "tense",
      "lexicon": []
    },
    {
      "id": "forgive_old_wound",
      "description": "Characters confront a past injury and find a path toward reconciliation or at least understanding",
      "category": "relational_shift",
      "visibility": "Signaled",
      "valence": "heavy",
      "lexicon": []
    },
    {
      "id": "challenge_authority",
      "description": "A character directly contests someone with power over them, risking consequences",
      "category": "confrontation",
      "visibility": "Overt",
      "valence": "tense",
      "lexicon": []
    },
    {
      "id": "demand_accountability",
      "description": "A character insists another answer for past actions, refusing to let the matter rest",
      "category": "confrontation",
      "visibility": "Overt",
      "valence": "heavy",
      "lexicon": []
    },
    {
      "id": "defend_against_accusation",
      "description": "A character must respond to charges — true or false — that threaten their standing or relationships",
      "category": "confrontation",
      "visibility": "Overt",
      "valence": "tense",
      "lexicon": []
    },
    {
      "id": "investigate_anomaly",
      "description": "Something doesn't add up and a character is compelled to understand why",
      "category": "discovery",
      "visibility": "Signaled",
      "valence": "tense",
      "lexicon": []
    },
    {
      "id": "piece_together_clues",
      "description": "Fragments of information exist across characters or setting — assembling them reveals something new",
      "category": "discovery",
      "visibility": "Hidden",
      "valence": "ambiguous",
      "lexicon": []
    },
    {
      "id": "uncover_history",
      "description": "Characters excavate the past of a place, relationship, or event that shapes the present",
      "category": "discovery",
      "visibility": "Signaled",
      "valence": "heavy",
      "lexicon": []
    },
    {
      "id": "offer_shelter",
      "description": "A character extends protection or comfort to someone in need, whether asked for or not",
      "category": "bonding",
      "visibility": "Overt",
      "valence": "warm",
      "lexicon": []
    },
    {
      "id": "share_vulnerability",
      "description": "A character reveals weakness or need, creating an opening for connection or exploitation",
      "category": "bonding",
      "visibility": "Signaled",
      "valence": "warm",
      "lexicon": []
    },
    {
      "id": "find_common_ground",
      "description": "Characters with different backgrounds or positions discover a shared value or experience",
      "category": "bonding",
      "visibility": "Signaled",
      "valence": "warm",
      "lexicon": []
    },
    {
      "id": "prepare_to_leave",
      "description": "A character readies to depart — physically or emotionally — and the scene is shaped by what remains unsaid",
      "category": "departure",
      "visibility": "Signaled",
      "valence": "heavy",
      "lexicon": []
    },
    {
      "id": "say_what_must_be_said",
      "description": "Time is running out and something essential must be communicated before the chance is lost",
      "category": "departure",
      "visibility": "Overt",
      "valence": "heavy",
      "lexicon": []
    },
    {
      "id": "resist_farewell",
      "description": "A character refuses to accept that a departure is happening, fighting to maintain connection",
      "category": "departure",
      "visibility": "Signaled",
      "valence": "heavy",
      "lexicon": []
    },
    {
      "id": "protect_secret",
      "description": "A character actively works to keep specific information hidden from others in the scene",
      "category": "protection",
      "visibility": "Hidden",
      "valence": "tense",
      "lexicon": []
    },
    {
      "id": "shield_someone_vulnerable",
      "description": "A character positions themselves between a vulnerable person and a source of harm",
      "category": "protection",
      "visibility": "Signaled",
      "valence": "warm",
      "lexicon": []
    },
    {
      "id": "maintain_deception",
      "description": "A character sustains a false narrative, managing the gap between what they present and what is true",
      "category": "protection",
      "visibility": "Hidden",
      "valence": "tense",
      "lexicon": []
    },
    {
      "id": "negotiate_terms",
      "description": "Characters haggle over the conditions of an exchange, alliance, or agreement",
      "category": "transaction",
      "visibility": "Overt",
      "valence": "tense",
      "lexicon": []
    },
    {
      "id": "secure_resource",
      "description": "A character needs something specific — an object, information, passage — and must obtain it through social means",
      "category": "transaction",
      "visibility": "Signaled",
      "valence": "ambiguous",
      "lexicon": []
    },
    {
      "id": "honor_obligation",
      "description": "A character fulfills a duty or debt, even when it conflicts with their immediate desires",
      "category": "relational_shift",
      "visibility": "Signaled",
      "valence": "heavy",
      "lexicon": []
    },
    {
      "id": "resist_comfort",
      "description": "A character refuses offered warmth or aid because accepting would mean acknowledging pain",
      "category": "protection",
      "visibility": "Signaled",
      "valence": "heavy",
      "lexicon": []
    },
    {
      "id": "test_the_room",
      "description": "A character assesses the safety of a social space before committing to openness",
      "category": "discovery",
      "visibility": "Hidden",
      "valence": "tense",
      "lexicon": []
    },
    {
      "id": "draw_out_the_unspoken",
      "description": "A character works to create conditions where another can safely share what they are carrying",
      "category": "bonding",
      "visibility": "Hidden",
      "valence": "warm",
      "lexicon": []
    },
    {
      "id": "reclaim_agency",
      "description": "A character who has been acted upon asserts their own will and direction",
      "category": "confrontation",
      "visibility": "Signaled",
      "valence": "tense",
      "lexicon": []
    },
    {
      "id": "broker_alliance",
      "description": "A character works to bring together parties who have reason to distrust each other",
      "category": "transaction",
      "visibility": "Overt",
      "valence": "ambiguous",
      "lexicon": []
    },
    {
      "id": "mutual_confession",
      "description": "Both characters have something to reveal, and the scene creates space for reciprocal honesty",
      "category": "revelation",
      "visibility": "Signaled",
      "valence": "heavy",
      "lexicon": []
    }
  ]
}
```

- [ ] **Step 2: Verify JSON is valid**

Run: `python3 -c "import json; json.load(open('storyteller-data/training-data/descriptors/goals.json')); print('Valid')"`

Note: Run from the `storyteller-data` parent directory or adjust path. Expected: `Valid`

- [ ] **Step 3: Commit**

```bash
git add storyteller-data/training-data/descriptors/goals.json
git commit -m "feat(descriptors): add goal vocabulary with 30 initial goals across 8 categories"
```

---

### Task 1: Tag existing descriptors with goal references

**Files:**
- Modify: `storyteller-data/training-data/descriptors/profiles.json`
- Modify: `storyteller-data/training-data/descriptors/archetypes.json`
- Modify: `storyteller-data/training-data/descriptors/dynamics.json`

Add goal reference arrays to each existing descriptor entry. These are the inputs to the two-pass set intersection.

- [ ] **Step 1: Add `scene_goals` to each profile**

Add a `"scene_goals"` array to each of the 10 profiles in `profiles.json`. Each profile should reference 2-4 goals from `goals.json` that represent what scenes of this type are *for*. Examples:

- `quiet_reunion`: `["confront_shared_loss", "approach_threshold_of_trust", "share_vulnerability"]`
- `confrontation_over_betrayal`: `["demand_accountability", "reveal_hidden_truth", "test_loyalty"]`
- `vulnerable_admission`: `["share_vulnerability", "mutual_confession", "approach_threshold_of_trust"]`
- `celebration_interrupted`: `["reveal_hidden_truth", "challenge_authority", "protect_secret"]`
- `first_meeting_hidden_agendas`: `["test_the_room", "maintain_deception", "investigate_anomaly"]`
- `physical_challenge_cooperation`: `["find_common_ground", "test_loyalty", "honor_obligation"]`
- `quiet_aftermath`: `["confront_shared_loss", "forgive_old_wound", "prepare_to_leave"]`
- `negotiation_under_pressure`: `["negotiate_terms", "secure_resource", "test_loyalty"]`
- `discovery_of_truth`: `["reveal_hidden_truth", "piece_together_clues", "uncover_history"]`
- `farewell_scene`: `["say_what_must_be_said", "resist_farewell", "prepare_to_leave"]`

- [ ] **Step 2: Add `pursuable_goals` to each archetype**

Add a `"pursuable_goals"` array to each of the 12 archetypes in `archetypes.json`. Each archetype should reference 4-6 goals representing what this character type *can want*. Examples:

- `wandering_artist`: `["share_vulnerability", "find_common_ground", "reclaim_agency", "investigate_anomaly"]`
- `stoic_survivor`: `["protect_secret", "resist_comfort", "test_the_room", "honor_obligation"]`
- `byronic_hero`: `["resist_comfort", "reclaim_agency", "maintain_deception", "challenge_authority"]`
- `protective_parent`: `["shield_someone_vulnerable", "draw_out_the_unspoken", "offer_shelter", "confront_shared_loss"]`
- `wise_elder`: `["draw_out_the_unspoken", "reveal_hidden_truth", "forgive_old_wound", "share_vulnerability"]`
- `clever_trickster`: `["maintain_deception", "investigate_anomaly", "test_loyalty", "secure_resource"]`
- `grieving_youth`: `["resist_comfort", "confront_shared_loss", "test_the_room", "share_vulnerability"]`
- `reluctant_leader`: `["honor_obligation", "challenge_authority", "protect_secret", "broker_alliance"]`
- `zealous_reformer`: `["challenge_authority", "demand_accountability", "reveal_hidden_truth", "reclaim_agency"]`
- `withdrawn_scholar`: `["piece_together_clues", "investigate_anomaly", "protect_secret", "test_the_room"]`
- `loyal_soldier`: `["honor_obligation", "shield_someone_vulnerable", "test_loyalty", "protect_secret"]`
- `fey_outsider`: `["investigate_anomaly", "test_the_room", "maintain_deception", "find_common_ground"]`

- [ ] **Step 3: Add `enabled_goals` and `blocked_goals` to each dynamic**

Add both arrays to each of the 10 dynamics in `dynamics.json`. `enabled_goals` (3-5 goals) are what this relational shape makes possible. `blocked_goals` (1-3 goals) are what it prevents. Examples:

- `mentor_student`: enabled `["draw_out_the_unspoken", "share_vulnerability", "challenge_authority"]`, blocked `["negotiate_terms"]`
- `siblings_complex`: enabled `["confront_shared_loss", "demand_accountability", "forgive_old_wound"]`, blocked `["maintain_deception"]`
- `rivals_with_respect`: enabled `["test_loyalty", "challenge_authority", "find_common_ground"]`, blocked `["offer_shelter"]`
- `strangers_in_shared_grief`: enabled `["mutual_confession", "confront_shared_loss", "approach_threshold_of_trust", "share_vulnerability"]`, blocked `["negotiate_terms"]`
- `authority_subject`: enabled `["challenge_authority", "honor_obligation", "demand_accountability"]`, blocked `["mutual_confession"]`
- `former_friends_estranged`: enabled `["forgive_old_wound", "demand_accountability", "confront_shared_loss"]`, blocked `["find_common_ground"]`
- `patron_protege`: enabled `["honor_obligation", "test_loyalty", "share_vulnerability"]`, blocked `["challenge_authority"]`
- `companions_on_the_road`: enabled `["find_common_ground", "share_vulnerability", "test_loyalty"]`, blocked `["demand_accountability"]`
- `suspicious_allies`: enabled `["test_loyalty", "investigate_anomaly", "maintain_deception"]`, blocked `["share_vulnerability"]`
- `guardian_ward`: enabled `["shield_someone_vulnerable", "draw_out_the_unspoken", "resist_comfort"]`, blocked `["negotiate_terms"]`

- [ ] **Step 4: Validate all descriptor files are valid JSON**

Run: `for f in profiles archetypes dynamics; do python3 -c "import json; json.load(open('storyteller-data/training-data/descriptors/${f}.json')); print('${f}: OK')"; done`

Expected: All three print OK.

- [ ] **Step 5: Commit**

```bash
git add storyteller-data/training-data/descriptors/profiles.json storyteller-data/training-data/descriptors/archetypes.json storyteller-data/training-data/descriptors/dynamics.json
git commit -m "feat(descriptors): tag profiles, archetypes, and dynamics with goal references"
```

---

### Task 2: Deserialize goal fields in Rust descriptor types

**Files:**
- Modify: `crates/storyteller-engine/src/scene_composer/descriptors.rs`

Add the `Goal` struct and the new optional goal-reference fields to `Profile`, `Archetype`, and `Dynamic`. The fields use `#[serde(default)]` so existing tests and any descriptors without goals still load.

- [ ] **Step 1: Write failing tests**

Add tests to the existing `tests` module in `descriptors.rs`:

```rust
#[test]
fn goals_descriptor_loads() {
    let data_path = std::env::var("STORYTELLER_DATA_PATH")
        .expect("STORYTELLER_DATA_PATH must be set");
    let set = DescriptorSet::load(Path::new(&data_path))
        .expect("descriptors should load");
    assert!(
        !set.goals.is_empty(),
        "should have loaded at least one goal"
    );

    let goal = set.goals.iter().find(|g| g.id == "protect_secret").expect("protect_secret should exist");
    assert_eq!(goal.category, "protection");
    assert_eq!(goal.visibility, "Hidden");
    assert_eq!(goal.valence, "tense");
}

#[test]
fn profiles_have_scene_goals() {
    let data_path = std::env::var("STORYTELLER_DATA_PATH")
        .expect("STORYTELLER_DATA_PATH must be set");
    let set = DescriptorSet::load(Path::new(&data_path))
        .expect("descriptors should load");
    let quiet_reunion = set.profiles.iter().find(|p| p.id == "quiet_reunion")
        .expect("quiet_reunion should exist");
    assert!(
        !quiet_reunion.scene_goals.is_empty(),
        "quiet_reunion should have scene_goals"
    );
}

#[test]
fn archetypes_have_pursuable_goals() {
    let data_path = std::env::var("STORYTELLER_DATA_PATH")
        .expect("STORYTELLER_DATA_PATH must be set");
    let set = DescriptorSet::load(Path::new(&data_path))
        .expect("descriptors should load");
    let stoic = set.archetypes.iter().find(|a| a.id == "stoic_survivor")
        .expect("stoic_survivor should exist");
    assert!(stoic.pursuable_goals.contains(&"protect_secret".to_string()));
}

#[test]
fn dynamics_have_enabled_and_blocked_goals() {
    let data_path = std::env::var("STORYTELLER_DATA_PATH")
        .expect("STORYTELLER_DATA_PATH must be set");
    let set = DescriptorSet::load(Path::new(&data_path))
        .expect("descriptors should load");
    let grief = set.dynamics.iter().find(|d| d.id == "strangers_in_shared_grief")
        .expect("strangers_in_shared_grief should exist");
    assert!(!grief.enabled_goals.is_empty());
    assert!(!grief.blocked_goals.is_empty());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p storyteller-engine goals_descriptor_loads profiles_have_scene_goals archetypes_have_pursuable_goals dynamics_have_enabled_and_blocked_goals -- --nocapture`

Expected: Compilation errors — `Goal` type doesn't exist, fields not on structs.

- [ ] **Step 3: Add Goal struct and update descriptor types**

In `descriptors.rs`, add the `Goal` struct near the other descriptor types:

```rust
/// A named narrative goal from the goal vocabulary.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Goal {
    pub id: String,
    pub description: String,
    pub category: String,
    pub visibility: String,
    pub valence: String,
    #[serde(default)]
    pub lexicon: Vec<LexiconEntry>,
}

/// A behavioral lexicon entry generated by the build-time enrichment pipeline.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LexiconEntry {
    pub fragment: String,
    pub register: String,
    pub dimensional_context: DimensionalContext,
}

/// Dimensional affinity tags for a lexicon fragment.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DimensionalContext {
    pub archetypes: Option<Vec<String>>,
    pub profiles: Option<Vec<String>>,
    pub dynamics: Option<Vec<String>>,
    #[serde(default)]
    pub valence: Vec<String>,
}
```

Add goal-reference fields to existing structs (all with `#[serde(default)]`):

On `Profile` (after `characteristic_events`):
```rust
    #[serde(default)]
    pub scene_goals: Vec<String>,
```

On `Archetype` (after `action_tendencies`):
```rust
    #[serde(default)]
    pub pursuable_goals: Vec<String>,
```

On `Dynamic` (after `topology_b`):
```rust
    #[serde(default)]
    pub enabled_goals: Vec<String>,
    #[serde(default)]
    pub blocked_goals: Vec<String>,
```

Add `goals` to `DescriptorSet`:
```rust
pub struct DescriptorSet {
    // ... existing fields ...
    pub goals: Vec<Goal>,
}
```

Wrapper for the goals file shape:
```rust
#[derive(Debug, serde::Deserialize)]
struct GoalsFile {
    goals: Vec<Goal>,
}
```

- [ ] **Step 4: Update `DescriptorSet::load` to load goals.json**

In the `load` function, add after the existing `load_required` calls (note: `desc_dir` is the local variable pointing to `base_dir/training-data/descriptors/`):

```rust
let goals_file = Self::load_required::<GoalsFile>(&desc_dir, "goals.json")?.goals;
```

And include `goals: goals_file` in the `DescriptorSet` construction.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p storyteller-engine goals_descriptor_loads profiles_have_scene_goals archetypes_have_pursuable_goals dynamics_have_enabled_and_blocked_goals -- --nocapture`

Expected: All 4 tests pass.

- [ ] **Step 6: Run full test suite to verify no regressions**

Run: `cargo test -p storyteller-engine -- --nocapture`

Expected: All existing tests pass. The `#[serde(default)]` on new fields ensures backward compatibility.

- [ ] **Step 7: Commit**

```bash
git add crates/storyteller-engine/src/scene_composer/descriptors.rs
git commit -m "feat(descriptors): add Goal type and goal-reference fields to Profile, Archetype, Dynamic"
```

---

## Chunk 2: Goal Intersection and Likeness Pass

### Task 3: Implement two-pass goal intersection

**Files:**
- Create: `crates/storyteller-engine/src/scene_composer/goals.rs`
- Modify: `crates/storyteller-engine/src/scene_composer/mod.rs`

The set intersection logic: Pass 1 finds scene goals (profile ∩ cast archetypes), Pass 2 finds per-character goals (archetype ∩ dynamics - blocked), then coherence filtering.

- [ ] **Step 1: Write failing tests**

Create `crates/storyteller-engine/src/scene_composer/goals.rs` with types and test module:

```rust
//! Goal intersection — determines which scene and character goals are active
//! for a composed scene based on descriptor tagging.
//!
//! See: `docs/plans/2026-03-11-scene-goals-and-character-intentions-design.md`

use std::collections::{HashMap, HashSet};

use storyteller_core::types::entity::EntityId;

use super::descriptors::{Archetype, Dynamic, Goal, LexiconEntry, Profile};

/// Visibility level for how a goal manifests to the player.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GoalVisibility {
    Overt,
    Signaled,
    Hidden,
    Structural,
}

/// Fragment register — how a lexicon fragment is used in the narrator prompt.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FragmentRegister {
    Atmospheric,
    CharacterSignal,
    Transitional,
}

/// A selected lexicon fragment for narrator context.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GoalFragment {
    pub text: String,
    pub register: FragmentRegister,
}

/// An active scene-level goal with selected fragments.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SceneGoal {
    pub goal_id: String,
    pub visibility: GoalVisibility,
    pub category: String,
    pub fragments: Vec<GoalFragment>,
}

/// An active per-character goal with selected fragments.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CharacterGoal {
    pub goal_id: String,
    pub visibility: GoalVisibility,
    pub category: String,
    pub fragments: Vec<GoalFragment>,
}

/// The full set of active goals for a composed scene.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ComposedGoals {
    pub scene_goals: Vec<SceneGoal>,
    pub character_goals: HashMap<EntityId, Vec<CharacterGoal>>,
}

/// Category affinity pairs — which goal categories are coherent together.
fn category_affinity(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    let compatible = match a {
        "revelation" => &["protection", "relational_shift", "discovery"][..],
        "relational_shift" => &["revelation", "bonding", "confrontation", "departure"],
        "confrontation" => &["relational_shift", "protection", "revelation"],
        "discovery" => &["revelation", "bonding", "protection"],
        "bonding" => &["relational_shift", "discovery", "departure"],
        "departure" => &["relational_shift", "bonding", "revelation"],
        "protection" => &["revelation", "confrontation", "discovery"],
        "transaction" => &["transaction", "confrontation"],
        _ => &[],
    };
    compatible.contains(&b)
}

/// A cast member's identity for goal intersection.
pub struct CastMember {
    pub entity_id: EntityId,
    pub archetype: Archetype,
    pub dynamics: Vec<Dynamic>,
}

/// Pass 1: Scene dramaturgical goals.
///
/// `scene_goals = profile.scene_goals ∩ (union of all cast archetypes' pursuable_goals)`
pub fn intersect_scene_goals(
    profile: &Profile,
    cast: &[CastMember],
    goals: &[Goal],
) -> Vec<SceneGoal> {
    let cast_pursuable: HashSet<&str> = cast
        .iter()
        .flat_map(|m| m.archetype.pursuable_goals.iter().map(|s| s.as_str()))
        .collect();

    profile
        .scene_goals
        .iter()
        .filter(|sg| cast_pursuable.contains(sg.as_str()))
        .filter_map(|sg| {
            goals.iter().find(|g| g.id == *sg).map(|g| SceneGoal {
                goal_id: g.id.clone(),
                visibility: parse_visibility(&g.visibility),
                category: g.category.clone(),
                fragments: Vec::new(), // populated by likeness pass
            })
        })
        .collect()
}

/// Pass 2: Per-character objectives.
///
/// For each character:
/// `character_goals = archetype.pursuable_goals
///                    ∩ (union of dynamics.enabled_goals for this character)
///                    - (union of dynamics.blocked_goals for this character)`
///
/// Then coherence-filtered against scene goal categories.
pub fn intersect_character_goals(
    member: &CastMember,
    scene_goals: &[SceneGoal],
    goals: &[Goal],
) -> Vec<CharacterGoal> {
    let enabled: HashSet<&str> = member
        .dynamics
        .iter()
        .flat_map(|d| d.enabled_goals.iter().map(|s| s.as_str()))
        .collect();

    let blocked: HashSet<&str> = member
        .dynamics
        .iter()
        .flat_map(|d| d.blocked_goals.iter().map(|s| s.as_str()))
        .collect();

    let scene_categories: Vec<&str> = scene_goals.iter().map(|sg| sg.category.as_str()).collect();

    member
        .archetype
        .pursuable_goals
        .iter()
        .filter(|pg| enabled.contains(pg.as_str()))
        .filter(|pg| !blocked.contains(pg.as_str()))
        .filter_map(|pg| goals.iter().find(|g| g.id == *pg))
        .filter(|g| {
            // Coherence: character goal category must be affine with at least one scene goal category.
            // If no scene goals, skip coherence check (graceful degradation).
            scene_categories.is_empty()
                || scene_categories
                    .iter()
                    .any(|sc| category_affinity(&g.category, sc))
        })
        .map(|g| CharacterGoal {
            goal_id: g.id.clone(),
            visibility: parse_visibility(&g.visibility),
            category: g.category.clone(),
            fragments: Vec::new(), // populated by likeness pass
        })
        .collect()
}

fn parse_visibility(s: &str) -> GoalVisibility {
    match s {
        "Overt" => GoalVisibility::Overt,
        "Signaled" => GoalVisibility::Signaled,
        "Hidden" => GoalVisibility::Hidden,
        "Structural" => GoalVisibility::Structural,
        _ => GoalVisibility::Signaled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene_composer::descriptors::*;

    fn test_goal(id: &str, category: &str, visibility: &str) -> Goal {
        Goal {
            id: id.to_string(),
            description: format!("Test goal: {id}"),
            category: category.to_string(),
            visibility: visibility.to_string(),
            valence: "tense".to_string(),
            lexicon: Vec::new(),
        }
    }

    fn test_profile(scene_goals: Vec<&str>) -> Profile {
        Profile {
            id: "test_profile".to_string(),
            display_name: "Test".to_string(),
            description: "Test profile".to_string(),
            scene_type: "Gravitational".to_string(),
            tension: RangeBounds { min: 0.3, max: 0.7 },
            cast_size: RangeBounds { min: 2.0, max: 4.0 },
            characteristic_events: Vec::new(),
            scene_goals: scene_goals.into_iter().map(String::from).collect(),
        }
    }

    fn test_archetype(id: &str, pursuable: Vec<&str>) -> Archetype {
        Archetype {
            id: id.to_string(),
            display_name: id.to_string(),
            description: String::new(),
            axes: Vec::new(),
            default_emotional_profile: EmotionalProfile {
                grammar_id: "western".to_string(),
                primaries: Vec::new(),
            },
            default_self_edge: SelfEdge {
                trust_competence: RangeBounds { min: 0.5, max: 0.5 },
                trust_intentions: RangeBounds { min: 0.5, max: 0.5 },
                trust_reliability: RangeBounds { min: 0.5, max: 0.5 },
                affection: RangeBounds { min: 0.5, max: 0.5 },
                debt: RangeBounds { min: 0.0, max: 0.0 },
                history_weight: RangeBounds { min: 0.5, max: 0.5 },
                projection_accuracy: RangeBounds { min: 0.5, max: 0.5 },
            },
            action_tendencies: ActionTendencies {
                primary_action_types: vec!["Speak".to_string()],
                primary_action_contexts: vec!["Conversation".to_string()],
                speech_likelihood: 0.7,
                speech_registers: vec!["Tender".to_string()],
                default_awareness: "Conscious".to_string(),
            },
            pursuable_goals: pursuable.into_iter().map(String::from).collect(),
        }
    }

    fn test_dynamic(enabled: Vec<&str>, blocked: Vec<&str>) -> Dynamic {
        Dynamic {
            id: "test_dynamic".to_string(),
            display_name: "Test".to_string(),
            description: String::new(),
            role_a: "role_a".to_string(),
            role_b: "role_b".to_string(),
            edge_a_to_b: RelationalEdge {
                trust_reliability: RangeBounds { min: 0.3, max: 0.6 },
                trust_competence: RangeBounds { min: 0.3, max: 0.6 },
                trust_benevolence: RangeBounds { min: 0.3, max: 0.6 },
                affection: RangeBounds { min: 0.2, max: 0.5 },
                debt: RangeBounds { min: 0.0, max: 0.1 },
            },
            edge_b_to_a: RelationalEdge {
                trust_reliability: RangeBounds { min: 0.3, max: 0.6 },
                trust_competence: RangeBounds { min: 0.3, max: 0.6 },
                trust_benevolence: RangeBounds { min: 0.3, max: 0.6 },
                affection: RangeBounds { min: 0.2, max: 0.5 },
                debt: RangeBounds { min: 0.0, max: 0.1 },
            },
            topology_a: "Hub".to_string(),
            topology_b: "Bridge".to_string(),
            enabled_goals: enabled.into_iter().map(String::from).collect(),
            blocked_goals: blocked.into_iter().map(String::from).collect(),
        }
    }

    #[test]
    fn scene_goals_intersection_filters_by_cast() {
        let goals = vec![
            test_goal("protect_secret", "protection", "Hidden"),
            test_goal("share_vulnerability", "bonding", "Signaled"),
            test_goal("negotiate_terms", "transaction", "Overt"),
        ];
        let profile = test_profile(vec!["protect_secret", "share_vulnerability", "negotiate_terms"]);
        let cast = vec![CastMember {
            entity_id: EntityId::new(),
            archetype: test_archetype("stoic", vec!["protect_secret"]),
            dynamics: Vec::new(),
        }];

        let result = intersect_scene_goals(&profile, &cast, &goals);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].goal_id, "protect_secret");
    }

    #[test]
    fn scene_goals_empty_when_no_cast_overlap() {
        let goals = vec![test_goal("negotiate_terms", "transaction", "Overt")];
        let profile = test_profile(vec!["negotiate_terms"]);
        let cast = vec![CastMember {
            entity_id: EntityId::new(),
            archetype: test_archetype("stoic", vec!["protect_secret"]),
            dynamics: Vec::new(),
        }];

        let result = intersect_scene_goals(&profile, &cast, &goals);
        assert!(result.is_empty());
    }

    #[test]
    fn character_goals_intersection_applies_enabled_and_blocked() {
        let goals = vec![
            test_goal("protect_secret", "protection", "Hidden"),
            test_goal("share_vulnerability", "bonding", "Signaled"),
            test_goal("test_loyalty", "relational_shift", "Hidden"),
        ];
        let scene_goals = vec![SceneGoal {
            goal_id: "confront_shared_loss".to_string(),
            visibility: GoalVisibility::Signaled,
            category: "revelation".to_string(),
            fragments: Vec::new(),
        }];
        let member = CastMember {
            entity_id: EntityId::new(),
            archetype: test_archetype("stoic", vec!["protect_secret", "share_vulnerability", "test_loyalty"]),
            dynamics: vec![test_dynamic(
                vec!["protect_secret", "share_vulnerability", "test_loyalty"],
                vec!["share_vulnerability"], // blocked!
            )],
        };

        let result = intersect_character_goals(&member, &scene_goals, &goals);
        // share_vulnerability is blocked, so only protect_secret and test_loyalty remain
        // protect_secret (protection) is affine with revelation ✓
        // test_loyalty (relational_shift) is affine with revelation ✓
        let ids: Vec<&str> = result.iter().map(|g| g.goal_id.as_str()).collect();
        assert!(ids.contains(&"protect_secret"));
        assert!(ids.contains(&"test_loyalty"));
        assert!(!ids.contains(&"share_vulnerability"));
    }

    #[test]
    fn coherence_filter_removes_incompatible_categories() {
        let goals = vec![
            test_goal("protect_secret", "protection", "Hidden"),
            test_goal("negotiate_terms", "transaction", "Overt"),
        ];
        let scene_goals = vec![SceneGoal {
            goal_id: "share_vulnerability".to_string(),
            visibility: GoalVisibility::Signaled,
            category: "bonding".to_string(),
            fragments: Vec::new(),
        }];
        let member = CastMember {
            entity_id: EntityId::new(),
            archetype: test_archetype("stoic", vec!["protect_secret", "negotiate_terms"]),
            dynamics: vec![test_dynamic(
                vec!["protect_secret", "negotiate_terms"],
                Vec::new(),
            )],
        };

        let result = intersect_character_goals(&member, &scene_goals, &goals);
        // bonding is compatible with: relational_shift, discovery, departure
        // protection is NOT compatible with bonding
        // transaction is NOT compatible with bonding
        // So both should be filtered out
        assert!(result.is_empty());
    }

    #[test]
    fn no_scene_goals_skips_coherence_check() {
        let goals = vec![
            test_goal("protect_secret", "protection", "Hidden"),
        ];
        let member = CastMember {
            entity_id: EntityId::new(),
            archetype: test_archetype("stoic", vec!["protect_secret"]),
            dynamics: vec![test_dynamic(vec!["protect_secret"], Vec::new())],
        };

        let result = intersect_character_goals(&member, &[], &goals);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].goal_id, "protect_secret");
    }

    #[test]
    fn category_affinity_is_symmetric() {
        assert!(category_affinity("revelation", "protection"));
        assert!(category_affinity("protection", "revelation"));
        assert!(category_affinity("bonding", "discovery"));
        assert!(category_affinity("discovery", "bonding"));
    }

    #[test]
    fn category_affinity_same_category() {
        assert!(category_affinity("revelation", "revelation"));
        assert!(category_affinity("transaction", "transaction"));
    }

    #[test]
    fn transaction_is_isolated() {
        assert!(!category_affinity("transaction", "bonding"));
        assert!(!category_affinity("transaction", "revelation"));
        assert!(category_affinity("transaction", "confrontation"));
    }
}
```

- [ ] **Step 2: Register the module**

In `crates/storyteller-engine/src/scene_composer/mod.rs`, add:

```rust
pub mod goals;
```

And update the re-exports line:

```rust
pub use goals::{ComposedGoals, SceneGoal, CharacterGoal, GoalVisibility, CastMember};
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p storyteller-engine scene_goals_intersection character_goals_intersection coherence_filter category_affinity no_scene_goals -- --nocapture`

Expected: All 7 tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-engine/src/scene_composer/goals.rs crates/storyteller-engine/src/scene_composer/mod.rs
git commit -m "feat(goals): implement two-pass goal intersection with coherence filtering"
```

---

### Task 4: Implement likeness pass for fragment selection

**Files:**
- Create: `crates/storyteller-engine/src/scene_composer/likeness.rs`
- Modify: `crates/storyteller-engine/src/scene_composer/mod.rs`

Scores lexicon fragments against scene context and selects a diverse subset per goal.

- [ ] **Step 1: Write the module with tests**

Create `crates/storyteller-engine/src/scene_composer/likeness.rs`:

```rust
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
            FragmentRegister::Transitional if transitionals.len() < 1 => {
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
    use super::*;
    use super::super::descriptors::DimensionalContext;

    fn make_entry(fragment: &str, register: &str, archetypes: Option<Vec<&str>>, profiles: Option<Vec<&str>>) -> LexiconEntry {
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
            dynamic_ids: vec!["strangers_in_shared_grief"],
        }
    }

    #[test]
    fn exact_match_scores_higher_than_wildcard() {
        let exact = make_entry("exact", "character_signal", Some(vec!["stoic_survivor"]), Some(vec!["quiet_reunion"]));
        let wildcard = make_entry("wild", "character_signal", None, None);
        let ctx = test_ctx();

        let exact_score = score_fragment(&exact, &ctx);
        let wild_score = score_fragment(&wildcard, &ctx);
        assert!(exact_score > wild_score, "exact {exact_score} should beat wildcard {wild_score}");
    }

    #[test]
    fn non_matching_scores_zero() {
        let entry = make_entry("miss", "character_signal", Some(vec!["byronic_hero"]), Some(vec!["farewell_scene"]));
        let ctx = test_ctx();
        let s = score_fragment(&entry, &ctx);
        // One dimension (dynamic) is None → 0.5/3, others are non-matching → 0
        // Total: 0.5/3 ≈ 0.17
        assert!(s > 0.0, "wildcard dynamic should give partial score");
        assert!(s < 0.5, "non-matching archetypes+profiles should keep score low");
    }

    #[test]
    fn empty_lexicon_returns_empty_fragments() {
        let goal = Goal {
            id: "test".to_string(),
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
            description: String::new(),
            category: "revelation".to_string(),
            visibility: "Signaled".to_string(),
            valence: "heavy".to_string(),
            lexicon: entries,
        };
        let ctx = test_ctx();
        let mut rng = rand::rng();
        let fragments = select_fragments(&goal, &ctx, &mut rng);

        let signals = fragments.iter().filter(|f| f.register == FragmentRegister::CharacterSignal).count();
        let atmos = fragments.iter().filter(|f| f.register == FragmentRegister::Atmospheric).count();
        let trans = fragments.iter().filter(|f| f.register == FragmentRegister::Transitional).count();

        assert!(signals <= 3, "max 3 character_signal, got {signals}");
        assert!(atmos <= 2, "max 2 atmospheric, got {atmos}");
        assert!(trans <= 1, "max 1 transitional, got {trans}");
    }
}
```

- [ ] **Step 2: Register the module**

In `crates/storyteller-engine/src/scene_composer/mod.rs`, add:

```rust
pub mod likeness;
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p storyteller-engine likeness -- --nocapture`

Expected: All 4 tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-engine/src/scene_composer/likeness.rs crates/storyteller-engine/src/scene_composer/mod.rs
git commit -m "feat(goals): implement likeness pass for lexicon fragment selection"
```

---

## Chunk 3: Preamble Integration and Intention Generation

### Task 5: Add goal types to narrator context in storyteller-core

**Files:**
- Modify: `crates/storyteller-core/src/types/narrator_context.rs`

Add `SceneDirection` and `CharacterDrive` structs, and optional goal fields to `PersistentPreamble`.

- [ ] **Step 1: Add types and update PersistentPreamble**

In `narrator_context.rs`, add after the `PersistentPreamble` struct definition:

```rust
/// Scene-level dramaturgical direction from goal system.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SceneDirection {
    /// What this scene is actually about — the specific situation.
    pub dramatic_tension: String,
    /// Where the scene is headed — the moment it's building toward.
    pub trajectory: String,
}

/// A character's concrete drive in this scene from goal system.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CharacterDrive {
    /// Character name.
    pub name: String,
    /// What they are concretely trying to do.
    pub objective: String,
    /// What makes it hard.
    pub constraint: String,
    /// How they pursue the objective — manner and tactics.
    pub behavioral_stance: String,
}
```

Add three fields to `PersistentPreamble` (after `boundaries`):

```rust
    /// Scene-level dramaturgical direction. None when goals system is not active.
    #[serde(default)]
    pub scene_direction: Option<SceneDirection>,
    /// Per-character drives generated from goals. Empty when goals system is not active.
    #[serde(default)]
    pub character_drives: Vec<CharacterDrive>,
    /// Player-facing goal context. None when goals system is not active or player has no goals.
    #[serde(default)]
    pub player_context: Option<String>,
```

- [ ] **Step 2: Update existing tests that construct PersistentPreamble**

Any test that constructs `PersistentPreamble` as a struct literal (e.g., in `narrator_context.rs` tests, `preamble.rs` tests) must add the new fields with default values:

```rust
scene_direction: None,
character_drives: Vec::new(),
player_context: None,
```

Search for `PersistentPreamble {` across the workspace to find all construction sites.

- [ ] **Step 3: Verify compilation**

Run: `cargo check --workspace --all-features`

Expected: Clean compilation. All existing struct literal constructions must include the new fields.

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-core/src/types/narrator_context.rs
git commit -m "feat(core): add SceneDirection, CharacterDrive types and goal fields on PersistentPreamble"
```

Note: If preamble.rs tests needed updating, include that file in the commit too.

---

### Task 6: Update preamble builder and renderer

**Files:**
- Modify: `crates/storyteller-engine/src/context/preamble.rs`

Update `build_preamble()` to accept composed goals, and `render_preamble()` to emit the new sections.

- [ ] **Step 1: Write failing test**

Add a test to the existing test module in `preamble.rs`:

```rust
#[test]
fn preamble_renders_scene_direction_and_drives() {
    let preamble = PersistentPreamble {
        narrator_identity: "Literary fiction narrator".to_string(),
        anti_patterns: vec!["Never break character".to_string()],
        setting_description: "A quiet rectory".to_string(),
        cast_descriptions: Vec::new(),
        boundaries: Vec::new(),
        scene_direction: Some(SceneDirection {
            dramatic_tension: "Arthur came for a hidden letter.".to_string(),
            trajectory: "Toward a moment of trust or betrayal.".to_string(),
        }),
        character_drives: vec![CharacterDrive {
            name: "Arthur".to_string(),
            objective: "Retrieve the letter from the tea caddy.".to_string(),
            constraint: "Margaret keeps gravitating to the mantel.".to_string(),
            behavioral_stance: "Polite deflection masking quiet urgency.".to_string(),
        }],
        player_context: Some("You sense Arthur is here for more than tea.".to_string()),
    };

    let rendered = render_preamble(&preamble);
    assert!(rendered.contains("## Scene Direction"), "should have scene direction header");
    assert!(rendered.contains("Arthur came for a hidden letter."), "should contain dramatic tension");
    assert!(rendered.contains("## Arthur's Drive"), "should have character drive header");
    assert!(rendered.contains("Retrieve the letter"), "should contain objective");
    assert!(rendered.contains("## Player Context"), "should have player context header");
}

#[test]
fn preamble_without_goals_renders_same_as_before() {
    let preamble = PersistentPreamble {
        narrator_identity: "Literary fiction narrator".to_string(),
        anti_patterns: vec!["Never break character".to_string()],
        setting_description: "A quiet rectory".to_string(),
        cast_descriptions: Vec::new(),
        boundaries: Vec::new(),
        scene_direction: None,
        character_drives: Vec::new(),
        player_context: None,
    };

    let rendered = render_preamble(&preamble);
    assert!(!rendered.contains("## Scene Direction"), "should not have scene direction");
    assert!(!rendered.contains("Drive"), "should not have character drives");
    assert!(!rendered.contains("## Player Context"), "should not have player context");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p storyteller-engine preamble_renders_scene_direction preamble_without_goals -- --nocapture`

Expected: Compilation errors (new fields don't exist on PersistentPreamble yet from engine's perspective) or assertion failures.

- [ ] **Step 3: Update render_preamble()**

In `render_preamble()`, add new sections *before* the existing Boundaries section. The existing implementation builds output by appending to a `String` with `output.push_str(...)`. Follow this same pattern — insert these blocks before the Boundaries block:

```rust
    // Scene Direction (from goal system)
    if let Some(ref direction) = preamble.scene_direction {
        output.push_str("\n## Scene Direction\n");
        output.push_str(&direction.dramatic_tension);
        output.push('\n');
        output.push_str(&direction.trajectory);
        output.push('\n');
    }

    // Character Drives (from goal system)
    for drive in &preamble.character_drives {
        output.push_str(&format!("\n## {}'s Drive\n", drive.name));
        output.push_str(&drive.objective);
        output.push(' ');
        output.push_str(&drive.constraint);
        output.push(' ');
        output.push_str(&drive.behavioral_stance);
        output.push('\n');
    }

    // Player Context (from goal system)
    if let Some(ref context) = preamble.player_context {
        output.push_str("\n## Player Context\n");
        output.push_str(context);
        output.push('\n');
    }
```

These must appear *before* the Boundaries section so that boundaries remain last in the rendered preamble.

- [ ] **Step 4: Update existing tests to include new default fields**

Any existing test that constructs a `PersistentPreamble` directly needs the new fields added with default values:

```rust
scene_direction: None,
character_drives: Vec::new(),
player_context: None,
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p storyteller-engine preamble -- --nocapture`

Expected: All preamble tests pass (both new and existing).

- [ ] **Step 6: Commit**

```bash
git add crates/storyteller-engine/src/context/preamble.rs
git commit -m "feat(preamble): render Scene Direction, Character Drives, and Player Context sections"
```

---

### Task 7: Implement composition-time intention generation

**Files:**
- Create: `crates/storyteller-engine/src/inference/intention_generation.rs`
- Modify: `crates/storyteller-engine/src/inference/mod.rs`

The single LLM call at scene setup that generates concrete situational intentions.

- [ ] **Step 1: Write the module**

Create `crates/storyteller-engine/src/inference/intention_generation.rs`:

```rust
//! Composition-time intention generation — transforms active goals + lexicon
//! fragments + scene context into concrete situational intentions via LLM.
//!
//! Called once at scene setup. Uses qwen2.5:14b-instruct (or configurable)
//! via the same Ollama infrastructure as intent synthesis.
//!
//! See: `docs/plans/2026-03-11-scene-goals-and-character-intentions-design.md`

use storyteller_core::traits::llm::{CompletionRequest, LlmProvider, Message, MessageRole};
use storyteller_core::types::character::{CharacterSheet, SceneData};
use storyteller_core::types::narrator_context::{CharacterDrive, SceneDirection};

use crate::scene_composer::goals::{CharacterGoal, ComposedGoals, SceneGoal};

/// Generated intentions ready for preamble injection.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GeneratedIntentions {
    pub scene_intention: SceneIntention,
    pub character_intentions: Vec<CharacterIntention>,
}

/// Scene-level dramatic intention.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SceneIntention {
    pub dramatic_tension: String,
    pub trajectory: String,
}

/// Per-character concrete intention.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CharacterIntention {
    pub character: String,
    pub objective: String,
    pub constraint: String,
    pub behavioral_stance: String,
}

/// Build the system prompt for intention generation.
pub fn intention_system_prompt() -> String {
    r#"You are a dramaturgical advisor for an interactive narrative engine. Your job is to generate concrete situational intentions for characters in a scene.

Rules:
- Each character's objective MUST reference physical objects, locations, or spatial relationships from the setting
- Objectives should create inter-character tension where characters' pursuits naturally complicate each other
- Constraints should arise from other characters' natural behavior, not arbitrary obstacles
- Behavioral stance describes HOW the character pursues their objective — manner, tactics, observable behavior
- The dramatic tension should describe the specific situation, not abstract themes
- The trajectory should describe the moment the scene is building toward

Respond with valid JSON matching this exact structure:
{
  "scene_intention": {
    "dramatic_tension": "1-3 sentences describing the specific dramatic situation",
    "trajectory": "1-2 sentences describing what moment the scene builds toward"
  },
  "character_intentions": [
    {
      "character": "Character Name",
      "objective": "What they are concretely trying to do, grounded in setting",
      "constraint": "What makes it hard — usually another character's behavior",
      "behavioral_stance": "How they pursue it — manner, tactics, observable behavior"
    }
  ]
}"#
    .to_string()
}

/// Build the user prompt from goals, fragments, and scene context.
pub fn build_intention_prompt(
    scene: &SceneData,
    characters: &[&CharacterSheet],
    composed_goals: &ComposedGoals,
) -> String {
    let mut prompt = String::new();

    // Scene context
    prompt.push_str("Scene Context:\n");
    prompt.push_str(&format!("- Setting: {}\n", scene.setting.description));
    if !scene.setting.affordances.is_empty() {
        prompt.push_str(&format!(
            "- Setting affordances: {}\n",
            scene.setting.affordances.join(", ")
        ));
    }
    if !scene.setting.sensory_details.is_empty() {
        prompt.push_str(&format!(
            "- Setting sensory details: {}\n",
            scene.setting.sensory_details.join(", ")
        ));
    }
    prompt.push('\n');

    // Scene goals
    if !composed_goals.scene_goals.is_empty() {
        prompt.push_str("Scene Goals:\n");
        for sg in &composed_goals.scene_goals {
            prompt.push_str(&format!("- {} ({:?})\n", sg.goal_id, sg.visibility));
            for f in &sg.fragments {
                prompt.push_str(&format!("  \"{}\"\n", f.text));
            }
        }
        prompt.push('\n');
    }

    // Cast and character goals
    // Note: CharacterSheet has no `role` field — look up role from scene.cast by entity_id
    prompt.push_str("Cast and Character Goals:\n");
    for character in characters {
        let entity_goals = composed_goals
            .character_goals
            .get(&character.entity_id);
        let role = scene
            .cast
            .iter()
            .find(|c| c.entity_id == character.entity_id)
            .map(|c| c.role.as_str())
            .unwrap_or("unknown");
        prompt.push_str(&format!(
            "- {} ({}): ",
            character.name, role
        ));
        if let Some(goals) = entity_goals {
            let goal_ids: Vec<&str> = goals.iter().map(|g| g.goal_id.as_str()).collect();
            prompt.push_str(&goal_ids.join(", "));
            prompt.push('\n');
            for g in goals {
                for f in &g.fragments {
                    prompt.push_str(&format!("  \"{}\"\n", f.text));
                }
            }
        } else {
            prompt.push_str("(no specific goals)\n");
        }
    }

    prompt
}

/// Generate concrete situational intentions via LLM.
///
/// Returns `None` on LLM failure (graceful degradation — scene proceeds without goals).
pub async fn generate_intentions(
    llm: &dyn LlmProvider,
    scene: &SceneData,
    characters: &[&CharacterSheet],
    composed_goals: &ComposedGoals,
) -> Option<GeneratedIntentions> {
    let system = intention_system_prompt();
    let user = build_intention_prompt(scene, characters, composed_goals);

    let request = CompletionRequest {
        system_prompt: system,
        messages: vec![Message {
            role: MessageRole::User,
            content: user,
        }],
        max_tokens: 800,
        temperature: 0.7,
    };

    let response = match llm.complete(request).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Intention generation LLM call failed: {e}");
            return None;
        }
    };

    parse_intentions(&response.content)
}

/// Parse the LLM JSON output into structured intentions.
fn parse_intentions(text: &str) -> Option<GeneratedIntentions> {
    // Try direct parse first
    if let Ok(intentions) = serde_json::from_str::<GeneratedIntentions>(text) {
        return Some(intentions);
    }

    // Try extracting JSON from markdown code blocks
    let json_str = extract_json_block(text)?;
    serde_json::from_str::<GeneratedIntentions>(json_str).ok()
}

/// Extract JSON from markdown code fences if present.
fn extract_json_block(text: &str) -> Option<&str> {
    let start = text.find('{');
    let end = text.rfind('}');
    match (start, end) {
        (Some(s), Some(e)) if s < e => Some(&text[s..=e]),
        _ => None,
    }
}

/// Convert generated intentions into preamble types.
pub fn intentions_to_preamble(
    intentions: &GeneratedIntentions,
) -> (SceneDirection, Vec<CharacterDrive>) {
    let direction = SceneDirection {
        dramatic_tension: intentions.scene_intention.dramatic_tension.clone(),
        trajectory: intentions.scene_intention.trajectory.clone(),
    };

    let drives = intentions
        .character_intentions
        .iter()
        .map(|ci| CharacterDrive {
            name: ci.character.clone(),
            objective: ci.objective.clone(),
            constraint: ci.constraint.clone(),
            behavioral_stance: ci.behavioral_stance.clone(),
        })
        .collect();

    (direction, drives)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_clean_json() {
        let json = r#"{
            "scene_intention": {
                "dramatic_tension": "Arthur came for a letter.",
                "trajectory": "Toward trust or betrayal."
            },
            "character_intentions": [
                {
                    "character": "Arthur",
                    "objective": "Get the letter.",
                    "constraint": "Margaret is near the mantel.",
                    "behavioral_stance": "Polite deflection."
                }
            ]
        }"#;

        let result = parse_intentions(json);
        assert!(result.is_some());
        let intentions = result.unwrap();
        assert_eq!(intentions.character_intentions.len(), 1);
        assert_eq!(intentions.character_intentions[0].character, "Arthur");
    }

    #[test]
    fn parse_json_from_code_block() {
        let text = "Here are the intentions:\n```json\n{\"scene_intention\":{\"dramatic_tension\":\"Test.\",\"trajectory\":\"Test.\"},\"character_intentions\":[]}\n```";
        let result = parse_intentions(text);
        assert!(result.is_some());
    }

    #[test]
    fn parse_garbage_returns_none() {
        let result = parse_intentions("This is not JSON at all");
        assert!(result.is_none());
    }

    #[test]
    fn intentions_convert_to_preamble_types() {
        let intentions = GeneratedIntentions {
            scene_intention: SceneIntention {
                dramatic_tension: "Test tension.".to_string(),
                trajectory: "Test trajectory.".to_string(),
            },
            character_intentions: vec![CharacterIntention {
                character: "Arthur".to_string(),
                objective: "Get the letter.".to_string(),
                constraint: "Margaret is near.".to_string(),
                behavioral_stance: "Polite deflection.".to_string(),
            }],
        };

        let (direction, drives) = intentions_to_preamble(&intentions);
        assert_eq!(direction.dramatic_tension, "Test tension.");
        assert_eq!(drives.len(), 1);
        assert_eq!(drives[0].name, "Arthur");
    }

    #[test]
    fn system_prompt_contains_json_schema() {
        let prompt = intention_system_prompt();
        assert!(prompt.contains("scene_intention"));
        assert!(prompt.contains("character_intentions"));
        assert!(prompt.contains("objective"));
        assert!(prompt.contains("constraint"));
        assert!(prompt.contains("behavioral_stance"));
    }

    #[test]
    fn user_prompt_includes_scene_and_goals() {
        use storyteller_core::types::character::*;
        use storyteller_core::types::entity::EntityId;
        use storyteller_core::types::scene::{SceneId, SceneType};

        let scene = SceneData {
            scene_id: SceneId::new(),
            title: "Test Scene".to_string(),
            scene_type: SceneType::Gravitational,
            setting: SceneSetting {
                description: "A quiet rectory".to_string(),
                affordances: vec!["Mantel".to_string(), "Tea caddy".to_string(), "Window".to_string()],
                sensory_details: vec!["Ticking clock".to_string(), "Lavender".to_string()],
                aesthetic_detail: "Faded wallpaper with roses".to_string(),
            },
            cast: Vec::new(),
            stakes: Vec::new(),
            constraints: SceneConstraints {
                hard: Vec::new(),
                soft: Vec::new(),
                perceptual: Vec::new(),
            },
            emotional_arc: Vec::new(),
            evaluation_criteria: Vec::new(),
        };

        let composed = ComposedGoals::default();
        let characters: Vec<&CharacterSheet> = Vec::new();

        let prompt = build_intention_prompt(&scene, &characters, &composed);
        assert!(prompt.contains("A quiet rectory"));
        assert!(prompt.contains("Mantel"));
    }
}
```

Note: The `SceneSetting` struct may not have `affordances` and `sensory_details` fields yet. Check the actual struct definition in `storyteller-core`. If they don't exist, use the existing `description` field and omit the affordance/sensory lines. The test fixtures should match the real types.

- [ ] **Step 2: Register the module**

In `crates/storyteller-engine/src/inference/mod.rs`, add:

```rust
pub mod intention_generation;
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p storyteller-engine intention_generation -- --nocapture`

Expected: All 6 tests pass. Fix any type mismatches between test fixtures and actual types.

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-engine/src/inference/intention_generation.rs crates/storyteller-engine/src/inference/mod.rs
git commit -m "feat(inference): add composition-time intention generation via LLM"
```

---

## Chunk 4: Workshop Integration and Session Persistence

### Task 8: Add goal persistence to session store

**Files:**
- Modify: `crates/storyteller-workshop/src-tauri/src/session.rs`

Add `save_goals()` and `load_goals()` to `SessionStore`.

- [ ] **Step 1: Write failing test**

Add to the test module in `session.rs`:

```rust
#[test]
fn goals_roundtrip_through_session() {
    let dir = tempfile::tempdir().unwrap();
    let store = SessionStore::new(dir.path()).unwrap();

    // Create a minimal session first
    let selections = test_selections(); // use existing test helper
    let scene = test_scene();           // use existing test helper
    let characters = vec![test_character()]; // use existing test helper
    let session_id = store.create_session(&selections, &scene, &characters).unwrap();

    let goals_json = serde_json::json!({
        "active_scene_goals": [{"goal_id": "protect_secret", "visibility": "Hidden", "selected_fragments": []}],
        "active_character_goals": {},
        "generated_intentions": {
            "scene_intention": {"dramatic_tension": "Test.", "trajectory": "Test."},
            "character_intentions": []
        }
    });

    store.save_goals(&session_id, &goals_json).unwrap();
    let loaded = store.load_goals(&session_id).unwrap();
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap()["active_scene_goals"][0]["goal_id"], "protect_secret");
}

#[test]
fn load_goals_returns_none_for_old_sessions() {
    let dir = tempfile::tempdir().unwrap();
    let store = SessionStore::new(dir.path()).unwrap();

    let selections = test_selections();
    let scene = test_scene();
    let characters = vec![test_character()];
    let session_id = store.create_session(&selections, &scene, &characters).unwrap();

    // Don't save goals — this is an old session
    let loaded = store.load_goals(&session_id).unwrap();
    assert!(loaded.is_none());
}
```

- [ ] **Step 2: Implement save_goals and load_goals**

Add to `SessionStore`:

```rust
/// Save composed goals to the session directory.
pub fn save_goals(&self, session_id: &str, goals: &serde_json::Value) -> Result<(), String> {
    let path = self.base_dir.join(session_id).join("goals.json");
    let json = serde_json::to_string_pretty(goals).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("Failed to write goals.json: {e}"))
}

/// Load composed goals from the session directory. Returns None if goals.json doesn't exist
/// (backward compatibility with pre-goal sessions).
pub fn load_goals(&self, session_id: &str) -> Result<Option<serde_json::Value>, String> {
    let path = self.base_dir.join(session_id).join("goals.json");
    if !path.exists() {
        return Ok(None);
    }
    let json = std::fs::read_to_string(&path).map_err(|e| format!("Failed to read goals.json: {e}"))?;
    let value: serde_json::Value = serde_json::from_str(&json).map_err(|e| format!("Failed to parse goals.json: {e}"))?;
    Ok(Some(value))
}
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p storyteller-workshop goals_roundtrip load_goals_returns_none -- --nocapture`

Expected: Both tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/storyteller-workshop/src-tauri/src/session.rs
git commit -m "feat(session): add goals.json persistence for scene goal data"
```

---

### Task 9: Wire goal system into compose_scene and setup_and_render_opening

**Files:**
- Modify: `crates/storyteller-engine/src/scene_composer/compose.rs`
- Modify: `crates/storyteller-workshop/src-tauri/src/commands.rs`

This is the integration task — connecting the goal intersection, likeness pass, intention generation, and preamble injection into the existing workshop pipeline.

- [ ] **Step 1: Add goal intersection to compose()**

In `compose.rs`, after the existing scene composition logic (around line 230), add goal intersection. The `compose()` function needs access to the goal vocabulary from `DescriptorSet`. Add a method on `SceneComposer` (in `catalog.rs`) rather than changing `compose()` directly:

In `catalog.rs`, add:

```rust
/// Run goal intersection for a composed scene.
pub fn intersect_goals(
    &self,
    selections: &SceneSelections,
    composed: &ComposedScene,
) -> ComposedGoals {
    use super::goals::{intersect_scene_goals, intersect_character_goals, CastMember};

    let profile = match self.find_profile(&selections.profile_id) {
        Some(p) => p,
        None => return ComposedGoals::default(),
    };

    // Build cast members with their archetypes and dynamics
    let cast_members: Vec<CastMember> = composed
        .characters
        .iter()
        .enumerate()
        .map(|(i, character)| {
            let archetype = self
                .find_archetype(&selections.cast[i].archetype_id)
                .cloned()
                .unwrap_or_else(|| panic!("archetype {} should exist", selections.cast[i].archetype_id));

            let dynamics: Vec<_> = selections
                .dynamics
                .iter()
                .filter(|d| d.cast_index_a == i || d.cast_index_b == i)
                .filter_map(|d| self.find_dynamic(&d.dynamic_id).cloned())
                .collect();

            CastMember {
                entity_id: character.entity_id,
                archetype,
                dynamics,
            }
        })
        .collect();

    let scene_goals = intersect_scene_goals(profile, &cast_members, &self.descriptors.goals);

    let mut character_goals = std::collections::HashMap::new();
    for member in &cast_members {
        let goals = intersect_character_goals(member, &scene_goals, &self.descriptors.goals);
        if !goals.is_empty() {
            character_goals.insert(member.entity_id, goals);
        }
    }

    ComposedGoals {
        scene_goals,
        character_goals,
    }
}
```

- [ ] **Step 2: Wire into compose_scene command**

In `commands.rs`, in the `compose_scene` function (around line 246-268), after the existing `composer.compose(&selections)` call:

```rust
// Goal intersection
let mut composed_goals = composer.intersect_goals(&selections, &composed);

// Likeness pass (populate fragments)
use storyteller_engine::scene_composer::likeness::{
    LikenessContext, populate_scene_goal_fragments, populate_character_goal_fragments,
};
let likeness_ctx = LikenessContext {
    genre_id: &selections.genre_id,
    profile_id: &selections.profile_id,
    archetype_ids: selections.cast.iter().map(|c| c.archetype_id.as_str()).collect(),
    dynamic_ids: selections.dynamics.iter().map(|d| d.dynamic_id.as_str()).collect(),
};
let mut rng = rand::rng();
populate_scene_goal_fragments(
    &mut composed_goals.scene_goals,
    composer.goal_defs(),
    &likeness_ctx,
    &mut rng,
);
for goals in composed_goals.character_goals.values_mut() {
    populate_character_goal_fragments(
        goals,
        composer.goal_defs(),
        &likeness_ctx,
        &mut rng,
    );
}
```

Note: The `SceneComposer`'s `descriptors` field is `pub(crate)` and not accessible from `storyteller-workshop`. Add a public accessor to `SceneComposer` in `catalog.rs`:

```rust
/// Access the goal definitions from the descriptor set.
pub fn goal_defs(&self) -> &[Goal] {
    &self.descriptors.goals
}
```

Then use `composer.goal_defs()` instead of `composer.goal_defs()` in `commands.rs`.

- [ ] **Step 3: Call intention generation LLM**

In `setup_and_render_opening()`, after the LLM providers are created (around line 1011) but before context assembly (around line 1084), add the intention generation call:

```rust
// Intention generation (composition-time LLM call)
let generated_intentions = if !composed_goals.scene_goals.is_empty() {
    // Create a dedicated LLM provider for intention generation
    // Use the narrator model (14b) since this needs richer output than 3b
    use storyteller_engine::inference::intention_generation::generate_intentions;
    let characters_refs: Vec<&CharacterSheet> = characters.iter().collect();
    generate_intentions(
        narrator_llm.as_ref(),
        &scene,
        &characters_refs,
        &composed_goals,
    )
    .await
} else {
    None
};
```

Note: `composed_goals` will need to be passed into `setup_and_render_opening()` as a parameter. Update the function signature accordingly.

- [ ] **Step 4: Inject into preamble**

Where `build_preamble()` is called in `setup_and_render_opening()`, pass the generated intentions:

```rust
use storyteller_engine::inference::intention_generation::intentions_to_preamble;

let (scene_direction, character_drives) = match &generated_intentions {
    Some(intentions) => {
        let (dir, drives) = intentions_to_preamble(intentions);
        (Some(dir), drives)
    }
    None => (None, Vec::new()),
};

// Update the preamble after build_preamble() returns:
preamble.scene_direction = scene_direction;
preamble.character_drives = character_drives;
// Player context based on player's character goals
preamble.player_context = composed_goals
    .character_goals
    .get(&player_entity_id.unwrap_or_default())
    .map(|goals| {
        goals
            .iter()
            .filter(|g| matches!(g.visibility, GoalVisibility::Overt | GoalVisibility::Signaled))
            .map(|g| g.goal_id.replace('_', " "))
            .collect::<Vec<_>>()
            .join("; ")
    })
    .filter(|s| !s.is_empty());
```

- [ ] **Step 5: Persist goals to session**

After the session is created in `compose_scene`, persist the goals:

```rust
let goals_json = serde_json::to_value(&composed_goals).map_err(|e| e.to_string())?;
session_store.save_goals(&session_id, &goals_json).map_err(|e| e.to_string())?;
```

- [ ] **Step 6: Load goals on session resume**

In `resume_session()`, after loading scene data, load goals:

```rust
let loaded_goals = session_store.load_goals(&session_id).map_err(|e| e.to_string())?;
let composed_goals: ComposedGoals = loaded_goals
    .and_then(|v| serde_json::from_value(v).ok())
    .unwrap_or_default();
```

Pass `composed_goals` into `setup_and_render_opening()`.

- [ ] **Step 7: Verify compilation**

Run: `cargo check -p storyteller-workshop --all-features`

Expected: Clean compilation. This task involves many integration points — fix any type mismatches. The key types that must align:
- `ComposedGoals` serializes/deserializes via serde
- `build_preamble()` return type already has the new fields (from Task 5)
- `setup_and_render_opening()` signature now accepts `ComposedGoals`

- [ ] **Step 8: Commit**

```bash
git add crates/storyteller-engine/src/scene_composer/catalog.rs crates/storyteller-engine/src/scene_composer/compose.rs crates/storyteller-workshop/src-tauri/src/commands.rs
git commit -m "feat(workshop): wire goal intersection, intention generation, and preamble injection into scene setup"
```

---

## Chunk 5: Verification and Build-Time Pipeline

### Task 10: Run full verification suite

**Files:** None (verification only)

- [ ] **Step 1: Run clippy**

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`

Expected: No warnings. Fix any issues.

- [ ] **Step 2: Run format check**

Run: `cargo fmt --check`

Expected: No formatting issues. Run `cargo fmt` if needed.

- [ ] **Step 3: Run workspace tests**

Run: `cargo test --workspace --all-features`

Expected: All existing tests pass plus new tests. The `with_model` tests that require `STORYTELLER_MODEL_PATH` will continue to fail as expected.

- [ ] **Step 4: Commit any fixes**

```bash
git add -A
git commit -m "fix: address clippy and test issues from goal system integration"
```

---

### Task 11: Create Python build-time lexicon enrichment pipeline

**Files:**
- Create: `tools/training/src/goal_lexicon/__init__.py`
- Create: `tools/training/src/goal_lexicon/enrich.py`
- Create: `tools/training/src/goal_lexicon/cli.py`
- Create: `tools/training/tests/test_goal_lexicon.py`

This task creates the build-time tool that generates behavioral lexicons for goals.

- [ ] **Step 1: Create the enrichment module**

Create `tools/training/src/goal_lexicon/__init__.py`:

```python
"""Build-time lexicon enrichment for scene goals."""
```

Create `tools/training/src/goal_lexicon/enrich.py`:

```python
"""Generate behavioral lexicons for goals via LLM (Ollama)."""

import json
from pathlib import Path
from typing import Any

import httpx


def load_descriptors(descriptor_dir: Path) -> dict[str, Any]:
    """Load all descriptor files from the given directory."""
    result = {}
    for name in ["goals", "profiles", "archetypes", "dynamics", "genres"]:
        path = descriptor_dir / f"{name}.json"
        if path.exists():
            with open(path) as f:
                result[name] = json.load(f)
    return result


def build_enrichment_prompt(
    goal: dict, profiles: list[dict], archetypes: list[dict], dynamics: list[dict]
) -> str:
    """Build the prompt for generating behavioral lexicon entries for a goal."""
    prompt = f"""Generate behavioral lexicon entries for this narrative goal.

Goal: {goal['id']}
Description: {goal['description']}
Category: {goal['category']}
Visibility: {goal['visibility']}
Valence: {goal['valence']}

Relevant profiles (scene types where this goal appears):
"""
    for p in profiles:
        prompt += f"- {p['id']}: {p['description']}\n"

    prompt += "\nRelevant archetypes (character types who can pursue this goal):\n"
    for a in archetypes:
        prompt += f"- {a['id']}: {a['description']}\n"

    prompt += "\nRelevant dynamics (relationships that enable this goal):\n"
    for d in dynamics:
        prompt += f"- {d['id']}: {d['description']}\n"

    prompt += """
Generate 15-25 behavioral lexicon entries. Each entry describes what pursuing this goal LOOKS LIKE — observable behavior, speech patterns, physical tells, relational moves. NOT abstract atmosphere.

For each entry, specify:
- fragment: the behavioral description (1-2 sentences)
- register: "character_signal" (primary), "atmospheric", or "transitional"
- dimensional_context: which archetypes/profiles/dynamics this fragment fits best (use null for wildcard)

Respond with valid JSON:
{
  "entries": [
    {
      "fragment": "...",
      "register": "character_signal",
      "dimensional_context": {
        "archetypes": ["archetype_id"] or null,
        "profiles": ["profile_id"] or null,
        "dynamics": ["dynamic_id"] or null,
        "valence": ["heavy", "tense"]
      }
    }
  ]
}"""
    return prompt


def enrich_goal(
    goal: dict,
    descriptors: dict[str, Any],
    model: str = "qwen2.5:32b-instruct",
    base_url: str = "http://localhost:11434",
) -> list[dict]:
    """Generate lexicon entries for a single goal via Ollama."""
    goals_data = descriptors.get("goals", {}).get("goals", [])
    profiles = descriptors.get("profiles", {}).get("profiles", [])
    archetypes = descriptors.get("archetypes", {}).get("archetypes", [])
    dynamics = descriptors.get("dynamics", {}).get("dynamics", [])

    # Find relevant descriptors that reference this goal
    relevant_profiles = [
        p for p in profiles if goal["id"] in p.get("scene_goals", [])
    ]
    relevant_archetypes = [
        a for a in archetypes if goal["id"] in a.get("pursuable_goals", [])
    ]
    relevant_dynamics = [
        d for d in dynamics if goal["id"] in d.get("enabled_goals", [])
    ]

    prompt = build_enrichment_prompt(
        goal, relevant_profiles, relevant_archetypes, relevant_dynamics
    )

    response = httpx.post(
        f"{base_url}/api/generate",
        json={
            "model": model,
            "prompt": prompt,
            "stream": False,
            "options": {"temperature": 0.8, "num_predict": 2000},
        },
        timeout=120.0,
    )
    response.raise_for_status()

    text = response.json()["response"]

    # Parse JSON from response
    try:
        start = text.index("{")
        end = text.rindex("}") + 1
        data = json.loads(text[start:end])
        return data.get("entries", [])
    except (ValueError, json.JSONDecodeError):
        print(f"  Warning: failed to parse LLM output for {goal['id']}")
        return []


def enrich_all_goals(
    descriptor_dir: Path,
    model: str = "qwen2.5:32b-instruct",
    base_url: str = "http://localhost:11434",
) -> None:
    """Enrich all goals in goals.json with behavioral lexicons."""
    descriptors = load_descriptors(descriptor_dir)
    goals_path = descriptor_dir / "goals.json"

    with open(goals_path) as f:
        goals_data = json.load(f)

    for goal in goals_data["goals"]:
        print(f"Enriching: {goal['id']}...")
        entries = enrich_goal(goal, descriptors, model, base_url)
        goal["lexicon"] = entries
        print(f"  Generated {len(entries)} entries")

    with open(goals_path, "w") as f:
        json.dump(goals_data, f, indent=2)

    print(f"\nDone. Enriched {len(goals_data['goals'])} goals.")
```

- [ ] **Step 2: Create CLI entry point**

Create `tools/training/src/goal_lexicon/cli.py`:

```python
"""CLI for goal lexicon enrichment."""

import argparse
from pathlib import Path

from .enrich import enrich_all_goals


def main():
    parser = argparse.ArgumentParser(description="Enrich goal vocabulary with behavioral lexicons")
    parser.add_argument(
        "descriptor_dir",
        type=Path,
        help="Path to training-data/descriptors directory",
    )
    parser.add_argument(
        "--model",
        default="qwen2.5:32b-instruct",
        help="Ollama model for enrichment (default: qwen2.5:32b-instruct)",
    )
    parser.add_argument(
        "--base-url",
        default="http://localhost:11434",
        help="Ollama base URL",
    )
    args = parser.parse_args()

    enrich_all_goals(args.descriptor_dir, args.model, args.base_url)


if __name__ == "__main__":
    main()
```

- [ ] **Step 3: Write tests**

Create `tools/training/tests/test_goal_lexicon.py`:

```python
"""Tests for goal lexicon enrichment."""

from goal_lexicon.enrich import build_enrichment_prompt, load_descriptors


def test_build_prompt_includes_goal_info():
    goal = {
        "id": "protect_secret",
        "description": "Keep information hidden",
        "category": "protection",
        "visibility": "Hidden",
        "valence": "tense",
    }
    prompt = build_enrichment_prompt(goal, [], [], [])
    assert "protect_secret" in prompt
    assert "protection" in prompt
    assert "behavioral lexicon" in prompt.lower()


def test_build_prompt_includes_relevant_descriptors():
    goal = {
        "id": "protect_secret",
        "description": "Keep information hidden",
        "category": "protection",
        "visibility": "Hidden",
        "valence": "tense",
    }
    profiles = [{"id": "quiet_reunion", "description": "A gentle meeting"}]
    archetypes = [{"id": "stoic_survivor", "description": "Endures without complaint"}]
    dynamics = [{"id": "strangers_in_shared_grief", "description": "Bound by loss"}]

    prompt = build_enrichment_prompt(goal, profiles, archetypes, dynamics)
    assert "quiet_reunion" in prompt
    assert "stoic_survivor" in prompt
    assert "strangers_in_shared_grief" in prompt
```

- [ ] **Step 4: Register in pyproject.toml**

Check `tools/training/pyproject.toml` for how packages are registered. Add `goal_lexicon` as a package and optionally add an entry point for `uv run enrich-goals`.

- [ ] **Step 5: Run Python tests**

Run: `cd tools/training && uv run pytest tests/test_goal_lexicon.py -v`

Expected: Both tests pass.

- [ ] **Step 6: Commit**

```bash
git add tools/training/src/goal_lexicon/ tools/training/tests/test_goal_lexicon.py
git commit -m "feat(tools): add Python build-time lexicon enrichment pipeline for goals"
```

---

### Task 12: Manual playtest verification

**Files:** None (playtest only)

- [ ] **Step 1: Run the enrichment pipeline**

Run: `cd tools/training && uv run python -m goal_lexicon.cli /path/to/storyteller-data/training-data/descriptors --model qwen2.5:14b-instruct`

This populates the lexicon arrays in `goals.json`. Verify the output looks like behavioral vocabulary, not abstract atmosphere.

- [ ] **Step 2: Start the workshop and compose a scene**

Run: `cargo make workshop` (or the appropriate command to start the Tauri workshop)

Compose a Cozy Ghost Story scene with Quiet Reunion profile, two characters with a dynamic. Observe:
- Does the scene setup take longer (the intention generation LLM call)?
- Does the opening narration reference the generated intentions?
- Do characters show goal-directed behavior?

- [ ] **Step 3: Play 5-7 turns**

Play the scene through several turns. Compare against the narrator drift observed in the turns.jsonl analysis:
- Does the narrator maintain direction past Turn 3?
- Do characters pursue concrete objectives (not just react)?
- Does inter-character tension arise from conflicting goals?

- [ ] **Step 4: Inspect the session goals.json**

Read `.story/sessions/{uuid}/goals.json` and verify:
- Scene goals were intersected correctly
- Character goals were assigned
- Generated intentions are grounded in the setting
- Fragments were selected from the lexicon

- [ ] **Step 5: Test session resume with goals**

Stop the workshop, restart, and resume the session. Verify:
- Goals are loaded from `goals.json`
- The narrator preamble still contains scene direction and character drives
- Play continues with goal-directed behavior

- [ ] **Step 6: Document findings**

Note what worked, what didn't, and what needs tuning. Likely areas:
- LLM prompt quality for intention generation
- Fragment selection diversity
- Token budget impact on narrator context
- Whether 14b is sufficient or 32b is needed
