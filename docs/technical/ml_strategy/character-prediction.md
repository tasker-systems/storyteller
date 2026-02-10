# Character Prediction Pipeline

## Purpose

The character prediction model replaces per-character LLM agents. Where the original architecture used an LLM call per character per turn — consuming a system prompt constructed from the character tensor and producing structured intent data (ACTION/BENEATH/THINKING) — the revised architecture treats this as a prediction problem.

Given a character's personality tensor, current emotional state, relational context, scene conditions, and the classified player input, the model predicts structured intent: what the character would do, say, and think, with confidence values. The Narrator then renders these intents as literary prose.

This is the "psychological frame" concept from [`tensor-schema-spec.md`](../tensor-schema-spec.md) and the [emotional model](../../foundation/emotional-model.md), promoted from a pre-processing step to the primary character-behavior path. The ML model reads structured features and produces structured output — no natural language generation, no context windows, no prompt engineering.

---

## Feature Schema

The model consumes a 453-dimensional float vector organized into seven named regions. This encoding is the shared contract between the Rust inference path (`ort`) and the Python training path (PyTorch). Both sides must agree on the exact layout.

The canonical definition lives at `storyteller-ml/src/feature_schema.rs`.

### Input Regions

| Region | Dimensions | Encoding |
|--------|-----------|----------|
| Character tensor | 208 | 16 axis slots × 13 features each (4 `AxisValue` floats + 4 `TemporalLayer` one-hot + 5 `Provenance` one-hot) |
| Emotional state | 48 | 8 Plutchik primaries × 6 features each (1 intensity float + 5 `AwarenessLevel` one-hot) |
| Self-edge | 7 | trust(3) + affection + debt + history_weight + projection_accuracy |
| Relational edges | 120 | 5 edge slots × 24 features each (5 substrate dimensions × 4 `AxisValue` floats + 4 `TopologicalRole` one-hot) |
| Scene context | 6 | 4 `SceneType` one-hot + cast_size + tension |
| Player event | 16 | 7 `EventType` one-hot + 7 `EmotionalRegister` one-hot + confidence + target_count |
| History | 48 | 3 recent turns × 16 features each (6 `ActionType` one-hot + 4 `SpeechRegister` one-hot + 5 `AwarenessLevel` one-hot + emotional_valence) |

**Total: 453 dimensions.**

### Why Each Region Matters

The **character tensor** provides the personality profile — which axes are active, how extreme, how stable, and how well-established (temporal layer) or reliable (provenance). Empty axis slots are zero-padded, so characters with fewer defined axes naturally produce sparser feature vectors.

The **emotional state** captures the character's current emotional landscape — not just what they feel, but how aware they are of each feeling. A character with high sadness intensity at `Defended` awareness will behave differently from one at `Articulate` awareness: the first deflects, the second names it.

The **self-edge** is the character's relationship with themselves — a loopback edge in the relational graph that follows the same schema as inter-entity edges. Self-trust, self-affection, and projection accuracy shape how a character processes internal experience.

The **relational edges** encode the character's relationships with others present in the scene. The substrate dimensions (trust reliability/competence/benevolence, affection, debt) provide the relational texture. The topological role (gate, bridge, hub, periphery) encodes structural position in the relational web, which generates power dynamics independently of edge properties.

The **scene context** tells the model where this is happening. Scene type determines the narrative register (a gate scene has different stakes than a connective scene). Cast size and tension calibrate the social dynamics.

The **player event** is the classified player input — what kind of action the player took, its emotional register, and how confident the classifier was. This is what the character is responding to.

The **history** provides trajectory — the character's own recent behavior (last 2-3 turns). This prevents repetition and enables behavioral arcs within a scene.

### Padding and Empty Slots

Axes beyond the first 16 are ignored. Characters with fewer axes get zero-padded slots. Relational edges beyond 5 are ignored; empty edge slots are zero-padded. History entries beyond 3 are dropped; missing entries are zero-padded. This fixed-size encoding is what makes the feature vector a valid tensor input.

---

## Output Structure

The model produces a 42-dimensional output vector decoded into four prediction heads.

### Action Head (14 dimensions)

| Field | Encoding | Description |
|-------|----------|-------------|
| ActionType | 6-dim softmax | Perform, Speak, Move, Examine, Wait, Resist |
| confidence | 1 float | How confident the model is in the action |
| target_index | 1 float | Cast member index (-1 for no target) |
| emotional_valence | 1 float | Positive (warmth) ↔ negative (tension) |
| ActionContext | 5-dim softmax | SharedHistory, CurrentScene, EmotionalReaction, RelationalDynamic, WorldResponse |

### Speech Head (6 dimensions)

| Field | Encoding | Description |
|-------|----------|-------------|
| speech_occurs | 1 float (>0.5 = yes) | Whether the character speaks |
| SpeechRegister | 4-dim softmax | Whisper, Conversational, Declamatory, Internal |
| confidence | 1 float | How confident the model is in the speech prediction |

### Thought Head (6 dimensions)

| Field | Encoding | Description |
|-------|----------|-------------|
| AwarenessLevel | 5-dim softmax | Articulate, Recognizable, Preconscious, Defended, Structural |
| dominant_emotion_index | 1 float | Index into the 8 Plutchik primaries |

### Emotion Head (16 dimensions)

| Field | Encoding | Description |
|-------|----------|-------------|
| intensity_deltas | 8 floats | Per-primary intensity change (positive = increase) |
| awareness_shifts | 8 floats | Per-primary awareness shift indicator (>0.5 = shift) |

**Total: 42 dimensions.**

All model outputs are raw logits — no activation functions in the model. Softmax, sigmoid, and threshold operations are applied at decode time in Rust, matching the decode logic in `feature_schema::decode_outputs()`.

---

## MLP Architecture

The model is a multi-head MLP trained in Python (PyTorch) and exported to ONNX.

```
Input (453) → Shared trunk → 4 prediction heads

Shared trunk:
  Linear(453, 384) → ReLU → Dropout(0.3)
  Linear(384, 256) → ReLU → Dropout(0.3)
  Linear(256, 256) → ReLU

Prediction heads (each):
  Linear(256, 64) → ReLU → Linear(64, N)
  where N = 14 (action), 6 (speech), 6 (thought), 16 (emotion)
```

The trunk extracts shared representations from the full feature space. The heads specialize — the action head learns different feature interactions than the emotion head, but both benefit from the shared trunk's learned representations.

### Why MLP, Not Transformer

The input is a 453-dimensional float vector — already feature-engineered with named regions, one-hot categoricals, and structured padding. There are no sequential dependencies between features (axis 3 doesn't attend to axis 7). A transformer's self-attention mechanism would add latency (~5-15ms) and parameters (~44M minimum for a small encoder) for no representational benefit over a flat feature vector.

The MLP is fast (<1ms inference), tiny (38KB ONNX), and produces the same quality predictions. The right model for structured features is a structured-feature model.

### Head-Specific Loss

The Python training pipeline uses `MultiHeadLoss` — composing per-head loss computations with configurable weighting:

- **Action head**: CrossEntropy for ActionType and ActionContext softmax targets, MSE for confidence and valence
- **Speech head**: BCE for the occurs flag, CrossEntropy for SpeechRegister
- **Thought head**: CrossEntropy for AwarenessLevel, MSE for dominant emotion index
- **Emotion head**: MSE for intensity deltas, BCE for awareness shift flags

The training implementation lives at `training/src/training/model.py`.

---

## Inference in Rust

The `CharacterPredictor` struct in `storyteller-engine/src/inference/frame.rs` owns the ONNX session and rayon thread pool.

### Single Prediction

```
predict(&self, input, character_id, axis_indices, confidence)
  1. encode_features(input)  →  [453] float vector
  2. Tensor::from_array([1, 453], features)
  3. session.run(inputs!["features" => tensor])
  4. Concatenate 4 output tensors ("action", "speech", "thought", "emotion") → [42]
  5. decode_outputs(flat, character_id, axis_indices, confidence)  →  RawCharacterPrediction
```

### Batch Prediction

`predict_batch()` runs multiple characters in parallel via the dedicated rayon pool. Each character's prediction is independent — no inter-character attention or shared state — so parallelism is straightforward.

```rust
self.pool.install(|| {
    inputs.par_iter()
        .map(|(input, id, axes, conf)| self.predict(input, *id, axes.clone(), *conf))
        .collect()
})
```

### Error Handling

All errors are wrapped in `StorytellerError::Inference`. The caller (turn cycle system) decides whether a prediction failure for one character should abort the turn or proceed without that character's intent.

---

## Enrichment and Rendering

Raw predictions are numeric indices and float values. The Narrator needs structured narrative fact with emotional annotation — not numbers. Two stages bridge this gap.

### Enrichment

`enrich_prediction()` in `storyteller-engine/src/context/prediction.rs` converts `RawCharacterPrediction` → `CharacterPrediction`:

1. **Axis resolution**: Maps activated axis indices to axis names from the character's tensor BTreeMap (sorted key order matches encoding order).
2. **Action description**: Templates a narrative description from action type, target name, action context, and emotional valence. Example: "Approaches Pyotir — driven by shared history, with warmth"
3. **Speech direction**: Templates a content direction from register, context, and scene stakes. Example: "In natural conversation — about what they share, in the context of whether the melody still means something"
4. **Thought subtext**: Generates emotional subtext from the dominant primary and awareness level. Example: "Bramblehoof senses joy"
5. **Internal conflict detection**: Scans emotional deltas for opposing movements (one primary increasing while another decreases). Example: "joy rising while sadness recedes"

All string generation is deterministic templates — no LLM calls in enrichment.

### Rendering

`render_predictions()` formats assembled predictions as markdown sections for the Narrator's context window:

```markdown
## Character Predictions

### Bramblehoof
**Frame**: empathy, grief (0.80 confidence)
Active in the context of whether the melody still means something
**Action** (0.85): Approaches Pyotir — driven by shared history, with warmth
**Speech** (Conversational, 0.70): In natural conversation — about what they share...
**Internal**: Bramblehoof senses joy
  Awareness: Recognizable | Conflict: joy rising while sadness recedes
**Emotional shifts**: joy +0.2, sadness -0.1
```

This markdown enters the Narrator's context as structured narrative fact — not pre-rendered prose, not raw numbers. The Narrator decides how to transform it into story.

---

## Validation Approach

### Schema Tests (always run)

- `TOTAL_INPUT_FEATURES == 453` and `TOTAL_OUTPUT_FEATURES == 42` verified arithmetically
- `encode_features()` produces correct-length vectors
- Individual regions encode expected values (tensor, emotions, self-edge, edges, history)
- `decode_outputs()` round-trips known values through encode → decode
- Schema metadata regions are contiguous and cover the full vector
- Zero-padding for empty slots (edges, history, tensor axes)

### Model Tests (feature-gated: `test-ml-model`)

- Load real ONNX model from `$STORYTELLER_DATA_PATH/models/character_predictor.onnx`
- Single prediction with workshop character data produces structurally valid output
- Batch prediction runs two characters in parallel with distinct results
- End-to-end pipeline: predict → enrich → render produces non-empty markdown with expected sections
- Enriched predictions have valid axis names, primary IDs, and awareness levels
