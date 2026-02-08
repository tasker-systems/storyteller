//! Character prediction via ONNX Runtime.
//!
//! See: `docs/technical/narrator-architecture.md`
//!
//! Loads a trained ONNX model and runs character prediction inference.
//! Uses `storyteller_ml::feature_schema` for encoding inputs and decoding
//! outputs — the same schema used by the Python training pipeline.
//!
//! Compute isolation: inference runs on a dedicated rayon thread pool,
//! separate from the tokio async runtime.

use std::path::Path;
use std::sync::Mutex;

use ort::session::Session;
use ort::value::Tensor;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use storyteller_core::errors::StorytellerError;
use storyteller_core::types::entity::EntityId;
use storyteller_core::types::prediction::RawCharacterPrediction;
use storyteller_ml::feature_schema::{
    self, PredictionInput, TOTAL_INPUT_FEATURES, TOTAL_OUTPUT_FEATURES,
};

/// ONNX model output tensor names, in the order they are concatenated
/// to form the flat output vector consumed by `decode_outputs()`.
const OUTPUT_NAMES: [&str; 4] = ["action", "speech", "thought", "emotion"];

/// Loads an ONNX character prediction model and runs inference.
///
/// The predictor owns a dedicated rayon thread pool for compute isolation.
/// Inference is CPU-bound and must not block the tokio async runtime.
///
/// # Session mutability
///
/// `ort::Session::run` requires `&mut self`. The session is wrapped in a
/// `Mutex` so that `predict` can take `&self`. For a 38KB model, inference
/// is sub-millisecond, so lock contention is negligible even under batch
/// workloads.
#[derive(Debug)]
pub struct CharacterPredictor {
    session: Mutex<Session>,
    pool: rayon::ThreadPool,
}

impl CharacterPredictor {
    /// Load the ONNX model from disk and create a dedicated rayon pool.
    ///
    /// # Errors
    ///
    /// Returns `StorytellerError::Inference` if the model file cannot be
    /// loaded or is not a valid ONNX model.
    pub fn load(model_path: &Path) -> Result<Self, StorytellerError> {
        let session = Session::builder()
            .and_then(|b| b.with_intra_threads(1))
            .and_then(|b| b.commit_from_file(model_path))
            .map_err(|e| StorytellerError::Inference(e.to_string()))?;

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .thread_name(|i| format!("ml-predict-{i}"))
            .build()
            .map_err(|e| StorytellerError::Inference(e.to_string()))?;

        Ok(Self {
            session: Mutex::new(session),
            pool,
        })
    }

    /// Run inference for a single character, returning raw predictions.
    ///
    /// The caller provides the `PredictionInput` (character sheet, edges,
    /// scene context, etc.), plus the activated axis indices and frame
    /// confidence that are passed through to the output.
    ///
    /// # Errors
    ///
    /// Returns `StorytellerError::Inference` if encoding, inference, or
    /// decoding fails.
    pub fn predict(
        &self,
        input: &PredictionInput<'_>,
        character_id: EntityId,
        activated_axis_indices: Vec<u16>,
        frame_confidence: f32,
    ) -> Result<RawCharacterPrediction, StorytellerError> {
        // 1. Encode features
        let features = feature_schema::encode_features(input);
        if features.len() != TOTAL_INPUT_FEATURES {
            return Err(StorytellerError::Inference(format!(
                "input feature length mismatch: got {}, expected {TOTAL_INPUT_FEATURES}",
                features.len(),
            )));
        }

        // 2. Build input tensor [1, 453]
        let tensor = Tensor::from_array(([1usize, TOTAL_INPUT_FEATURES], features))
            .map_err(|e| StorytellerError::Inference(e.to_string()))?;

        // 3. Run inference
        let mut session = self
            .session
            .lock()
            .map_err(|e| StorytellerError::Inference(format!("session lock poisoned: {e}")))?;

        let outputs = session
            .run(ort::inputs!["features" => tensor])
            .map_err(|e| StorytellerError::Inference(e.to_string()))?;

        // 4. Extract and concatenate output tensors into flat [42]
        let mut flat = Vec::with_capacity(TOTAL_OUTPUT_FEATURES);
        for name in &OUTPUT_NAMES {
            let output = outputs
                .get(*name)
                .ok_or_else(|| StorytellerError::Inference(format!("missing output: {name}")))?;
            let array = output
                .try_extract_array::<f32>()
                .map_err(|e| StorytellerError::Inference(e.to_string()))?;
            flat.extend(array.iter().copied());
        }

        // Drop outputs before session lock is released (outputs borrow session)
        drop(outputs);
        drop(session);

        if flat.len() != TOTAL_OUTPUT_FEATURES {
            return Err(StorytellerError::Inference(format!(
                "output length mismatch: got {}, expected {}",
                flat.len(),
                TOTAL_OUTPUT_FEATURES,
            )));
        }

        // 5. Decode into structured prediction
        feature_schema::decode_outputs(
            &flat,
            character_id,
            activated_axis_indices,
            frame_confidence,
        )
    }

    /// Run inference for multiple characters in parallel via rayon.
    ///
    /// Each tuple contains the prediction input, character ID, activated
    /// axis indices, and frame confidence.
    pub fn predict_batch(
        &self,
        inputs: &[(PredictionInput<'_>, EntityId, Vec<u16>, f32)],
    ) -> Vec<Result<RawCharacterPrediction, StorytellerError>> {
        self.pool.install(|| {
            inputs
                .par_iter()
                .map(|(input, character_id, axes, confidence)| {
                    self.predict(input, *character_id, axes.clone(), *confidence)
                })
                .collect()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_nonexistent_model_errors() {
        let result = CharacterPredictor::load(Path::new("/nonexistent/model.onnx"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, StorytellerError::Inference(_)),
            "expected Inference error, got: {err:?}"
        );
    }

    /// Tests requiring an ONNX model on disk.
    ///
    /// Enable with: `cargo test --features test-ml-model`
    /// Requires `STORYTELLER_MODEL_PATH` or `STORYTELLER_DATA_PATH` env var.
    #[cfg(feature = "test-ml-model")]
    mod with_model {
        use super::*;
        use std::path::PathBuf;

        use storyteller_core::types::character::{
            CharacterSheet, CharacterTensor, EmotionalPrimary, EmotionalState, SelfEdge,
            SelfEdgeTrust, SelfKnowledge,
        };
        use storyteller_core::types::prediction::{
            ActionType, EmotionalRegister, EventType, SpeechRegister,
        };
        use storyteller_core::types::scene::SceneType;
        use storyteller_core::types::tensor::{
            AwarenessLevel, AxisValue, Provenance, TemporalLayer,
        };
        use storyteller_core::types::world_model::CapabilityProfile;
        use storyteller_ml::feature_schema::{EventFeatureInput, SceneFeatureInput};

        fn model_path() -> PathBuf {
            let data_path = std::env::var("STORYTELLER_MODEL_PATH")
                .or_else(|_| std::env::var("STORYTELLER_DATA_PATH").map(|p| format!("{p}/models")))
                .expect("STORYTELLER_MODEL_PATH or STORYTELLER_DATA_PATH must be set");
            PathBuf::from(data_path).join("character_predictor.onnx")
        }

        fn test_character_sheet(name: &str) -> CharacterSheet {
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
                name: name.to_string(),
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
                    mood_vector_notes: vec![],
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
                        knows: vec![],
                        does_not_know: vec![],
                    },
                },
                triggers: vec![],
                performance_notes: String::new(),
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
        fn predict_with_real_model() {
            let predictor = CharacterPredictor::load(&model_path()).expect("model should load");

            let sheet = test_character_sheet("Bramblehoof");
            let input = test_prediction_input(&sheet);

            let prediction = predictor
                .predict(&input, sheet.entity_id, vec![0, 1], 0.8)
                .expect("inference should succeed");

            // Verify structural validity
            assert_eq!(prediction.character_id, sheet.entity_id);
            assert_eq!(prediction.frame.activated_axis_indices, vec![0, 1]);
            assert!((prediction.frame.confidence - 0.8).abs() < f32::EPSILON);

            // Action type should be a valid variant (any of the 6)
            let valid_actions = [
                ActionType::Perform,
                ActionType::Speak,
                ActionType::Move,
                ActionType::Examine,
                ActionType::Wait,
                ActionType::Resist,
            ];
            assert!(
                valid_actions.contains(&prediction.action.action_type),
                "invalid action type: {:?}",
                prediction.action.action_type
            );

            // Speech register should be valid
            let valid_registers = [
                SpeechRegister::Whisper,
                SpeechRegister::Conversational,
                SpeechRegister::Declamatory,
                SpeechRegister::Internal,
            ];
            assert!(
                valid_registers.contains(&prediction.speech.register),
                "invalid speech register: {:?}",
                prediction.speech.register
            );

            // Awareness level should be valid
            let valid_awareness = [
                AwarenessLevel::Articulate,
                AwarenessLevel::Recognizable,
                AwarenessLevel::Preconscious,
                AwarenessLevel::Defended,
                AwarenessLevel::Structural,
            ];
            assert!(
                valid_awareness.contains(&prediction.thought.awareness_level),
                "invalid awareness level: {:?}",
                prediction.thought.awareness_level
            );
        }

        #[test]
        fn predict_batch_runs_parallel() {
            let predictor = CharacterPredictor::load(&model_path()).expect("model should load");

            let bramblehoof = test_character_sheet("Bramblehoof");
            let pyotir = test_character_sheet("Pyotir");

            let bramblehoof_input = test_prediction_input(&bramblehoof);
            let pyotir_input = test_prediction_input(&pyotir);

            let batch = vec![
                (bramblehoof_input, bramblehoof.entity_id, vec![0, 1], 0.8f32),
                (pyotir_input, pyotir.entity_id, vec![2, 3], 0.7f32),
            ];

            let results = predictor.predict_batch(&batch);

            assert_eq!(results.len(), 2);
            let pred_b = results[0].as_ref().expect("bramblehoof prediction");
            let pred_p = results[1].as_ref().expect("pyotir prediction");

            assert_eq!(pred_b.character_id, bramblehoof.entity_id);
            assert_eq!(pred_p.character_id, pyotir.entity_id);
            assert_ne!(pred_b.character_id, pred_p.character_id);
        }
    }
}
