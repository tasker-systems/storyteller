# Narrative Pacing: Scene Purpose, Density, and the Sparsity Problem

## Purpose

This document defines the conceptual model for narrative pacing that underlies scene generation. It answers three questions:

1. **What does a scene do?** — A typology of narrative purposes, modeled as weighted compositions rather than single labels.
2. **What does well-paced narrative look like?** — A configurable pacing grammar that defines expected rhythms for a given narrative mode.
3. **When is the graph sparse?** — How to compute narrative density and identify gaps that need filling.

The pacing model operates at two levels: **static graph analysis** (evaluating the authored narrative topology for structural gaps) and **runtime session analysis** (evaluating a specific player's trajectory for gaps that matter *now*). Both use the same pacing grammar; they differ in what dimensions they consider.

**Prerequisites**: [`authoring-economics.md`](authoring-economics.md), [`scene-provenance.md`](scene-provenance.md).

**Stance**: This document takes an opinionated position grounded in low fantasy as the reference narrative mode. The model is designed to be pluggable — different genres install different pacing grammars — but the first implementation is unapologetically specific.

---

## Scene Purpose as Weighted Composition

### The Problem with Single Labels

The current `:Scene` vertex has a `scene_type` property: `gravitational`, `connective`, `gate`, `threshold`. This is a structural classification — it tells us about the scene's role in the graph topology. But it doesn't capture what the scene *does* narratively.

More importantly, well-written scenes don't serve a single purpose. A connective scene that merely describes a walk is thin. A connective scene where two characters bond during the walk, where the landscape reveals something about the world's history, where the physical threshold from village to wilderness mirrors an emotional threshold — that scene is rich because it serves multiple purposes simultaneously.

Scene purpose should be a **weighted vector**, not a single label.

### The Purpose Dimensions

A scene's narrative purpose is a composition across these dimensions:

| Dimension | What It Means | Example |
|-----------|--------------|---------|
| **Gravitational** | Advances the central plot; resolves or creates major narrative tensions | Sarah confronts Adam about the contradictory orders |
| **Connective** | Provides spatial, temporal, or contextual transition between significant moments | Walking from the village to the deep wood |
| **Developmental** | Deepens character or relationship; reveals internal state; grows a bond or fracture | Sarah and Kate's conversation about courage before departure |
| **World-Revealing** | Shows how the world works — its rules, history, social structures, physical realities | The village elder explaining why no one enters the wood |
| **Reflective** | Processes what has happened; allows characters and player to absorb significance | Sarah alone at the campfire after Adam's dwelling |
| **Escalatory** | Raises tension without resolving it; introduces new stakes or complications | Signs of the Wolf's presence along the path |
| **Threshold** | Marks an irreversible boundary crossing — spatial, temporal, emotional, or narrative | Kate's farewell at the edge of the wood |

### Purpose Vector

Each scene carries a purpose vector — weights summing to 1.0 that describe how much of each dimension the scene serves:

```rust
struct ScenePurpose {
    gravitational: f32,
    connective: f32,
    developmental: f32,
    world_revealing: f32,
    reflective: f32,
    escalatory: f32,
    threshold: f32,
}
```

**Examples from TFATD:**

| Scene | Grav | Conn | Dev | World | Refl | Esc | Thresh | Description |
|-------|------|------|-----|-------|------|-----|--------|-------------|
| S1: Tom Lies Still | 0.5 | 0.0 | 0.3 | 0.0 | 0.0 | 0.2 | 0.0 | High gravity (dying boy), developmental (family bonds), escalatory (stakes established) |
| S3: Mother's Prayer | 0.3 | 0.0 | 0.3 | 0.1 | 0.0 | 0.0 | 0.3 | Gravitational (blessing quest), developmental (mother-daughter), threshold (departure) |
| Campfire scene (hypothetical) | 0.0 | 0.2 | 0.3 | 0.1 | 0.4 | 0.0 | 0.0 | Primarily reflective, with developmental and world-revealing texture |
| Path to the Wood | 0.0 | 0.4 | 0.2 | 0.2 | 0.0 | 0.2 | 0.0 | Primarily connective, with world-revealing landscape and escalatory tension |

The purpose vector is **authored for Tier 1 scenes** (the designer knows what the scene is for), **computed for Tier 2 scenes** (derived from the gap the scene is filling), and **computed from session context for Tier 3 scenes** (derived from what the player needs).

### Relationship to `scene_type`

The existing `scene_type` (`gravitational`, `connective`, `gate`, `threshold`) is a structural classification that remains useful for graph traversal queries — filtering for gate scenes ahead, finding thresholds. It answers "what is this scene's role in the graph?" The purpose vector answers "what does this scene accomplish narratively?" They are complementary, not redundant.

A `gate` scene (structural) might have a purpose vector that is 0.5 gravitational, 0.3 threshold, 0.2 escalatory. A `connective` scene (structural) might have a purpose vector that is 0.4 connective, 0.3 developmental, 0.2 reflective, 0.1 world-revealing.

---

## The Pacing Grammar

### Definition

A pacing grammar is a configurable model that describes the expected rhythm of scene purposes for a given narrative mode. It defines:

1. **Purpose density expectations** — between gravitational scenes, what distribution of purposes should be present?
2. **Sequential constraints** — what purpose patterns feel right? (Don't follow escalation with escalation with escalation without relief.)
3. **Temporal expectations** — how much narrative-chronological time should typically pass between scene types?
4. **Minimum path richness** — for a path of N hops between two gravitational scenes, what purposes must be represented?

### Grammar Structure

```rust
struct PacingGrammar {
    /// Human-readable name
    name: String,
    /// Genre/mode this grammar applies to
    mode: NarrativeMode,  // e.g., LowFantasy, Thriller, LiteraryFiction

    /// Purpose density: expected cumulative purpose weight
    /// along any path of N hops between gravitational scenes
    density_expectations: DensityExpectations,

    /// Sequential constraints: patterns to encourage or avoid
    sequential_rules: Vec<SequentialRule>,

    /// Temporal pacing: expected narrative-chronological rhythm
    temporal_rhythm: TemporalRhythm,
}
```

### Density Expectations

For a path of N hops between two gravitational scenes, the pacing grammar specifies the minimum cumulative purpose weight that should be present:

```rust
struct DensityExpectations {
    /// Minimum cumulative developmental weight along any
    /// path of 3+ hops between gravitational scenes
    min_developmental: f32,     // default: 0.3 for low fantasy
    /// Minimum cumulative reflective weight after any
    /// scene with gravitational > 0.4
    min_reflective_after_gravity: f32,  // default: 0.2 for low fantasy
    /// Minimum cumulative world-revealing weight in the
    /// first third of the story
    min_world_revealing_early: f32,     // default: 0.4 for low fantasy
    /// Maximum consecutive escalatory weight without
    /// reflective or developmental relief
    max_escalatory_without_relief: f32, // default: 0.8 for low fantasy
}
```

**How to read this**: "In a low fantasy narrative, any path of 3+ hops between gravitational scenes should have at least 0.3 cumulative developmental weight. After any scene that's more than 40% gravitational, at least 0.2 reflective weight should follow before the next gravitational scene."

### Sequential Rules

Patterns the grammar encourages or discourages:

```rust
enum SequentialRule {
    /// After a scene with high weight in `trigger`, the next N scenes
    /// should have at least `min_weight` in `response`
    RequireAfter {
        trigger: PurposeDimension,
        trigger_threshold: f32,
        response: PurposeDimension,
        min_weight: f32,
        within_hops: usize,
    },
    /// Avoid sequences where `dimension` exceeds `threshold`
    /// for more than `max_consecutive` scenes
    LimitConsecutive {
        dimension: PurposeDimension,
        threshold: f32,
        max_consecutive: usize,
    },
}
```

**Low fantasy sequential rules** (opinionated first pass):

| Rule | Meaning |
|------|---------|
| RequireAfter(Gravitational > 0.4, Reflective ≥ 0.2, within 2) | After pivotal moments, allow space to process |
| RequireAfter(Threshold > 0.3, WorldRevealing ≥ 0.1, within 2) | After crossing a boundary, show the new territory |
| LimitConsecutive(Escalatory > 0.3, max 2) | No more than 2 escalatory scenes without relief |
| LimitConsecutive(Connective > 0.4, max 2) | No more than 2 primarily-connective scenes in a row |
| RequireAfter(Developmental > 0.4, any non-Developmental, within 1) | Don't pile development without action |

### Temporal Rhythm

Expected narrative-chronological pacing:

```rust
struct TemporalRhythm {
    /// Expected narrative time between scenes (range, in story time units)
    typical_gap: (f32, f32),
    /// Maximum narrative time that can pass without a scene
    /// (beyond this, a time-bridge scene is expected)
    max_gap_without_scene: f32,
    /// Minimum narrative time between gravitational scenes
    /// (story needs breathing room)
    min_time_between_gravitational: f32,
}
```

**Low fantasy defaults**: Typical gap 0.5-2.0 days. Max gap without a scene: 7 days. Min time between gravitational scenes: 0.5 days (you can't have two climactic confrontations in the same afternoon — the story needs to breathe).

---

## Computing Narrative Density

### The Density Metric

Narrative density along a path is the cumulative purpose vector, normalized by hop count:

```rust
fn path_density(scenes: &[ScenePurpose]) -> ScenePurpose {
    let sum = scenes.iter().fold(ScenePurpose::zero(), |acc, s| acc + s);
    sum / scenes.len() as f32
}
```

But raw density isn't enough — we need to compare against the grammar's expectations. A path is **sparse** when it fails one or more density expectations:

```rust
struct SparsityReport {
    path: Vec<SceneId>,
    /// Which density expectations are unmet
    unmet_expectations: Vec<UnmetExpectation>,
    /// Which sequential rules are violated
    violated_rules: Vec<ViolatedRule>,
    /// Temporal gaps that exceed grammar limits
    temporal_gaps: Vec<TemporalGap>,
}

struct UnmetExpectation {
    dimension: PurposeDimension,
    expected: f32,
    actual: f32,
    deficit: f32,  // how much purpose weight is missing
}
```

### Static Analysis (Tier 2: Collaborative Generation)

Static analysis examines the authored graph without session context. It answers: "Is this story well-paced *structurally*?"

**Inputs**:
- The full narrative graph (all `:Scene` vertices and `:TRANSITIONS_TO` edges)
- Purpose vectors for all authored scenes
- The pacing grammar for this story's narrative mode
- Narrative-chronological metadata (time settings on scenes)
- Event DAG structure (gating features — what events must occur)

**Algorithm**:

1. **Enumerate paths between gravitational scenes** — for each pair of gravitational scenes connected by a path of 2+ hops, extract the intermediate scenes
2. **Compute density along each path** — cumulative purpose vectors
3. **Check density expectations** — compare against grammar minimums
4. **Check sequential rules** — scan for violated patterns
5. **Check temporal rhythm** — identify chronological gaps that exceed limits
6. **Check gating coherence** — are there event gates along the path that require specific scene content (information passing, relationship changes) that no existing scene provides?

**Output**: A sparsity report per path, identifying which dimensions are underrepresented and what kind of scene would address the gap.

**When it runs**: During story development (author tooling) or at story-load time (pre-populating Tier 2 scenes). Not in the gameplay loop.

### Runtime Analysis (Tier 3: Runtime Generation)

Runtime analysis examines the player's specific position in the narrative and evaluates whether the topology ahead is adequate for *this player's* state. It considers everything static analysis does, plus session-specific dimensions:

**Additional inputs**:
- **Entity attention features** — which entities has the player focused on recently? Which relationships are active vs dormant?
- **Recent session events** — what has happened in the last N turns? What emotional register has the narrative been in?
- **Relational dynamics** — which relationships need development to make upcoming gravitational scenes accessible? (A gate scene requiring trust ≥ 0.7 between Sarah and Adam, but the relationship is at 0.3 — developmental scenes are needed)
- **Approach vector gaps** — which approach predicates for nearby gravitational scenes are unsatisfied, and what scene content could help satisfy them?

**The key difference**: Static analysis asks "is this path well-paced?" Runtime analysis asks "is this path well-paced *for this player given their current state*?"

A path that is structurally adequate (sufficient developmental scenes exist) might be inadequate for a specific player because the developmental scenes involve characters the player hasn't engaged with, or because the player's relational state requires a specific kind of development that the existing scenes don't provide.

**When it runs**: At scene exit, as part of the scene generation pipeline (see [`scene-provenance.md`](scene-provenance.md)).

---

## What a Generated Scene Specification Looks Like

When analysis identifies a gap, the output is a **scene specification** — not the scene content itself, but the constraints that define what the generated scene needs to accomplish:

```rust
struct SceneSpecification {
    /// Where in the graph this scene should be inserted
    insertion: InsertionPoint,
    /// The purpose vector this scene should target
    target_purpose: ScenePurpose,
    /// Setting constraints (should be at or near this setting)
    setting_hint: Option<SettingId>,
    /// Cast constraints (these entities should be present)
    required_cast: Vec<EntityId>,
    /// Suggested cast (would benefit from presence but not required)
    suggested_cast: Vec<EntityId>,
    /// Relational goals (these relationships should have opportunity to develop)
    relational_goals: Vec<RelationalGoal>,
    /// Narrative mass (how gravitationally significant this scene should be)
    target_mass: f32,
    /// Narrative-chronological placement
    temporal_hint: Option<NarrativeTimeRange>,
    /// What triggered this specification
    provenance: GenerationTrigger,
}

struct InsertionPoint {
    /// Scene this new scene follows
    after_scene: SceneId,
    /// Scene this new scene leads to
    before_scene: SceneId,
    /// Whether the direct edge should be retained
    retain_direct_edge: bool,
}

struct RelationalGoal {
    entity_a: EntityId,
    entity_b: EntityId,
    /// Which substrate dimension needs movement
    target_dimension: Option<SubstrateDimension>,
    /// Direction of needed movement
    direction: RelationalDirection,  // Strengthen, Weaken, Reveal, Complicate
}

enum GenerationTrigger {
    /// Static analysis identified structural gap
    StaticGap { path: Vec<SceneId>, unmet: Vec<UnmetExpectation> },
    /// Runtime analysis identified session-specific need
    RuntimeNeed { session_context: SessionAnalysisContext },
}
```

The scene specification is what gets handed to the generation pipeline — whether that's a template system, an LLM-assisted authoring process, or a Storykeeper-driven scene construction algorithm. The specification defines the *shape* of the hole; the generation pipeline fills it.

---

## The Low Fantasy Pacing Grammar

### Reference Implementation

Low fantasy as a genre is characterized by: grounded worlds where magic is rare or constrained, character-driven narratives, consequences that accumulate over time, physical journeys that mirror psychological ones, and stories that breathe — that allow space for the weight of events to be felt.

```rust
fn low_fantasy_grammar() -> PacingGrammar {
    PacingGrammar {
        name: "Low Fantasy".to_string(),
        mode: NarrativeMode::LowFantasy,

        density_expectations: DensityExpectations {
            min_developmental: 0.3,
            min_reflective_after_gravity: 0.2,
            min_world_revealing_early: 0.4,
            max_escalatory_without_relief: 0.8,
        },

        sequential_rules: vec![
            // After pivotal scenes, breathe
            SequentialRule::RequireAfter {
                trigger: PurposeDimension::Gravitational,
                trigger_threshold: 0.4,
                response: PurposeDimension::Reflective,
                min_weight: 0.2,
                within_hops: 2,
            },
            // After boundary crossings, show the world
            SequentialRule::RequireAfter {
                trigger: PurposeDimension::Threshold,
                trigger_threshold: 0.3,
                response: PurposeDimension::WorldRevealing,
                min_weight: 0.1,
                within_hops: 2,
            },
            // Don't escalate endlessly
            SequentialRule::LimitConsecutive {
                dimension: PurposeDimension::Escalatory,
                threshold: 0.3,
                max_consecutive: 2,
            },
            // Don't trudge — connective scenes need texture
            SequentialRule::LimitConsecutive {
                dimension: PurposeDimension::Connective,
                threshold: 0.4,
                max_consecutive: 2,
            },
        ],

        temporal_rhythm: TemporalRhythm {
            typical_gap: (0.5, 2.0),     // half-day to 2 days
            max_gap_without_scene: 7.0,   // a week max
            min_time_between_gravitational: 0.5,
        },
    }
}
```

### TFATD Validation

Apply the low fantasy grammar to the TFATD scene sequence to see if it identifies known pacing:

| Path | Density Check | Sequential Check | Result |
|------|--------------|-----------------|--------|
| S1 (Tom Lies Still) → S3 (Mother's Prayer) | 2-hop path, both high gravity | RequireAfter(Grav, Reflective) — no reflective scene between | **Gap identified**: needs a reflective beat between S1 and S3 |
| S3 (Mother's Prayer) → S4 (Adam's Dwelling) | Direct transition with threshold crossing | RequireAfter(Threshold, WorldRevealing) — no world-revealing content between | **Gap identified**: the journey from village to Adam should reveal the landscape |
| S2 (Adam's Dwelling) → S5 (The Deep Wood) | Long path with significant tonal shift | Density check passes if connective scenes exist | Depends on intermediate scene content |

The grammar correctly identifies that TFATD — which was designed as a dense authored story — would benefit from interstitial scenes between its gravitational peaks. In a fully authored story, those beats exist inside the scenes themselves (the writer handles pacing through prose). In a system-generated topology, they need to be separate scene containers.

---

## Contrasting Grammars (Future Work)

Different narrative modes would install different grammars. Sketches for contrast:

### Thriller

```
density_expectations:
    min_developmental: 0.15         # much lower — characters revealed through action
    min_reflective_after_gravity: 0.05  # almost no breathing room
    max_escalatory_without_relief: 1.5  # escalation is the mode

sequential_rules:
    LimitConsecutive(Reflective > 0.3, max 1)  # don't linger
    RequireAfter(Gravitational > 0.5, Escalatory, within 1)  # climax → complication

temporal_rhythm:
    typical_gap: (0.01, 0.5)    # minutes to hours, not days
    max_gap_without_scene: 1.0   # a day at most
    min_time_between_gravitational: 0.05  # climaxes can come fast
```

### Literary Fiction

```
density_expectations:
    min_developmental: 0.5          # character is the story
    min_reflective_after_gravity: 0.4  # everything is processed deeply
    min_world_revealing_early: 0.2  # world emerges through character, not exposition

sequential_rules:
    RequireAfter(any > 0.3, Reflective ≥ 0.3, within 1)  # always reflect
    LimitConsecutive(Escalatory > 0.2, max 1)  # tension is quiet

temporal_rhythm:
    typical_gap: (0.1, 30.0)    # scenes may be moments apart or years apart
    max_gap_without_scene: 365.0  # time jumps are a feature
    min_time_between_gravitational: 1.0  # need distance for significance
```

These are illustrative, not designs. Each would require the same depth of treatment we've given low fantasy.

---

## Storage and Configuration

### Where the Pacing Grammar Lives

The pacing grammar is a **story-level configuration** — each story declares its narrative mode, which selects a grammar:

```sql
-- In stories.story_config JSONB:
{
    "narrative_mode": "low_fantasy",
    "pacing_overrides": {
        "min_developmental": 0.35,
        "temporal_rhythm": {
            "typical_gap": [0.25, 1.5]
        }
    }
}
```

The base grammar is code-defined (the `low_fantasy_grammar()` function). Story-level overrides allow authors to tune without replacing the entire grammar.

### Where Purpose Vectors Live

Purpose vectors are part of the scene definition — they belong in the PostgreSQL `scenes.scene_data` JSONB alongside approach vectors, stakes, cast, and constraints:

```json
{
    "purpose": {
        "gravitational": 0.5,
        "connective": 0.0,
        "developmental": 0.3,
        "world_revealing": 0.0,
        "reflective": 0.0,
        "escalatory": 0.2,
        "threshold": 0.0
    }
}
```

They do NOT go on the AGE vertex — purpose vectors are not needed for graph traversal queries. They are loaded from PostgreSQL when computing narrative density along a path.

---

## What This Document Does NOT Resolve

1. **Purpose vector authoring UX** — how do story designers specify purpose vectors? Do they assign weights directly, or does the system infer from scene content? Likely a combination: coarse authoring with system refinement.
2. **Grammar tuning methodology** — how do we validate that a grammar produces good pacing? Playtesting, or can we compute pacing quality from the graph alone?
3. **Cross-grammar transitions** — a story that shifts genre mid-narrative (starts literary, becomes thriller) would need grammar blending. Deferred.
4. **Scene generation algorithms** — this document defines what a generated scene needs to *accomplish* (the specification). The algorithm that produces the scene content is a separate design problem.
5. **ML-assisted purpose inference** — can we train a model to infer purpose vectors from scene content? Useful for analyzing existing fiction and for validating generated scenes.
6. **Interaction with sub-graph pacing** — sub-graphs (tales-within-tales) may have their own pacing grammar distinct from the parent narrative. A fairy tale sub-graph might use a folklore grammar with different density expectations.

---

## Relationship to Other Documents

| Document | Relationship |
|----------|-------------|
| [`scene-provenance.md`](scene-provenance.md) | Defines when and why scenes are generated; this document defines *what* they should accomplish |
| [`authoring-economics.md`](authoring-economics.md) | Establishes the sparse-graph problem; this document provides the formal measurement of sparsity |
| [`narrative-graph-age.md`](../technical/age-persistence/narrative-graph-age.md) | AGE schema — purpose vectors stored in PostgreSQL, not AGE |
| [`narrative-gravity.md`](../technical/graph-strategy/narrative-gravity.md) | Gravitational mass is complementary to purpose — mass says how important, purpose says what kind |
| [`scene-model.md`](../technical/scene-model.md) | Scene lifecycle — generation inserts into the exit pipeline |
| [`traversal-friction.md`](../technical/graph-strategy/traversal-friction.md) | Temporal friction — the temporal rhythm section connects to communication velocity and narrative-chronological constraints |
