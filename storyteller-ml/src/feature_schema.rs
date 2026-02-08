//! Feature encoding schema — the shared contract between Rust and Python.
//!
//! This module defines exactly how storyteller types map to fixed-size float
//! vectors for ML consumption. Both the Rust inference path (ort) and the
//! Python training path (PyTorch) must agree on this encoding.
//!
//! ## Architecture
//!
//! The feature vector is a flat `Vec<f32>` with named regions. Each region
//! encodes a specific data source (tensor, emotions, edges, scene, etc.)
//! using a mix of raw floats and one-hot categorical encodings.
//!
//! The output vector is also flat `Vec<f32>`, decoded into a
//! [`RawCharacterPrediction`] by `decode_outputs()`.
//!
//! ## Schema Export
//!
//! `schema_metadata()` returns a JSON-serializable description of every
//! feature dimension, so Python can generate matching DataLoader code.
//!
//! ## Conventions
//!
//! - One-hot encodings use the enum's discriminant order (first variant = index 0).
//! - Padded regions (axes, edges) use all-zero vectors for empty slots.
//! - Float values are used as-is (no normalization in this layer).

use serde::{Deserialize, Serialize};

use storyteller_core::types::character::{CharacterSheet, EmotionalPrimary, SelfEdge};
use storyteller_core::types::prediction::{
    ActionContext, ActionType, EmotionalRegister, EventType, RawActionPrediction,
    RawActivatedTensorFrame, RawCharacterPrediction, RawEmotionalDelta, RawSpeechPrediction,
    RawThoughtPrediction, SpeechRegister,
};
use storyteller_core::types::relational::{DirectedEdge, RelationalSubstrate, TopologicalRole};
use storyteller_core::types::scene::SceneType;
use storyteller_core::types::tensor::{AwarenessLevel, AxisValue, Provenance, TemporalLayer};

// ===========================================================================
// Schema constants
// ===========================================================================

/// Maximum number of tensor axes encoded. Axes beyond this are ignored;
/// characters with fewer axes get zero-padded slots.
pub const MAX_TENSOR_AXES: usize = 16;

/// Features per tensor axis: 4 (AxisValue) + 4 (TemporalLayer one-hot) + 5 (Provenance one-hot).
pub const FEATURES_PER_AXIS: usize = 4 + 4 + 5; // = 13

/// Maximum number of relational edges encoded per character.
pub const MAX_EDGES: usize = 5;

/// Features per relational edge: 5 substrate dimensions × 4 (AxisValue) + 4 (TopologicalRole one-hot).
pub const FEATURES_PER_EDGE: usize = 5 * 4 + 4; // = 24

/// Number of Plutchik primaries in the default grammar.
pub const NUM_PRIMARIES: usize = 8;

/// Features per emotional primary: 1 (intensity) + 5 (AwarenessLevel one-hot).
pub const FEATURES_PER_PRIMARY: usize = 1 + 5; // = 6

/// Features for the self-edge: trust(3) + affection + debt + history_weight + projection_accuracy.
pub const SELF_EDGE_FEATURES: usize = 7;

/// Features for scene context: 4 (SceneType one-hot) + 1 (cast_size) + 1 (tension).
pub const SCENE_FEATURES: usize = 6;

/// Features for classified player input: 7 (EventType one-hot) + 7 (EmotionalRegister one-hot) +
/// 1 (confidence) + 1 (target_count).
pub const EVENT_FEATURES: usize = 16;

/// Features per history entry: 6 (ActionType one-hot) + 4 (SpeechRegister one-hot) +
/// 5 (AwarenessLevel one-hot) + 1 (emotional_valence).
pub const FEATURES_PER_HISTORY: usize = 16;

/// Number of recent turns encoded as history.
pub const HISTORY_DEPTH: usize = 3;

/// Total input feature vector length.
pub const TOTAL_INPUT_FEATURES: usize = MAX_TENSOR_AXES * FEATURES_PER_AXIS // 208
    + NUM_PRIMARIES * FEATURES_PER_PRIMARY                                   // 48
    + SELF_EDGE_FEATURES                                                     // 7
    + MAX_EDGES * FEATURES_PER_EDGE                                          // 120
    + SCENE_FEATURES                                                         // 6
    + EVENT_FEATURES                                                         // 16
    + HISTORY_DEPTH * FEATURES_PER_HISTORY; // 48
                                            // Total: 453

// --- Output schema constants ---

/// ActionType variants.
pub const NUM_ACTION_TYPES: usize = 6;
/// ActionContext variants.
pub const NUM_ACTION_CONTEXTS: usize = 5;
/// SpeechRegister variants.
pub const NUM_SPEECH_REGISTERS: usize = 4;
/// AwarenessLevel variants.
pub const NUM_AWARENESS_LEVELS: usize = 5;

/// Total output feature vector length.
///
/// Layout:
/// - ActionType softmax (6) + confidence (1) + target_index (1) + emotional_valence (1) + ActionContext softmax (5)  = 14
/// - speech_occurs (1) + SpeechRegister softmax (4) + confidence (1)  = 6
/// - AwarenessLevel softmax (5) + dominant_emotion_index (1)  = 6
/// - per-primary emotional deltas (8) + per-primary awareness shifts (8)  = 16
///
/// Total: 42
pub const TOTAL_OUTPUT_FEATURES: usize = NUM_ACTION_TYPES + 1 + 1 + 1 + NUM_ACTION_CONTEXTS // 14
    + 1 + NUM_SPEECH_REGISTERS + 1                                                          // 6
    + NUM_AWARENESS_LEVELS + 1                                                               // 6
    + NUM_PRIMARIES + NUM_PRIMARIES; // 16
                                     // Total: 42

// ===========================================================================
// Input types — what the encoder consumes
// ===========================================================================

/// Scene-level features for encoding.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SceneFeatureInput {
    /// Scene classification.
    pub scene_type: SceneType,
    /// Number of characters in the scene's cast.
    pub cast_size: u8,
    /// Scene tension level. Range: \[0.0, 1.0\].
    pub tension: f32,
}

/// Classified player input features for encoding.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EventFeatureInput {
    /// What kind of event the player input represents.
    pub event_type: EventType,
    /// The emotional register of the input.
    pub emotional_register: EmotionalRegister,
    /// Classifier confidence.
    pub confidence: f32,
    /// Number of targets referenced.
    pub target_count: u8,
}

/// A single history entry — what happened in a recent turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// What action type was predicted last turn.
    pub action_type: ActionType,
    /// What speech register was used (if any).
    pub speech_register: SpeechRegister,
    /// Awareness level of thought.
    pub awareness_level: AwarenessLevel,
    /// Emotional valence of the action.
    pub emotional_valence: f32,
}

/// Everything needed to encode a single character's features for one turn.
#[derive(Debug, Clone)]
pub struct PredictionInput<'a> {
    /// The character sheet (tensor, emotions, self-edge).
    pub character: &'a CharacterSheet,
    /// Relational edges FROM this character toward others in the scene.
    pub edges: &'a [DirectedEdge],
    /// Topological roles for each edge's target entity, in the same order as `edges`.
    pub target_roles: &'a [TopologicalRole],
    /// Scene context.
    pub scene: SceneFeatureInput,
    /// Current player input classification.
    pub event: EventFeatureInput,
    /// Recent history (up to `HISTORY_DEPTH` entries, most recent first).
    pub history: &'a [HistoryEntry],
}

// ===========================================================================
// One-hot encoding helpers
// ===========================================================================

fn one_hot_temporal_layer(layer: TemporalLayer) -> [f32; 4] {
    match layer {
        TemporalLayer::Topsoil => [1.0, 0.0, 0.0, 0.0],
        TemporalLayer::Sediment => [0.0, 1.0, 0.0, 0.0],
        TemporalLayer::Bedrock => [0.0, 0.0, 1.0, 0.0],
        TemporalLayer::Primordial => [0.0, 0.0, 0.0, 1.0],
    }
}

fn one_hot_provenance(prov: Provenance) -> [f32; 5] {
    match prov {
        Provenance::Authored => [1.0, 0.0, 0.0, 0.0, 0.0],
        Provenance::Inferred => [0.0, 1.0, 0.0, 0.0, 0.0],
        Provenance::Generated => [0.0, 0.0, 1.0, 0.0, 0.0],
        Provenance::Confirmed => [0.0, 0.0, 0.0, 1.0, 0.0],
        Provenance::Overridden => [0.0, 0.0, 0.0, 0.0, 1.0],
    }
}

fn one_hot_awareness(level: AwarenessLevel) -> [f32; 5] {
    match level {
        AwarenessLevel::Articulate => [1.0, 0.0, 0.0, 0.0, 0.0],
        AwarenessLevel::Recognizable => [0.0, 1.0, 0.0, 0.0, 0.0],
        AwarenessLevel::Preconscious => [0.0, 0.0, 1.0, 0.0, 0.0],
        AwarenessLevel::Defended => [0.0, 0.0, 0.0, 1.0, 0.0],
        AwarenessLevel::Structural => [0.0, 0.0, 0.0, 0.0, 1.0],
    }
}

fn one_hot_scene_type(st: SceneType) -> [f32; 4] {
    match st {
        SceneType::Gravitational => [1.0, 0.0, 0.0, 0.0],
        SceneType::Connective => [0.0, 1.0, 0.0, 0.0],
        SceneType::Gate => [0.0, 0.0, 1.0, 0.0],
        SceneType::Threshold => [0.0, 0.0, 0.0, 1.0],
    }
}

fn one_hot_event_type(et: EventType) -> [f32; 7] {
    match et {
        EventType::Speech => [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        EventType::Action => [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        EventType::Movement => [0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
        EventType::Observation => [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
        EventType::Interaction => [0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        EventType::Emote => [0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
        EventType::Inquiry => [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
    }
}

fn one_hot_emotional_register(er: EmotionalRegister) -> [f32; 7] {
    match er {
        EmotionalRegister::Aggressive => [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        EmotionalRegister::Vulnerable => [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        EmotionalRegister::Playful => [0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
        EmotionalRegister::Guarded => [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
        EmotionalRegister::Neutral => [0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        EmotionalRegister::Tender => [0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
        EmotionalRegister::Inquisitive => [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
    }
}

fn one_hot_action_type(at: ActionType) -> [f32; 6] {
    match at {
        ActionType::Perform => [1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        ActionType::Speak => [0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
        ActionType::Move => [0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
        ActionType::Examine => [0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        ActionType::Wait => [0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
        ActionType::Resist => [0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
    }
}

fn one_hot_speech_register(sr: SpeechRegister) -> [f32; 4] {
    match sr {
        SpeechRegister::Whisper => [1.0, 0.0, 0.0, 0.0],
        SpeechRegister::Conversational => [0.0, 1.0, 0.0, 0.0],
        SpeechRegister::Declamatory => [0.0, 0.0, 1.0, 0.0],
        SpeechRegister::Internal => [0.0, 0.0, 0.0, 1.0],
    }
}

fn one_hot_topological_role(role: TopologicalRole) -> [f32; 4] {
    match role {
        TopologicalRole::Gate => [1.0, 0.0, 0.0, 0.0],
        TopologicalRole::Bridge => [0.0, 1.0, 0.0, 0.0],
        TopologicalRole::Hub => [0.0, 0.0, 1.0, 0.0],
        TopologicalRole::Periphery => [0.0, 0.0, 0.0, 1.0],
    }
}

/// Encode `ActionContext` as one-hot. Used by training data generation.
pub fn one_hot_action_context(ac: ActionContext) -> [f32; 5] {
    match ac {
        ActionContext::SharedHistory => [1.0, 0.0, 0.0, 0.0, 0.0],
        ActionContext::CurrentScene => [0.0, 1.0, 0.0, 0.0, 0.0],
        ActionContext::EmotionalReaction => [0.0, 0.0, 1.0, 0.0, 0.0],
        ActionContext::RelationalDynamic => [0.0, 0.0, 0.0, 1.0, 0.0],
        ActionContext::WorldResponse => [0.0, 0.0, 0.0, 0.0, 1.0],
    }
}

// ===========================================================================
// Encoding helpers
// ===========================================================================

fn encode_axis_value(v: &AxisValue, out: &mut Vec<f32>) {
    out.push(v.central_tendency);
    out.push(v.variance);
    out.push(v.range_low);
    out.push(v.range_high);
}

fn encode_substrate(s: &RelationalSubstrate, out: &mut Vec<f32>) {
    encode_axis_value(&s.trust_reliability, out);
    encode_axis_value(&s.trust_competence, out);
    encode_axis_value(&s.trust_benevolence, out);
    encode_axis_value(&s.affection, out);
    encode_axis_value(&s.debt, out);
}

fn encode_self_edge(se: &SelfEdge, out: &mut Vec<f32>) {
    out.push(se.trust.competence);
    out.push(se.trust.intentions);
    out.push(se.trust.reliability);
    out.push(se.affection);
    out.push(se.debt);
    out.push(se.history_weight);
    out.push(se.projection_accuracy);
}

fn encode_emotional_primaries(primaries: &[EmotionalPrimary], out: &mut Vec<f32>) {
    // Encode exactly NUM_PRIMARIES slots. If we have fewer, zero-pad.
    for i in 0..NUM_PRIMARIES {
        if let Some(p) = primaries.get(i) {
            out.push(p.intensity);
            out.extend_from_slice(&one_hot_awareness(p.awareness));
        } else {
            // Zero-pad: 1 float + 5 one-hot
            out.extend_from_slice(&[0.0; FEATURES_PER_PRIMARY]);
        }
    }
}

// ===========================================================================
// Public API: encode
// ===========================================================================

/// Encode a character's features into a fixed-size float vector.
///
/// The output vector has exactly [`TOTAL_INPUT_FEATURES`] elements.
/// This is the shared contract: the Python training code must read
/// features in the same order.
pub fn encode_features(input: &PredictionInput<'_>) -> Vec<f32> {
    let mut features = Vec::with_capacity(TOTAL_INPUT_FEATURES);

    // --- Region 1: Character tensor (MAX_TENSOR_AXES × FEATURES_PER_AXIS) ---
    let axes: Vec<_> = input.character.tensor.axes.iter().collect();
    for i in 0..MAX_TENSOR_AXES {
        if let Some((_name, entry)) = axes.get(i) {
            encode_axis_value(&entry.value, &mut features);
            features.extend_from_slice(&one_hot_temporal_layer(entry.layer));
            features.extend_from_slice(&one_hot_provenance(entry.provenance));
        } else {
            features.extend_from_slice(&[0.0; FEATURES_PER_AXIS]);
        }
    }

    // --- Region 2: Emotional state (NUM_PRIMARIES × FEATURES_PER_PRIMARY) ---
    encode_emotional_primaries(&input.character.emotional_state.primaries, &mut features);

    // --- Region 3: Self-edge (SELF_EDGE_FEATURES) ---
    encode_self_edge(&input.character.self_edge, &mut features);

    // --- Region 4: Relational edges (MAX_EDGES × FEATURES_PER_EDGE) ---
    for i in 0..MAX_EDGES {
        if let Some(edge) = input.edges.get(i) {
            encode_substrate(&edge.substrate, &mut features);
            let role = input
                .target_roles
                .get(i)
                .copied()
                .unwrap_or(TopologicalRole::Periphery);
            features.extend_from_slice(&one_hot_topological_role(role));
        } else {
            features.extend_from_slice(&[0.0; FEATURES_PER_EDGE]);
        }
    }

    // --- Region 5: Scene context (SCENE_FEATURES) ---
    features.extend_from_slice(&one_hot_scene_type(input.scene.scene_type));
    features.push(input.scene.cast_size as f32);
    features.push(input.scene.tension);

    // --- Region 6: Classified player input (EVENT_FEATURES) ---
    features.extend_from_slice(&one_hot_event_type(input.event.event_type));
    features.extend_from_slice(&one_hot_emotional_register(input.event.emotional_register));
    features.push(input.event.confidence);
    features.push(input.event.target_count as f32);

    // --- Region 7: Recent history (HISTORY_DEPTH × FEATURES_PER_HISTORY) ---
    for i in 0..HISTORY_DEPTH {
        if let Some(h) = input.history.get(i) {
            features.extend_from_slice(&one_hot_action_type(h.action_type));
            features.extend_from_slice(&one_hot_speech_register(h.speech_register));
            features.extend_from_slice(&one_hot_awareness(h.awareness_level));
            features.push(h.emotional_valence);
        } else {
            features.extend_from_slice(&[0.0; FEATURES_PER_HISTORY]);
        }
    }

    debug_assert_eq!(
        features.len(),
        TOTAL_INPUT_FEATURES,
        "Feature vector length mismatch: got {}, expected {}",
        features.len(),
        TOTAL_INPUT_FEATURES,
    );

    features
}

// ===========================================================================
// Public API: decode
// ===========================================================================

/// Decode a model output vector into a [`RawCharacterPrediction`].
///
/// The `character_id` and `activated_axis_indices` must be provided
/// separately — they come from the input context, not from the model.
///
/// `output` must have exactly [`TOTAL_OUTPUT_FEATURES`] elements.
///
/// # Panics
///
/// Panics if `output.len() != TOTAL_OUTPUT_FEATURES`.
pub fn decode_outputs(
    output: &[f32],
    character_id: storyteller_core::types::entity::EntityId,
    activated_axis_indices: Vec<u16>,
    frame_confidence: f32,
) -> RawCharacterPrediction {
    assert_eq!(
        output.len(),
        TOTAL_OUTPUT_FEATURES,
        "Output vector length mismatch: got {}, expected {}",
        output.len(),
        TOTAL_OUTPUT_FEATURES,
    );

    let mut offset = 0;

    // --- Action head ---
    let action_type = argmax_to_action_type(&output[offset..offset + NUM_ACTION_TYPES]);
    offset += NUM_ACTION_TYPES;
    let action_confidence = output[offset];
    offset += 1;
    // target_index: -1 encodes "no target", 0..N encodes cast member index.
    // We can't resolve EntityId here (needs cast list), so store as None.
    // The caller resolves this from the raw float.
    let _target_index_raw = output[offset];
    offset += 1;
    let emotional_valence = output[offset];
    offset += 1;
    let action_context = argmax_to_action_context(&output[offset..offset + NUM_ACTION_CONTEXTS]);
    offset += NUM_ACTION_CONTEXTS;

    // --- Speech head ---
    let speech_occurs = output[offset] > 0.5;
    offset += 1;
    let speech_register = argmax_to_speech_register(&output[offset..offset + NUM_SPEECH_REGISTERS]);
    offset += NUM_SPEECH_REGISTERS;
    let speech_confidence = output[offset];
    offset += 1;

    // --- Thought head ---
    let awareness_level = argmax_to_awareness_level(&output[offset..offset + NUM_AWARENESS_LEVELS]);
    offset += NUM_AWARENESS_LEVELS;
    let dominant_emotion_index = output[offset].round() as u8;
    offset += 1;

    // --- Emotion head ---
    let deltas_raw = &output[offset..offset + NUM_PRIMARIES];
    offset += NUM_PRIMARIES;
    let shifts_raw = &output[offset..offset + NUM_PRIMARIES];
    offset += NUM_PRIMARIES;

    debug_assert_eq!(offset, TOTAL_OUTPUT_FEATURES);

    // Build emotional deltas — only include primaries with non-trivial change.
    let emotional_deltas: Vec<RawEmotionalDelta> = deltas_raw
        .iter()
        .zip(shifts_raw.iter())
        .enumerate()
        .filter(|(_, (delta, _))| delta.abs() > 0.01)
        .map(|(i, (&intensity_change, &shift))| RawEmotionalDelta {
            primary_index: i as u8,
            intensity_change,
            awareness_shifts: shift > 0.5,
        })
        .collect();

    RawCharacterPrediction {
        character_id,
        frame: RawActivatedTensorFrame {
            activated_axis_indices,
            confidence: frame_confidence,
        },
        action: RawActionPrediction {
            action_type,
            confidence: action_confidence,
            target: None, // Resolved by caller from _target_index_raw + cast list
            emotional_valence,
            action_context,
        },
        speech: RawSpeechPrediction {
            occurs: speech_occurs,
            register: speech_register,
            confidence: speech_confidence,
        },
        thought: RawThoughtPrediction {
            awareness_level,
            dominant_emotion_index,
        },
        emotional_deltas,
    }
}

// ===========================================================================
// Public API: encode labels (for training data generation)
// ===========================================================================

/// Encode a [`RawCharacterPrediction`] into a fixed-size float label vector.
///
/// This is the reverse of [`decode_outputs()`]: given a prediction struct,
/// produce the float vector that the model should output. Used by the
/// training data pipeline to generate ground-truth labels.
///
/// The output vector has exactly [`TOTAL_OUTPUT_FEATURES`] elements, in the
/// same order as `decode_outputs()` reads them.
pub fn encode_labels(prediction: &RawCharacterPrediction) -> Vec<f32> {
    let mut labels = Vec::with_capacity(TOTAL_OUTPUT_FEATURES);

    // --- Action head ---
    labels.extend_from_slice(&one_hot_action_type(prediction.action.action_type));
    labels.push(prediction.action.confidence);
    // target_index: encode as -1.0 for no target, 0.0+ for cast index.
    // Training data doesn't resolve cast indices, so always -1.0.
    labels.push(-1.0);
    labels.push(prediction.action.emotional_valence);
    labels.extend_from_slice(&one_hot_action_context(prediction.action.action_context));

    // --- Speech head ---
    labels.push(if prediction.speech.occurs { 1.0 } else { 0.0 });
    labels.extend_from_slice(&one_hot_speech_register(prediction.speech.register));
    labels.push(prediction.speech.confidence);

    // --- Thought head ---
    labels.extend_from_slice(&one_hot_awareness(prediction.thought.awareness_level));
    labels.push(prediction.thought.dominant_emotion_index as f32);

    // --- Emotion head ---
    // Per-primary deltas and awareness shifts — sparse, fill from deltas vec.
    let mut intensity_deltas = [0.0f32; NUM_PRIMARIES];
    let mut awareness_shifts = [0.0f32; NUM_PRIMARIES];
    for delta in &prediction.emotional_deltas {
        let idx = delta.primary_index as usize;
        if idx < NUM_PRIMARIES {
            intensity_deltas[idx] = delta.intensity_change;
            awareness_shifts[idx] = if delta.awareness_shifts { 1.0 } else { 0.0 };
        }
    }
    labels.extend_from_slice(&intensity_deltas);
    labels.extend_from_slice(&awareness_shifts);

    debug_assert_eq!(
        labels.len(),
        TOTAL_OUTPUT_FEATURES,
        "Label vector length mismatch: got {}, expected {}",
        labels.len(),
        TOTAL_OUTPUT_FEATURES,
    );

    labels
}

// --- argmax helpers ---

fn argmax(slice: &[f32]) -> usize {
    slice
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap_or(0)
}

fn argmax_to_action_type(slice: &[f32]) -> ActionType {
    match argmax(slice) {
        0 => ActionType::Perform,
        1 => ActionType::Speak,
        2 => ActionType::Move,
        3 => ActionType::Examine,
        4 => ActionType::Wait,
        _ => ActionType::Resist,
    }
}

fn argmax_to_action_context(slice: &[f32]) -> ActionContext {
    match argmax(slice) {
        0 => ActionContext::SharedHistory,
        1 => ActionContext::CurrentScene,
        2 => ActionContext::EmotionalReaction,
        3 => ActionContext::RelationalDynamic,
        _ => ActionContext::WorldResponse,
    }
}

fn argmax_to_speech_register(slice: &[f32]) -> SpeechRegister {
    match argmax(slice) {
        0 => SpeechRegister::Whisper,
        1 => SpeechRegister::Conversational,
        2 => SpeechRegister::Declamatory,
        _ => SpeechRegister::Internal,
    }
}

fn argmax_to_awareness_level(slice: &[f32]) -> AwarenessLevel {
    match argmax(slice) {
        0 => AwarenessLevel::Articulate,
        1 => AwarenessLevel::Recognizable,
        2 => AwarenessLevel::Preconscious,
        3 => AwarenessLevel::Defended,
        _ => AwarenessLevel::Structural,
    }
}

// ===========================================================================
// Schema metadata — exported for Python
// ===========================================================================

/// A named region in the feature vector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureRegion {
    /// Region name (e.g., "tensor", "emotional_state").
    pub name: String,
    /// Starting index in the feature vector.
    pub offset: usize,
    /// Number of features in this region.
    pub length: usize,
    /// Human-readable description.
    pub description: String,
}

/// Complete schema metadata for the feature encoding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaMetadata {
    /// Total number of input features.
    pub input_size: usize,
    /// Total number of output features.
    pub output_size: usize,
    /// Named regions in the input vector.
    pub input_regions: Vec<FeatureRegion>,
    /// Named regions in the output vector.
    pub output_regions: Vec<FeatureRegion>,
}

/// Return the complete schema metadata as a serializable struct.
///
/// This is exported to JSON so the Python training code can
/// build matching dataset loaders.
pub fn schema_metadata() -> SchemaMetadata {
    let mut input_offset = 0;
    let mut input_regions = Vec::new();

    let mut add_input = |name: &str, length: usize, desc: &str| {
        input_regions.push(FeatureRegion {
            name: name.to_string(),
            offset: input_offset,
            length,
            description: desc.to_string(),
        });
        input_offset += length;
    };

    add_input(
        "tensor",
        MAX_TENSOR_AXES * FEATURES_PER_AXIS,
        "Character tensor axes (value[4] + temporal_layer[4] + provenance[5]) × 16 slots",
    );
    add_input(
        "emotional_state",
        NUM_PRIMARIES * FEATURES_PER_PRIMARY,
        "Emotional primaries (intensity[1] + awareness[5]) × 8",
    );
    add_input(
        "self_edge",
        SELF_EDGE_FEATURES,
        "Self-edge: trust(3) + affection + debt + history_weight + projection_accuracy",
    );
    add_input(
        "relational_edges",
        MAX_EDGES * FEATURES_PER_EDGE,
        "Relational edges (substrate[20] + topology[4]) × 5 slots",
    );
    add_input(
        "scene_context",
        SCENE_FEATURES,
        "Scene type[4] + cast_size + tension",
    );
    add_input(
        "player_event",
        EVENT_FEATURES,
        "EventType[7] + EmotionalRegister[7] + confidence + target_count",
    );
    add_input(
        "history",
        HISTORY_DEPTH * FEATURES_PER_HISTORY,
        "Recent turns: (ActionType[6] + SpeechRegister[4] + AwarenessLevel[5] + valence) × 3",
    );

    let mut output_offset = 0;
    let mut output_regions = Vec::new();

    let mut add_output = |name: &str, length: usize, desc: &str| {
        output_regions.push(FeatureRegion {
            name: name.to_string(),
            offset: output_offset,
            length,
            description: desc.to_string(),
        });
        output_offset += length;
    };

    add_output(
        "action",
        NUM_ACTION_TYPES + 1 + 1 + 1 + NUM_ACTION_CONTEXTS,
        "ActionType[6] + confidence + target_index + valence + ActionContext[5]",
    );
    add_output(
        "speech",
        1 + NUM_SPEECH_REGISTERS + 1,
        "occurs + SpeechRegister[4] + confidence",
    );
    add_output(
        "thought",
        NUM_AWARENESS_LEVELS + 1,
        "AwarenessLevel[5] + dominant_emotion_index",
    );
    add_output(
        "emotion",
        NUM_PRIMARIES + NUM_PRIMARIES,
        "intensity_deltas[8] + awareness_shifts[8]",
    );

    SchemaMetadata {
        input_size: TOTAL_INPUT_FEATURES,
        output_size: TOTAL_OUTPUT_FEATURES,
        input_regions,
        output_regions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use storyteller_core::types::character::{
        CharacterTensor, EmotionalState, SelfEdgeTrust, SelfKnowledge,
    };
    use storyteller_core::types::entity::EntityId;
    use storyteller_core::types::world_model::CapabilityProfile;

    fn test_character_sheet() -> CharacterSheet {
        let mut tensor = CharacterTensor::new();
        tensor.insert(
            "empathy",
            AxisValue {
                central_tendency: 0.7,
                variance: 0.15,
                range_low: 0.3,
                range_high: 0.9,
            },
            TemporalLayer::Sediment,
            Provenance::Authored,
        );
        tensor.insert(
            "grief",
            AxisValue {
                central_tendency: 0.5,
                variance: 0.2,
                range_low: 0.1,
                range_high: 0.8,
            },
            TemporalLayer::Bedrock,
            Provenance::Authored,
        );

        CharacterSheet {
            entity_id: EntityId::new(),
            name: "Bramblehoof".to_string(),
            voice: "Test voice".to_string(),
            backstory: "Test backstory".to_string(),
            tensor,
            grammar_id: "plutchik_western".to_string(),
            emotional_state: EmotionalState {
                grammar_id: "plutchik_western".to_string(),
                primaries: vec![
                    EmotionalPrimary {
                        primary_id: "joy".to_string(),
                        intensity: 0.6,
                        awareness: AwarenessLevel::Articulate,
                    },
                    EmotionalPrimary {
                        primary_id: "trust".to_string(),
                        intensity: 0.4,
                        awareness: AwarenessLevel::Recognizable,
                    },
                    EmotionalPrimary {
                        primary_id: "fear".to_string(),
                        intensity: 0.2,
                        awareness: AwarenessLevel::Preconscious,
                    },
                    EmotionalPrimary {
                        primary_id: "surprise".to_string(),
                        intensity: 0.1,
                        awareness: AwarenessLevel::Articulate,
                    },
                    EmotionalPrimary {
                        primary_id: "sadness".to_string(),
                        intensity: 0.5,
                        awareness: AwarenessLevel::Defended,
                    },
                    EmotionalPrimary {
                        primary_id: "disgust".to_string(),
                        intensity: 0.05,
                        awareness: AwarenessLevel::Structural,
                    },
                    EmotionalPrimary {
                        primary_id: "anger".to_string(),
                        intensity: 0.1,
                        awareness: AwarenessLevel::Preconscious,
                    },
                    EmotionalPrimary {
                        primary_id: "anticipation".to_string(),
                        intensity: 0.7,
                        awareness: AwarenessLevel::Articulate,
                    },
                ],
                mood_vector_notes: vec!["joy + anticipation → eager hope".to_string()],
            },
            self_edge: SelfEdge {
                trust: SelfEdgeTrust {
                    competence: 0.7,
                    intentions: 0.6,
                    reliability: 0.5,
                },
                affection: 0.4,
                debt: 0.3,
                history_pattern: "Test pattern".to_string(),
                history_weight: 0.6,
                projection_content: "Test projection".to_string(),
                projection_accuracy: 0.5,
                self_knowledge: SelfKnowledge {
                    knows: vec!["Test knowledge".to_string()],
                    does_not_know: vec!["Test unknown".to_string()],
                },
            },
            triggers: vec![],
            performance_notes: "Test notes".to_string(),
            knows: vec![],
            does_not_know: vec![],
            capabilities: CapabilityProfile::default(),
        }
    }

    fn test_prediction_input(sheet: &CharacterSheet) -> PredictionInput<'_> {
        PredictionInput {
            character: sheet,
            edges: &[],
            target_roles: &[],
            scene: SceneFeatureInput {
                scene_type: SceneType::Gravitational,
                cast_size: 2,
                tension: 0.6,
            },
            event: EventFeatureInput {
                event_type: EventType::Speech,
                emotional_register: EmotionalRegister::Tender,
                confidence: 0.9,
                target_count: 1,
            },
            history: &[],
        }
    }

    #[test]
    fn total_features_constant_is_correct() {
        // Verify the arithmetic manually
        let expected = 16 * 13  // tensor
            + 8 * 6             // emotions
            + 7                 // self-edge
            + 5 * 24            // edges
            + 6                 // scene
            + 16                // event
            + 3 * 16; // history
        assert_eq!(TOTAL_INPUT_FEATURES, expected);
        assert_eq!(TOTAL_INPUT_FEATURES, 453);
    }

    #[test]
    fn total_output_constant_is_correct() {
        let expected = 6 + 1 + 1 + 1 + 5  // action
            + 1 + 4 + 1                     // speech
            + 5 + 1                          // thought
            + 8 + 8; // emotion
        assert_eq!(TOTAL_OUTPUT_FEATURES, expected);
        assert_eq!(TOTAL_OUTPUT_FEATURES, 42);
    }

    #[test]
    fn encode_produces_correct_length() {
        let sheet = test_character_sheet();
        let input = test_prediction_input(&sheet);
        let features = encode_features(&input);
        assert_eq!(features.len(), TOTAL_INPUT_FEATURES);
    }

    #[test]
    fn encode_tensor_region_captures_values() {
        let sheet = test_character_sheet();
        let input = test_prediction_input(&sheet);
        let features = encode_features(&input);

        // First axis ("empathy" — BTreeMap sorts alphabetically) is "empathy"
        // AxisValue: central_tendency=0.7, variance=0.15, range_low=0.3, range_high=0.9
        assert!((features[0] - 0.7).abs() < f32::EPSILON);
        assert!((features[1] - 0.15).abs() < f32::EPSILON);
        assert!((features[2] - 0.3).abs() < f32::EPSILON);
        assert!((features[3] - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn encode_emotional_region_captures_intensities() {
        let sheet = test_character_sheet();
        let input = test_prediction_input(&sheet);
        let features = encode_features(&input);

        // Emotional state starts after tensor region
        let emo_start = MAX_TENSOR_AXES * FEATURES_PER_AXIS;
        // First primary (joy): intensity = 0.6
        assert!((features[emo_start] - 0.6).abs() < f32::EPSILON);
        // Awareness = Articulate → [1, 0, 0, 0, 0]
        assert!((features[emo_start + 1] - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn encode_self_edge_captures_trust() {
        let sheet = test_character_sheet();
        let input = test_prediction_input(&sheet);
        let features = encode_features(&input);

        let se_start = MAX_TENSOR_AXES * FEATURES_PER_AXIS + NUM_PRIMARIES * FEATURES_PER_PRIMARY;
        // trust.competence = 0.7
        assert!((features[se_start] - 0.7).abs() < f32::EPSILON);
        // trust.intentions = 0.6
        assert!((features[se_start + 1] - 0.6).abs() < f32::EPSILON);
    }

    #[test]
    fn empty_edges_produce_zeros() {
        let sheet = test_character_sheet();
        let input = test_prediction_input(&sheet);
        let features = encode_features(&input);

        let edge_start = MAX_TENSOR_AXES * FEATURES_PER_AXIS
            + NUM_PRIMARIES * FEATURES_PER_PRIMARY
            + SELF_EDGE_FEATURES;
        // All edge features should be 0.0 when no edges provided
        for i in 0..MAX_EDGES * FEATURES_PER_EDGE {
            assert!(
                features[edge_start + i].abs() < f32::EPSILON,
                "Edge feature at offset {} should be 0.0, got {}",
                i,
                features[edge_start + i]
            );
        }
    }

    #[test]
    fn empty_history_produces_zeros() {
        let sheet = test_character_sheet();
        let input = test_prediction_input(&sheet);
        let features = encode_features(&input);

        let hist_start = TOTAL_INPUT_FEATURES - HISTORY_DEPTH * FEATURES_PER_HISTORY;
        for i in 0..HISTORY_DEPTH * FEATURES_PER_HISTORY {
            assert!(
                features[hist_start + i].abs() < f32::EPSILON,
                "History feature at offset {} should be 0.0, got {}",
                i,
                features[hist_start + i]
            );
        }
    }

    #[test]
    fn decode_roundtrip_action_type() {
        // Construct a fake output vector with known values
        let mut output = vec![0.0f32; TOTAL_OUTPUT_FEATURES];

        // Action head: ActionType = Perform (index 0 should be highest)
        output[0] = 0.9; // Perform
        output[6] = 0.85; // confidence
        output[7] = -1.0; // target_index (no target)
        output[8] = 0.6; // emotional_valence
        output[9] = 0.8; // ActionContext = SharedHistory

        // Speech head: occurs = true
        output[14] = 0.8; // speech_occurs > 0.5
        output[15] = 0.0; // Whisper
        output[16] = 0.9; // Conversational (highest)
        output[18] = 0.7; // confidence

        // Thought head: Awareness = Recognizable
        output[20] = 0.5; // Recognizable (highest)
        output[24] = 0.0; // dominant_emotion_index = 0 (joy)

        let character_id = EntityId::new();
        let pred = decode_outputs(&output, character_id, vec![0, 3, 7], 0.8);

        assert_eq!(pred.action.action_type, ActionType::Perform);
        assert!((pred.action.confidence - 0.85).abs() < f32::EPSILON);
        assert!((pred.action.emotional_valence - 0.6).abs() < f32::EPSILON);
        assert_eq!(pred.action.action_context, ActionContext::SharedHistory);
        assert!(pred.speech.occurs);
        assert_eq!(pred.speech.register, SpeechRegister::Conversational);
        assert_eq!(pred.thought.awareness_level, AwarenessLevel::Articulate);
        assert_eq!(pred.thought.dominant_emotion_index, 0);
        assert_eq!(pred.frame.activated_axis_indices, vec![0, 3, 7]);
    }

    #[test]
    fn decode_filters_trivial_deltas() {
        let mut output = vec![0.0f32; TOTAL_OUTPUT_FEATURES];
        // Set action head defaults so it doesn't panic
        output[0] = 1.0;

        // Emotional deltas: only index 0 and 4 have significant changes
        let delta_start = TOTAL_OUTPUT_FEATURES - 2 * NUM_PRIMARIES;
        output[delta_start] = 0.2; // joy: significant
        output[delta_start + 1] = 0.005; // trust: trivial, should be filtered
        output[delta_start + 4] = -0.15; // sadness: significant

        let pred = decode_outputs(&output, EntityId::new(), vec![], 0.8);

        // Only joy and sadness should appear
        assert_eq!(pred.emotional_deltas.len(), 2);
        assert_eq!(pred.emotional_deltas[0].primary_index, 0);
        assert_eq!(pred.emotional_deltas[1].primary_index, 4);
    }

    #[test]
    fn schema_metadata_is_consistent() {
        let meta = schema_metadata();
        assert_eq!(meta.input_size, TOTAL_INPUT_FEATURES);
        assert_eq!(meta.output_size, TOTAL_OUTPUT_FEATURES);

        // Input regions should cover the full vector without gaps
        let total_input_from_regions: usize = meta.input_regions.iter().map(|r| r.length).sum();
        assert_eq!(total_input_from_regions, TOTAL_INPUT_FEATURES);

        // Output regions should cover the full vector without gaps
        let total_output_from_regions: usize = meta.output_regions.iter().map(|r| r.length).sum();
        assert_eq!(total_output_from_regions, TOTAL_OUTPUT_FEATURES);

        // Regions should be contiguous
        let mut expected_offset = 0;
        for region in &meta.input_regions {
            assert_eq!(
                region.offset, expected_offset,
                "Input region '{}' starts at {} but expected {}",
                region.name, region.offset, expected_offset
            );
            expected_offset += region.length;
        }
    }

    #[test]
    fn schema_metadata_is_serializable() {
        let meta = schema_metadata();
        let json = serde_json::to_string_pretty(&meta).expect("schema should serialize");
        assert!(json.contains("\"input_size\": 453"));
        assert!(json.contains("\"output_size\": 42"));
    }

    #[test]
    fn encode_with_edges() {
        let sheet = test_character_sheet();
        let target_id = EntityId::new();
        let edges = vec![DirectedEdge {
            source: sheet.entity_id,
            target: target_id,
            substrate: RelationalSubstrate {
                trust_reliability: AxisValue {
                    central_tendency: 0.6,
                    variance: 0.1,
                    range_low: 0.3,
                    range_high: 0.8,
                },
                trust_competence: AxisValue {
                    central_tendency: 0.5,
                    variance: 0.15,
                    range_low: 0.2,
                    range_high: 0.7,
                },
                trust_benevolence: AxisValue {
                    central_tendency: 0.7,
                    variance: 0.1,
                    range_low: 0.4,
                    range_high: 0.9,
                },
                affection: AxisValue {
                    central_tendency: 0.6,
                    variance: 0.2,
                    range_low: 0.2,
                    range_high: 0.9,
                },
                debt: AxisValue {
                    central_tendency: 0.3,
                    variance: 0.1,
                    range_low: 0.0,
                    range_high: 0.5,
                },
            },
            information_state: storyteller_core::types::relational::InformationState {
                known_facts: vec![],
                beliefs: vec![],
                blind_spots: vec![],
            },
        }];
        let roles = vec![TopologicalRole::Periphery];

        let input = PredictionInput {
            character: &sheet,
            edges: &edges,
            target_roles: &roles,
            scene: SceneFeatureInput {
                scene_type: SceneType::Gravitational,
                cast_size: 2,
                tension: 0.6,
            },
            event: EventFeatureInput {
                event_type: EventType::Speech,
                emotional_register: EmotionalRegister::Tender,
                confidence: 0.9,
                target_count: 1,
            },
            history: &[],
        };

        let features = encode_features(&input);
        assert_eq!(features.len(), TOTAL_INPUT_FEATURES);

        // First edge's first substrate value (trust_reliability.central_tendency = 0.6)
        let edge_start = MAX_TENSOR_AXES * FEATURES_PER_AXIS
            + NUM_PRIMARIES * FEATURES_PER_PRIMARY
            + SELF_EDGE_FEATURES;
        assert!((features[edge_start] - 0.6).abs() < f32::EPSILON);
    }

    #[test]
    fn encode_with_history() {
        let sheet = test_character_sheet();
        let history = vec![
            HistoryEntry {
                action_type: ActionType::Speak,
                speech_register: SpeechRegister::Conversational,
                awareness_level: AwarenessLevel::Articulate,
                emotional_valence: 0.5,
            },
            HistoryEntry {
                action_type: ActionType::Examine,
                speech_register: SpeechRegister::Internal,
                awareness_level: AwarenessLevel::Preconscious,
                emotional_valence: -0.2,
            },
        ];

        let input = PredictionInput {
            character: &sheet,
            edges: &[],
            target_roles: &[],
            scene: SceneFeatureInput {
                scene_type: SceneType::Gravitational,
                cast_size: 2,
                tension: 0.6,
            },
            event: EventFeatureInput {
                event_type: EventType::Speech,
                emotional_register: EmotionalRegister::Tender,
                confidence: 0.9,
                target_count: 1,
            },
            history: &history,
        };

        let features = encode_features(&input);
        assert_eq!(features.len(), TOTAL_INPUT_FEATURES);

        // History region starts at known offset
        let hist_start = TOTAL_INPUT_FEATURES - HISTORY_DEPTH * FEATURES_PER_HISTORY;

        // First history entry: ActionType::Speak = [0, 1, 0, 0, 0, 0]
        assert!((features[hist_start] - 0.0).abs() < f32::EPSILON); // Not Perform
        assert!((features[hist_start + 1] - 1.0).abs() < f32::EPSILON); // Speak

        // Third history slot should be zeros (only 2 entries provided)
        let third_start = hist_start + 2 * FEATURES_PER_HISTORY;
        for i in 0..FEATURES_PER_HISTORY {
            assert!(features[third_start + i].abs() < f32::EPSILON);
        }
    }
}
