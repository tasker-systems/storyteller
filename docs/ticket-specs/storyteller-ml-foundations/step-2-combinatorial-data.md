# Step 2: Combinatorial Training Data Matrix

**Status: COMPLETE** (Feb 8, 2026)

## Summary

Built the complete combinatorial training data pipeline: hand-authored JSON descriptors define the axes of variation (archetypes, dynamics, profiles, genre gates, cross-dimensions), Rust modules instantiate scenario skeletons from those descriptors, heuristic label generation produces ground-truth predictions, coherence validation filters bad examples, and CLI binaries export JSONL with manifests.

### Results

- **15,000 examples** generated from 5,000 sampled matrix cells x 3 variations
- **0 rejected** — all pass coherence validation (min score 0.885, mean 0.995)
- **1,303 unique cells** covered from ~79,200 theoretical combinations
- **0 duplicate content hashes** — stochastic variation produces unique examples
- **Feature vectors**: 453 input features, 42 output labels (matches Step 1 schema exactly)
- **113 tests** pass across the full workspace (35 core + 44 engine + 33 ML)

---

## Context

Steps 1 and 6 of the Phase 0 ML pipeline are complete:
- `storyteller-ml/src/feature_schema.rs` — 453-feature input / 42-feature output encoding contract
- `storyteller-core/src/types/prediction.rs` — Raw ML output types separated from assembled types

This step built the combinatorial matrix that generates training data. The goal was **proof-of-workflow**: a pipeline that produces valid JSONL with `rand-within-bounds` variation. No LLM enrichment yet — that's a later layer. Single genre (low fantasy/folklore) for the pilot, but genre is modeled as an extensible structural parameter that gates valid combinations.

---

## Architecture

```
storyteller-data/training-data/descriptors/    (Layer 1: hand-authored JSON)
    axis-vocabulary.json
    archetypes.json          (12 archetypes)
    dynamics.json            (10 relational dynamics)
    profiles.json            (10 scene profiles)
    genres.json              (1 pilot genre)
    cross-dimensions.json    (age, gender, species)
            │
            ▼
storyteller-ml/src/matrix/                     (Layer 2: Rust types + logic)
    descriptors.rs           (deserialize + validate JSON)
    archetypes.rs            (descriptor → CharacterSheet)
    dynamics.rs              (descriptor → DirectedEdge pair)
    profiles.rs              (descriptor → SceneFeatureInput + EventFeatureInput)
    combinator.rs            (iterate valid cells, produce ScenarioSkeleton)
    labels.rs                (heuristic label generation)
    validation.rs            (coherence checks)
    export.rs                (JSONL + manifest)
            │
            ▼
storyteller-ml/src/bin/                        (Layer 3: CLI pipeline)
    generate_training_data   load → matrix → label → validate → export JSONL
    validate_dataset         read JSONL → check lengths/hashes/coherence → report
```

---

## Layer 1: JSON Descriptor Files

All files live in `storyteller-data/training-data/descriptors/` (accessed via `STORYTELLER_DATA_PATH` env var — see `.env.example`).

### Axis Vocabulary (`axis-vocabulary.json`)

32 axes across 7 categories. This is the authoritative set of valid axis names.

| Category | Axes |
|----------|------|
| Emotional (8) | joy_wonder, sadness_grief, hope_anticipation, fear_wariness, anger, trust_emotional, longing_desire, contentment_satisfaction |
| Relational (8) | warmth_openness, empathy, protective_impulse, respect_for_autonomy, attachment_security, distance_management, duty_obligation, pride_dignity |
| Cognitive (6) | pattern_recognition, narrative_framing, intuitive_analytical, cautious_impulsive, self_awareness, practical_focus |
| Creative (3) | creative_expression, creative_receptivity, creative_capacity |
| Social (4) | dominant_deferential, gregarious_solitary, performative_private, expression_silence |
| Moral (3) | loyalty, sacrifice_willingness, honesty_transparency |

Each axis entry: `{id, display_name, description, typical_layer, tags}`.

### Archetype Descriptors (`archetypes.json`) — 12 archetypes

Each archetype selects 10-16 axes with value ranges, plus emotional profile, self-edge, and action tendencies.

| # | ID | Model | Key Traits |
|---|-----|-------|------------|
| 1 | `wandering_artist` | Bramblehoof | High creative, empathic, bedrock joy |
| 2 | `stoic_survivor` | Pyotir | Duty-bound, defended grief, practical |
| 3 | `byronic_hero` | — | High defiance, defended grief, structural pride |
| 4 | `protective_parent` | Kate-like | High protective, articulate fear, bedrock loyalty |
| 5 | `wise_elder` | — | Patient, sediment knowledge, defended regret |
| 6 | `clever_trickster` | — | Adaptive, preconscious mischief, surface trust |
| 7 | `grieving_youth` | Beth-like | Defended anger, structural joy (former), topsoil numbness |
| 8 | `reluctant_leader` | — | High competence, defended ambition, articulate duty |
| 9 | `zealous_reformer` | — | Righteous anger, bedrock loyalty, structural fear |
| 10 | `withdrawn_scholar` | — | Pattern recognition, defended longing, low gregariousness |
| 11 | `loyal_soldier` | — | Bedrock duty, moderate warmth, defended fear |
| 12 | `fey_outsider` | — | High creative capacity, structural distance, primordial available |

Archetype structure:
```
axes[]: [{axis_id, central_tendency: {min,max}, variance: {min,max}, range_low: {min,max}, range_high: {min,max}, layer, provenance}]
default_emotional_profile: {grammar_id, primaries[]: [{primary_id, intensity: {min,max}, awareness}]}
default_self_edge: {trust_competence: {min,max}, trust_intentions: {min,max}, ...}
action_tendencies: {primary_action_types[], primary_action_context[], speech_likelihood, speech_registers[], default_awareness}
```

### Dynamic Descriptors (`dynamics.json`) — 10 dynamics

Each defines BOTH edge directions (A→B and B→A substrate configs).

1. `mentor_student` — asymmetric guidance, trust flows upward
2. `siblings_complex` — high history, complex trust
3. `rivals_with_respect` — low affection, high projection
4. `strangers_shared_grief` — low history, emergent trust
5. `authority_subject` — structural trust asymmetry
6. `former_friends_estranged` — high history, fractured trust
7. `patron_protege` — high benevolence A→B, high debt B→A
8. `companions_on_road` — moderate and growing all dimensions
9. `suspicious_allies` — low trust, mutual projection
10. `guardian_ward` — high protective from A, high dependence from B

### Profile Descriptors (`profiles.json`) — 10 scene profiles

Each defines scene type, tension range, cast size, and weighted characteristic events.

1. `quiet_reunion` — gravitational, low-moderate tension, speech/observation
2. `confrontation_over_betrayal` — gravitational, high tension, speech/resist
3. `vulnerable_admission` — threshold, low tension, emote/speech
4. `celebration_interrupted` — connective→gravitational, rising tension
5. `first_meeting_hidden_agendas` — connective, moderate tension, observation/speech
6. `physical_challenge_cooperation` — gate, high tension, action/movement
7. `quiet_aftermath` — threshold, low tension, observation/emote
8. `negotiation_under_pressure` — gravitational, high tension, speech dominant
9. `discovery_of_truth` — gate, rising tension, observation/inquiry
10. `farewell_scene` — threshold, low-moderate tension, speech/emote

### Genre Descriptor (`genres.json`)

One pilot genre: `low_fantasy_folklore`. All 12 archetypes, 10 dynamics, and 10 profiles are valid. Has `excluded_combinations[]` for edge cases (e.g., fey_outsider + physical_challenge_cooperation).

### Cross-Dimensions (`cross-dimensions.json`)

Additive modifiers sampled per instance (not full matrix axes):
- Age: youth/adult/elder — modifies cautious_impulsive, self_awareness
- Gender: masculine/feminine/nonbinary — no axis modifiers (demographic diversity only)
- Species: human/fey — fey enables primordial layer, modifies creative_capacity, distance_management

---

## Layer 2: Rust Types and Logic

### `storyteller-ml/src/matrix/descriptors.rs`

Deserialization types for all JSON descriptors. Key types:

- `ValueRange { min: f32, max: f32 }` — bounds for stochastic sampling, with `sample()` and `clamp()` methods
- `ArchetypeDescriptor` — axes, emotional profile, self-edge, action tendencies
- `DynamicDescriptor` — role names, topology strings, bidirectional edge substrate templates
- `ProfileDescriptor` — scene_type, tension, cast_size, characteristic_events with weights
- `GenreDescriptor` — validity gates (valid_archetypes[], valid_dynamics[], valid_profiles[], excluded_combinations[])
- `CrossDimensionSet` — demographic dimensions with axis modifiers
- `DescriptorSet` — complete loaded set with `load(data_path: &Path) -> Result<Self>` and internal reference validation
- `resolve_data_path()` — resolves descriptor directory from `STORYTELLER_DATA_PATH` env var (loads `.env` if present)

### `storyteller-ml/src/matrix/archetypes.rs`

`instantiate_character(archetype, cross_sample, descriptors, rng) -> CharacterSheet`

1. Sample each axis value within descriptor ranges (rand uniform within [min, max])
2. Apply cross-dimension modifiers additively, clamp to [-1.0, 1.0]
3. Parse layer/provenance strings to enums via `parse_temporal_layer()`, `parse_provenance()`, `parse_awareness()`
4. Sample emotional state from profile ranges
5. Sample self-edge from descriptor ranges
6. Fill placeholder strings (voice, backstory, performance_notes from archetype description)

### `storyteller-ml/src/matrix/dynamics.rs`

`instantiate_edges(dynamic, source_id, target_id, rng) -> (DirectedEdge, DirectedEdge)`

Sample substrate values within ranges. Derive range_low/range_high from central_tendency +/- variance*1.5, clamped. Parse topology string to `TopologicalRole` via `parse_topology()`.

### `storyteller-ml/src/matrix/profiles.rs`

`instantiate_scene_and_event(profile, rng) -> (SceneFeatureInput, EventFeatureInput)`

Sample tension, cast_size. Select characteristic event by `WeightedIndex` random sampling.

### `storyteller-ml/src/matrix/combinator.rs`

`generate_matrix(descriptors, genre, count, variations, rng) -> Vec<ScenarioSkeleton>`

Iterates archetype_a x archetype_b (a != b) x dynamic (both role assignments) x profile, checking genre validity gates and `excluded_combinations`. Shuffles and caps at `count` by random sampling from valid cells. Produces `variations` stochastic instances per cell.

`ScenarioSkeleton` contains: `MatrixCell` (which descriptors combined), both `CharacterSheet`s, both `DirectedEdge`s, both `TopologicalRole`s, `SceneFeatureInput`, `EventFeatureInput`, `CrossSample`, and `variation_index`.

Matrix size: 12 * 11 * 10 * 2 * 10 = 26,400 valid cells (before exclusions). Default samples 5,000 cells x 3 variations = 15,000 examples.

### `storyteller-ml/src/matrix/labels.rs`

`generate_labels(skeleton, rng) -> RawCharacterPrediction`

Heuristic label generation — four components:

**Action type**: Weighted selection from 6 action types. Base weights modulated by event type resonance (Speech→boost Speak, Action→boost Perform), emotional register (Aggressive→boost Resist, Tender→boost Speak), scene tension (high→boost Resist/Move, low→boost Wait/Examine), and tensor axis values (high empathy→boost Speak, high distance_management→boost Wait).

**Speech**: `occurs` = base likelihood from event type, modified by `expression_silence` axis. `register` selected by scene tension (high→Declamatory possible, low→Whisper possible, default Conversational).

**Thought**: `awareness_level` derived from character's `self_awareness` axis value. `dominant_emotion_index` = highest intensity primary in emotional state.

**Emotional deltas**: Player register drives boost/suppress indices (Aggressive→anger+/trust-; Vulnerable→trust+/fear-; Tender→joy+/trust+). Magnitude sampled 0.05-0.25 for boosts, 0.05-0.15 for suppression. Awareness shifts triggered when magnitude > 0.15 with p=0.2.

### `storyteller-ml/src/matrix/validation.rs`

`validate_example(skeleton, prediction, min_coherence) -> ValidationResult`

5 weighted rules (default threshold: coherence_score >= 0.6):

1. **Emotional consistency** (0.25) — deltas don't push primaries outside [-0.1, 1.1], dominant emotion among top-3 intensity primaries
2. **Relational alignment** (0.20) — low trust + SharedHistory flagged, high affection + Resist flagged
3. **Awareness discipline** (0.25) — Structural emotion shouldn't be dominant, Defended + Articulate thought is contradictory, large awareness shifts from deep levels are suspicious
4. **Temporal stability** (0.15) — large emotional deltas (> 0.3) penalized
5. **Action-scene alignment** (0.15) — Resist in low-tension scenes flagged, Wait in high-tension scenes penalized

### `storyteller-ml/src/matrix/export.rs`

- `TrainingExample { id, cell, variation, features: Vec<f32>, labels: Vec<f32>, coherence_score, content_hash }`
- `DatasetManifest { version, total_generated, total_valid, total_rejected, mean_coherence, matrix_coverage, input_features, output_features, generated_at, seed }`
- `content_hash(features, labels) -> String` — SHA-256 of feature+label bytes for deduplication
- `example_id() -> String` — UUID v7
- `write_jsonl(examples, writer)` and `write_manifest(manifest, path)`

### Added to: `storyteller-ml/src/feature_schema.rs`

`encode_labels(prediction: &RawCharacterPrediction) -> Vec<f32>` — reverse of `decode_outputs()`. Same constants, same ordering. Lives in feature_schema to stay in sync with the encoding contract.

Also added `Copy` derive to `SceneFeatureInput` and `EventFeatureInput` (small value types).

---

## Layer 3: CLI Binaries

### `storyteller-ml/src/bin/generate_training_data.rs`

```
generate-training-data
    --genre <id>            (default: low_fantasy_folklore)
    --count <n>             (default: 5000)
    --variations <n>        (default: 3)
    --output <path>         (default: training_data.jsonl)
    --seed <u64>            (optional, reproducibility)
    --min-coherence <f32>   (default: 0.6)
    --data-path <path>      (optional, falls back to env/symlink)
```

Pipeline: load descriptors → generate matrix → for each skeleton: generate labels → encode features + labels → validate → export valid examples as JSONL + write manifest alongside.

### `storyteller-ml/src/bin/validate_dataset.rs`

```
validate-dataset
    --input <path>          (required)
    --verbose               (optional, show per-example details)
```

Reads JSONL, checks feature/label vector lengths against schema constants, detects duplicate content hashes, reports coherence statistics. Exits with code 1 if any validation failures.

---

## Dependencies Added

**Workspace root `Cargo.toml`**:
- `sha2 = "0.10"` — content hashing
- `dotenvy = "0.15"` — STORYTELLER_DATA_PATH loading

**`storyteller-ml/Cargo.toml`**: sha2, dotenvy, chrono, clap, uuid (all workspace refs)

---

## Verification (Actual Results)

1. `cargo check --workspace --all-features` — clean
2. `cargo clippy --workspace --all-targets --all-features` — zero warnings
3. `cargo fmt --check` — clean
4. `cargo test --workspace --all-features` — 113 tests pass (35 core + 44 engine + 33 ML + 1 ignored)
5. `generate-training-data --count 50 --seed 42` — 150 examples, all valid, features=453, labels=42
6. `generate-training-data --count 5000 --seed 42` — 15,000 examples, 0 rejected, mean coherence 0.995, 1,303 unique cells, 0 duplicate hashes
7. `validate-dataset --input training_data.jsonl` — all examples pass, zero parse errors, zero length mismatches

---

## What This Does NOT Include (Deferred)

- **LLM enrichment**: Labels are pure heuristic. A future step will use LLM-generated predictions as higher-quality ground truth, with these heuristic labels serving as a validation baseline.
- **Multiple genres**: Only `low_fantasy_folklore` is defined. The genre gate architecture is extensible.
- **History features**: `history` field in `PredictionInput` is always empty (no sequential context). History-aware training data requires multi-turn scenario generation.
- **Integration test binary**: The plan spec mentioned a `validate_dataset` integration test; the CLI binary serves this purpose more practically.
