//! Event classification via ONNX Runtime + HuggingFace tokenizers.
//!
//! See: `docs/ticket-specs/event-system-foundations/phase-c-ml-classification-pipeline.md`
//!
//! Classifies natural language text into EventKind labels and extracts entity
//! mentions, producing the types defined in `storyteller-core::types::event_grammar`.
//!
//! Follows the same pattern as [`super::frame::CharacterPredictor`]:
//! - ONNX models loaded via `ort::Session`
//! - Sessions wrapped in `Mutex` (ort `run()` needs `&mut self`)
//! - Dedicated rayon thread pool for compute isolation
//!
//! The tokenizer (`tokenizers` crate) is the HuggingFace reference
//! implementation — the same code that backs the Python `tokenizers` package.
//! It is `Send + Sync` and does not require a mutex.

use std::path::Path;
use std::sync::Mutex;

use ort::session::Session;
use ort::value::Tensor;
use tokenizers::Tokenizer;

use storyteller_core::errors::StorytellerError;
use storyteller_ml::event_labels::{
    self, BIO_LABELS, EVENT_KIND_LABELS, MAX_SEQ_LENGTH, NUM_BIO_LABELS, NUM_EVENT_KINDS,
};
use storyteller_ml::event_templates::NerCategory;

// ===========================================================================
// Output types
// ===========================================================================

/// Output of event classification and entity extraction for a single text.
///
/// Produced by [`EventClassifier::classify_text`]. The pipeline orchestration
/// layer (turn cycle system) is responsible for any downstream conversion
/// (e.g., mapping event kinds to the character prediction model's feature
/// input).
#[derive(Debug, Clone)]
pub struct ClassificationOutput {
    /// Event kind labels with confidence scores (sigmoid-activated).
    /// Only labels above the classification threshold are included.
    pub event_kinds: Vec<(String, f32)>,
    /// Entity mentions extracted via NER with BIO span assembly.
    pub entity_mentions: Vec<ExtractedEntity>,
}

/// An entity mention extracted from text via NER.
#[derive(Debug, Clone)]
pub struct ExtractedEntity {
    /// The extracted text span.
    pub text: String,
    /// Character offset start (inclusive) in the original text.
    pub start: usize,
    /// Character offset end (exclusive) in the original text.
    pub end: usize,
    /// Entity category from the NER model.
    pub category: NerCategory,
    /// Average softmax confidence across tokens in this span.
    pub confidence: f32,
}

// ===========================================================================
// EventClassifier
// ===========================================================================

/// Loads ONNX classification model(s) and tokenizer, runs inference to
/// classify text into event kinds and extract entity mentions.
///
/// # Architecture
///
/// Wraps separate ONNX models for event classification and entity
/// extraction (Approach B from the implementation plan). The public API is
/// designed so that consolidation to a single multi-task model (Approach A)
/// requires no caller changes.
///
/// # Thread safety
///
/// `ort::Session::run` requires `&mut self`. Sessions are wrapped in `Mutex`
/// so that `classify_text` can take `&self`. For transformer models (~5-15ms
/// per forward pass), lock contention is manageable under sequential turn
/// processing.
///
/// The `Tokenizer` is `Send + Sync` and shared without locking.
#[derive(Debug)]
pub struct EventClassifier {
    /// Event classification model (sequence-level [CLS] head).
    /// Produces EventKind logits from tokenized text.
    event_session: Mutex<Session>,
    /// NER model (token-level BIO head).
    /// Produces BIO label logits per token.
    ner_session: Mutex<Session>,
    /// Shared tokenizer loaded from `tokenizer.json`.
    tokenizer: Tokenizer,
    /// Dedicated thread pool for CPU-bound inference.
    pool: rayon::ThreadPool,
}

impl EventClassifier {
    /// Load the ONNX models and tokenizer from a directory.
    ///
    /// Expects:
    /// - `model_dir/event_classifier.onnx` — event classification model
    /// - `model_dir/ner_classifier.onnx` — NER entity extraction model
    /// - `model_dir/tokenizer.json` — HuggingFace tokenizer configuration
    ///
    /// # Errors
    ///
    /// Returns `StorytellerError::Inference` if any model or tokenizer file
    /// cannot be loaded.
    pub fn load(model_dir: &Path) -> Result<Self, StorytellerError> {
        let tokenizer_path = model_dir.join("tokenizer.json");
        let tokenizer = Tokenizer::from_file(&tokenizer_path).map_err(|e| {
            StorytellerError::Inference(format!(
                "failed to load tokenizer from {}: {e}",
                tokenizer_path.display()
            ))
        })?;

        let event_model_path = model_dir.join("event_classifier.onnx");
        let event_session = Session::builder()
            .and_then(|b| b.with_intra_threads(1))
            .and_then(|b| b.commit_from_file(&event_model_path))
            .map_err(|e| {
                StorytellerError::Inference(format!(
                    "failed to load event classifier from {}: {e}",
                    event_model_path.display()
                ))
            })?;

        let ner_model_path = model_dir.join("ner_classifier.onnx");
        let ner_session = Session::builder()
            .and_then(|b| b.with_intra_threads(1))
            .and_then(|b| b.commit_from_file(&ner_model_path))
            .map_err(|e| {
                StorytellerError::Inference(format!(
                    "failed to load NER classifier from {}: {e}",
                    ner_model_path.display()
                ))
            })?;

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .thread_name(|i| format!("event-classify-{i}"))
            .build()
            .map_err(|e| StorytellerError::Inference(e.to_string()))?;

        Ok(Self {
            event_session: Mutex::new(event_session),
            ner_session: Mutex::new(ner_session),
            tokenizer,
            pool,
        })
    }

    /// Tokenize input text and return token IDs, attention mask, and
    /// character-level offset mappings.
    ///
    /// This is the first stage of the classification pipeline. Exposed
    /// as a public method for testing and debugging tokenization behavior
    /// independently of model inference.
    ///
    /// # Errors
    ///
    /// Returns `StorytellerError::Inference` if tokenization fails.
    pub fn tokenize(&self, input: &str) -> Result<TokenizedInput, StorytellerError> {
        let encoding = self
            .tokenizer
            .encode(input, true)
            .map_err(|e| StorytellerError::Inference(format!("tokenization failed: {e}")))?;

        Ok(TokenizedInput {
            token_ids: encoding.get_ids().to_vec(),
            attention_mask: encoding.get_attention_mask().to_vec(),
            offsets: encoding.get_offsets().to_vec(),
            word_ids: encoding.get_word_ids().to_vec(),
        })
    }

    /// Classify text into event kinds and extract entity mentions.
    ///
    /// Full pipeline:
    /// 1. Tokenize input text
    /// 2. Pad/truncate to `MAX_SEQ_LENGTH`
    /// 3. Run event classification model → sigmoid → threshold
    /// 4. Run NER model → argmax → BIO span assembly
    ///
    /// # Errors
    ///
    /// Returns `StorytellerError::Inference` on tokenization, tensor
    /// construction, or model execution failure.
    pub fn classify_text(&self, input: &str) -> Result<ClassificationOutput, StorytellerError> {
        self.pool.install(|| self.classify_text_inner(input))
    }

    fn classify_text_inner(&self, input: &str) -> Result<ClassificationOutput, StorytellerError> {
        // 1. Tokenize
        let tokenized = self.tokenize(input)?;

        // 2. Pad/truncate to fixed length
        let (token_ids, attention_mask) = pad_or_truncate(&tokenized, MAX_SEQ_LENGTH);

        // 3. Build input tensors [1, MAX_SEQ_LENGTH] as i64
        let ids_tensor = Tensor::from_array(([1usize, MAX_SEQ_LENGTH], token_ids.clone()))
            .map_err(|e| StorytellerError::Inference(format!("input_ids tensor: {e}")))?;
        let mask_tensor = Tensor::from_array(([1usize, MAX_SEQ_LENGTH], attention_mask.clone()))
            .map_err(|e| StorytellerError::Inference(format!("attention_mask tensor: {e}")))?;

        // 4. Event classification
        let event_kinds = {
            let mut session = self.event_session.lock().map_err(|e| {
                StorytellerError::Inference(format!("event session lock poisoned: {e}"))
            })?;

            let outputs = session
                .run(ort::inputs!["input_ids" => ids_tensor, "attention_mask" => mask_tensor])
                .map_err(|e| {
                    StorytellerError::Inference(format!("event classifier forward pass: {e}"))
                })?;

            let logits_value = outputs.get("logits").ok_or_else(|| {
                StorytellerError::Inference("event model missing 'logits' output".into())
            })?;
            let logits_array = logits_value
                .try_extract_array::<f32>()
                .map_err(|e| StorytellerError::Inference(format!("extract event logits: {e}")))?;
            let logits: Vec<f32> = logits_array.iter().copied().collect();

            drop(outputs);
            drop(session);

            decode_event_logits(&logits, 0.5)
        };

        // 5. NER classification — rebuild tensors (consumed by event model)
        let ids_tensor2 = Tensor::from_array(([1usize, MAX_SEQ_LENGTH], token_ids))
            .map_err(|e| StorytellerError::Inference(format!("input_ids tensor (NER): {e}")))?;
        let mask_tensor2 =
            Tensor::from_array(([1usize, MAX_SEQ_LENGTH], attention_mask)).map_err(|e| {
                StorytellerError::Inference(format!("attention_mask tensor (NER): {e}"))
            })?;

        let entity_mentions = {
            let mut session = self.ner_session.lock().map_err(|e| {
                StorytellerError::Inference(format!("NER session lock poisoned: {e}"))
            })?;

            let outputs = session
                .run(ort::inputs!["input_ids" => ids_tensor2, "attention_mask" => mask_tensor2])
                .map_err(|e| {
                    StorytellerError::Inference(format!("NER classifier forward pass: {e}"))
                })?;

            let logits_value = outputs.get("logits").ok_or_else(|| {
                StorytellerError::Inference("NER model missing 'logits' output".into())
            })?;
            let logits_array = logits_value
                .try_extract_array::<f32>()
                .map_err(|e| StorytellerError::Inference(format!("extract NER logits: {e}")))?;
            let logits: Vec<f32> = logits_array.iter().copied().collect();

            drop(outputs);
            drop(session);

            // Actual sequence length before padding (capped at MAX_SEQ_LENGTH)
            let seq_len = tokenized.token_ids.len().min(MAX_SEQ_LENGTH);

            assemble_entity_spans(
                &logits,
                seq_len,
                &tokenized.offsets,
                &tokenized.word_ids,
                input,
            )
        };

        Ok(ClassificationOutput {
            event_kinds,
            entity_mentions,
        })
    }
}

// ===========================================================================
// Tokenized input
// ===========================================================================

/// Tokenized text ready for model inference.
///
/// Provides token IDs for the model, attention mask for padding awareness,
/// character offsets for mapping predictions back to text spans, and word
/// IDs for grouping subword tokens into original words (critical for NER
/// span assembly).
#[derive(Debug, Clone)]
pub struct TokenizedInput {
    /// Token IDs for the model vocabulary.
    pub token_ids: Vec<u32>,
    /// Attention mask (1 for real tokens, 0 for padding).
    pub attention_mask: Vec<u32>,
    /// Character-level offsets: `(start, end)` in the original text.
    /// Special tokens ([CLS], [SEP]) have offset `(0, 0)`.
    pub offsets: Vec<(usize, usize)>,
    /// Word IDs: maps each token to its original word index.
    /// `None` for special tokens. Used to group subword tokens
    /// (e.g., "playing" → ["play", "##ing"] both map to word 0).
    pub word_ids: Vec<Option<u32>>,
}

/// Load a tokenizer from a file path, independent of any model.
///
/// Useful for testing tokenization behavior without loading ONNX models.
///
/// # Errors
///
/// Returns `StorytellerError::Inference` if the file cannot be loaded.
pub fn load_tokenizer(path: &Path) -> Result<Tokenizer, StorytellerError> {
    Tokenizer::from_file(path).map_err(|e| {
        StorytellerError::Inference(format!(
            "failed to load tokenizer from {}: {e}",
            path.display()
        ))
    })
}

// ===========================================================================
// Decoding helpers
// ===========================================================================

/// Pad or truncate tokenized input to exactly `max_len` tokens.
///
/// Returns `(token_ids, attention_mask)` as i64 vectors ready for ONNX.
/// Token IDs are padded with 0 (PAD token); attention mask with 0.
/// Truncation preserves the first `max_len` tokens (including [CLS]).
fn pad_or_truncate(input: &TokenizedInput, max_len: usize) -> (Vec<i64>, Vec<i64>) {
    let len = input.token_ids.len().min(max_len);

    let mut token_ids = Vec::with_capacity(max_len);
    let mut attention_mask = Vec::with_capacity(max_len);

    for i in 0..len {
        token_ids.push(i64::from(input.token_ids[i]));
        attention_mask.push(i64::from(input.attention_mask[i]));
    }

    // Pad remaining positions
    token_ids.resize(max_len, 0);
    attention_mask.resize(max_len, 0);

    (token_ids, attention_mask)
}

fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

/// Apply sigmoid to event logits and return labels above threshold.
fn decode_event_logits(logits: &[f32], threshold: f32) -> Vec<(String, f32)> {
    logits
        .iter()
        .enumerate()
        .take(NUM_EVENT_KINDS)
        .filter_map(|(i, &logit)| {
            let conf = sigmoid(logit);
            if conf > threshold {
                Some((EVENT_KIND_LABELS[i].to_string(), conf))
            } else {
                None
            }
        })
        .collect()
}

/// Assemble contiguous BIO-tagged token spans into entity mentions.
///
/// Algorithm:
/// 1. Argmax over `NUM_BIO_LABELS` classes per token
/// 2. Walk left-to-right: B-X starts a span, I-X continues if category
///    matches, anything else emits the current span
/// 3. Map token positions to character offsets via `offsets`
fn assemble_entity_spans(
    bio_logits: &[f32],
    seq_len: usize,
    offsets: &[(usize, usize)],
    word_ids: &[Option<u32>],
    original_text: &str,
) -> Vec<ExtractedEntity> {
    let mut entities = Vec::new();

    // Per-token: argmax label ID + softmax confidence for that label
    let mut token_labels: Vec<(usize, f32)> = Vec::with_capacity(seq_len);
    for t in 0..seq_len {
        let row_start = t * NUM_BIO_LABELS;
        let row_end = row_start + NUM_BIO_LABELS;
        if row_end > bio_logits.len() {
            break;
        }
        let row = &bio_logits[row_start..row_end];

        // Argmax
        let (best_idx, &best_logit) = row
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or((0, &0.0));

        // Softmax confidence for the argmax label
        let max_logit = best_logit;
        let exp_sum: f32 = row.iter().map(|&l| (l - max_logit).exp()).sum();
        let confidence = 1.0 / exp_sum; // exp(0) / exp_sum

        token_labels.push((best_idx, confidence));
    }

    // State machine for span assembly
    let mut current_category: Option<NerCategory> = None;
    let mut span_start_char: usize = 0;
    let mut span_end_char: usize = 0;
    let mut span_confidences: Vec<f32> = Vec::new();

    for (t, &(label_idx, conf)) in token_labels.iter().enumerate() {
        // Skip special tokens (word_id = None) and padding
        let is_special = t >= word_ids.len() || word_ids[t].is_none();
        if is_special {
            // Emit any active span
            if let Some(cat) = current_category.take() {
                emit_entity(
                    &mut entities,
                    original_text,
                    span_start_char,
                    span_end_char,
                    cat,
                    &span_confidences,
                );
                span_confidences.clear();
            }
            continue;
        }

        let label_str = BIO_LABELS.get(label_idx).copied().unwrap_or("O");
        let tag_category = event_labels::bio_label_to_category(label_str);
        let is_begin = event_labels::is_begin_tag(label_str);

        let (char_start, char_end) = if t < offsets.len() {
            offsets[t]
        } else {
            (0, 0)
        };

        match (is_begin, tag_category, current_category) {
            // B-X: start new span (emit previous if any)
            (true, Some(cat), prev) => {
                if let Some(prev_cat) = prev {
                    emit_entity(
                        &mut entities,
                        original_text,
                        span_start_char,
                        span_end_char,
                        prev_cat,
                        &span_confidences,
                    );
                    span_confidences.clear();
                }
                current_category = Some(cat);
                span_start_char = char_start;
                span_end_char = char_end;
                span_confidences.push(conf);
            }
            // I-X matching current category: extend span
            (false, Some(cat), Some(cur)) if cat == cur => {
                span_end_char = char_end;
                span_confidences.push(conf);
            }
            // I-X not matching, O, or no tag: emit and reset
            _ => {
                if let Some(cat) = current_category.take() {
                    emit_entity(
                        &mut entities,
                        original_text,
                        span_start_char,
                        span_end_char,
                        cat,
                        &span_confidences,
                    );
                    span_confidences.clear();
                }
            }
        }
    }

    // Emit trailing span
    if let Some(cat) = current_category {
        emit_entity(
            &mut entities,
            original_text,
            span_start_char,
            span_end_char,
            cat,
            &span_confidences,
        );
    }

    entities
}

fn emit_entity(
    entities: &mut Vec<ExtractedEntity>,
    original_text: &str,
    start: usize,
    end: usize,
    category: NerCategory,
    confidences: &[f32],
) {
    if start >= end || end > original_text.len() || confidences.is_empty() {
        return;
    }
    let text = original_text[start..end].to_string();
    let avg_conf = confidences.iter().sum::<f32>() / confidences.len() as f32;
    entities.push(ExtractedEntity {
        text,
        start,
        end,
        category,
        confidence: avg_conf,
    });
}

// ===========================================================================
// Tests — always run (no models needed)
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Output type construction
    // -----------------------------------------------------------------------

    #[test]
    fn classification_output_is_constructible() {
        let output = ClassificationOutput {
            event_kinds: vec![("ActionOccurrence".to_string(), 0.95)],
            entity_mentions: vec![ExtractedEntity {
                text: "the stone".to_string(),
                start: 14,
                end: 23,
                category: NerCategory::Object,
                confidence: 0.88,
            }],
        };
        assert_eq!(output.event_kinds.len(), 1);
        assert_eq!(output.entity_mentions.len(), 1);
        assert_eq!(output.entity_mentions[0].category, NerCategory::Object);
    }

    #[test]
    fn tokenized_input_is_constructible() {
        let input = TokenizedInput {
            token_ids: vec![101, 2023, 2003, 1037, 3231, 102],
            attention_mask: vec![1, 1, 1, 1, 1, 1],
            offsets: vec![(0, 0), (0, 4), (5, 7), (8, 9), (10, 14), (0, 0)],
            word_ids: vec![None, Some(0), Some(1), Some(2), Some(3), None],
        };
        assert_eq!(input.token_ids.len(), input.attention_mask.len());
        assert_eq!(input.token_ids.len(), input.offsets.len());
        assert_eq!(input.token_ids.len(), input.word_ids.len());
        assert!(input.word_ids[0].is_none());
        assert!(input.word_ids[5].is_none());
    }

    #[test]
    fn load_tokenizer_missing_file_returns_error() {
        let result = load_tokenizer(Path::new("/nonexistent/tokenizer.json"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, StorytellerError::Inference(_)),
            "expected Inference error, got: {err:?}"
        );
    }

    // -----------------------------------------------------------------------
    // sigmoid
    // -----------------------------------------------------------------------

    #[test]
    fn sigmoid_at_zero_is_half() {
        let val = sigmoid(0.0);
        assert!((val - 0.5).abs() < 1e-6, "sigmoid(0) = {val}, expected 0.5");
    }

    #[test]
    fn sigmoid_large_positive_near_one() {
        let val = sigmoid(10.0);
        assert!(val > 0.999, "sigmoid(10) = {val}, expected near 1.0");
    }

    #[test]
    fn sigmoid_large_negative_near_zero() {
        let val = sigmoid(-10.0);
        assert!(val < 0.001, "sigmoid(-10) = {val}, expected near 0.0");
    }

    // -----------------------------------------------------------------------
    // pad_or_truncate
    // -----------------------------------------------------------------------

    #[test]
    fn pad_short_input() {
        let input = TokenizedInput {
            token_ids: vec![101, 2023, 102],
            attention_mask: vec![1, 1, 1],
            offsets: vec![(0, 0), (0, 4), (0, 0)],
            word_ids: vec![None, Some(0), None],
        };
        let (ids, mask) = pad_or_truncate(&input, 8);
        assert_eq!(ids.len(), 8);
        assert_eq!(mask.len(), 8);
        assert_eq!(ids[0], 101);
        assert_eq!(ids[1], 2023);
        assert_eq!(ids[2], 102);
        assert_eq!(ids[3], 0); // padding
        assert_eq!(mask[3], 0); // padding
    }

    #[test]
    fn truncate_long_input() {
        let input = TokenizedInput {
            token_ids: vec![101, 1, 2, 3, 4, 5, 6, 7, 8, 102],
            attention_mask: vec![1; 10],
            offsets: vec![(0, 0); 10],
            word_ids: vec![
                None,
                Some(0),
                Some(1),
                Some(2),
                Some(3),
                Some(4),
                Some(5),
                Some(6),
                Some(7),
                None,
            ],
        };
        let (ids, mask) = pad_or_truncate(&input, 5);
        assert_eq!(ids.len(), 5);
        assert_eq!(ids[0], 101); // [CLS] preserved
        assert_eq!(ids[4], 4);
        assert_eq!(mask.iter().sum::<i64>(), 5); // all real tokens
    }

    #[test]
    fn exact_length_unchanged() {
        let input = TokenizedInput {
            token_ids: vec![101, 2023, 102],
            attention_mask: vec![1, 1, 1],
            offsets: vec![(0, 0), (0, 4), (0, 0)],
            word_ids: vec![None, Some(0), None],
        };
        let (ids, mask) = pad_or_truncate(&input, 3);
        assert_eq!(ids.len(), 3);
        assert_eq!(ids, vec![101, 2023, 102]);
        assert_eq!(mask, vec![1, 1, 1]);
    }

    // -----------------------------------------------------------------------
    // decode_event_logits
    // -----------------------------------------------------------------------

    #[test]
    fn decode_all_above_threshold() {
        // High logits → all labels returned
        let logits = vec![5.0; NUM_EVENT_KINDS];
        let result = decode_event_logits(&logits, 0.5);
        assert_eq!(result.len(), NUM_EVENT_KINDS);
        for (label, conf) in &result {
            assert!(conf > &0.99);
            assert!(EVENT_KIND_LABELS.contains(&label.as_str()));
        }
    }

    #[test]
    fn decode_none_above_threshold() {
        // Very negative logits → no labels
        let logits = vec![-10.0; NUM_EVENT_KINDS];
        let result = decode_event_logits(&logits, 0.5);
        assert!(result.is_empty());
    }

    #[test]
    fn decode_mixed_logits() {
        // Only index 1 (ActionOccurrence) above threshold
        let mut logits = vec![-10.0; NUM_EVENT_KINDS];
        logits[1] = 5.0; // ActionOccurrence
        let result = decode_event_logits(&logits, 0.5);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "ActionOccurrence");
        assert!(result[0].1 > 0.99);
    }

    // -----------------------------------------------------------------------
    // BIO span assembly
    // -----------------------------------------------------------------------

    fn make_bio_logits(seq_len: usize, assignments: &[(usize, &str)]) -> Vec<f32> {
        // Build logits where the assigned label has a high value and others are low.
        let mut logits = vec![-10.0_f32; seq_len * NUM_BIO_LABELS];
        for &(token_idx, label) in assignments {
            let label_idx = BIO_LABELS.iter().position(|&l| l == label).unwrap_or(0);
            let offset = token_idx * NUM_BIO_LABELS + label_idx;
            logits[offset] = 10.0;
        }
        // Tokens not in assignments default to O (index 0) via the max of the
        // low values — set O explicitly high for unassigned tokens.
        for t in 0..seq_len {
            let assigned = assignments.iter().any(|&(idx, _)| idx == t);
            if !assigned {
                logits[t * NUM_BIO_LABELS] = 10.0; // O label
            }
        }
        logits
    }

    #[test]
    fn assemble_single_token_entity() {
        // [CLS] "Sarah" [SEP] — token 1 is B-CHARACTER
        let text = "Sarah";
        let offsets = vec![(0, 0), (0, 5), (0, 0)];
        let word_ids: Vec<Option<u32>> = vec![None, Some(0), None];
        let logits = make_bio_logits(3, &[(1, "B-CHARACTER")]);

        let entities = assemble_entity_spans(&logits, 3, &offsets, &word_ids, text);
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].text, "Sarah");
        assert_eq!(entities[0].category, NerCategory::Character);
        assert_eq!(entities[0].start, 0);
        assert_eq!(entities[0].end, 5);
    }

    #[test]
    fn assemble_multi_token_entity() {
        // "the ancient stone" — 3 tokens: B-OBJECT, I-OBJECT, I-OBJECT
        let text = "the ancient stone";
        let offsets = vec![(0, 0), (0, 3), (4, 11), (12, 17), (0, 0)];
        let word_ids: Vec<Option<u32>> = vec![None, Some(0), Some(1), Some(2), None];
        let logits = make_bio_logits(5, &[(1, "B-OBJECT"), (2, "I-OBJECT"), (3, "I-OBJECT")]);

        let entities = assemble_entity_spans(&logits, 5, &offsets, &word_ids, text);
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].text, "the ancient stone");
        assert_eq!(entities[0].category, NerCategory::Object);
    }

    #[test]
    fn assemble_adjacent_different_entities() {
        // "Sarah stone" — token 1 is B-CHARACTER, token 2 is B-OBJECT
        let text = "Sarah stone";
        let offsets = vec![(0, 0), (0, 5), (6, 11), (0, 0)];
        let word_ids: Vec<Option<u32>> = vec![None, Some(0), Some(1), None];
        let logits = make_bio_logits(4, &[(1, "B-CHARACTER"), (2, "B-OBJECT")]);

        let entities = assemble_entity_spans(&logits, 4, &offsets, &word_ids, text);
        assert_eq!(entities.len(), 2);
        assert_eq!(entities[0].text, "Sarah");
        assert_eq!(entities[0].category, NerCategory::Character);
        assert_eq!(entities[1].text, "stone");
        assert_eq!(entities[1].category, NerCategory::Object);
    }

    #[test]
    fn assemble_no_entities() {
        let text = "nothing here";
        let offsets = vec![(0, 0), (0, 7), (8, 12), (0, 0)];
        let word_ids: Vec<Option<u32>> = vec![None, Some(0), Some(1), None];
        let logits = make_bio_logits(4, &[]); // all O

        let entities = assemble_entity_spans(&logits, 4, &offsets, &word_ids, text);
        assert!(entities.is_empty());
    }

    #[test]
    fn assemble_skips_special_tokens() {
        // Even if special tokens get non-O predictions, they should be skipped
        let text = "test";
        let offsets = vec![(0, 0), (0, 4), (0, 0)];
        let word_ids: Vec<Option<u32>> = vec![None, Some(0), None];
        let logits = make_bio_logits(
            3,
            &[(0, "B-CHARACTER"), (2, "B-OBJECT")], // special tokens tagged
        );

        let entities = assemble_entity_spans(&logits, 3, &offsets, &word_ids, text);
        assert!(
            entities.is_empty(),
            "special tokens should be skipped, got: {entities:?}"
        );
    }

    #[test]
    fn assemble_i_tag_without_b_is_ignored() {
        // I-CHARACTER without a preceding B-CHARACTER should not produce an entity
        let text = "Sarah";
        let offsets = vec![(0, 0), (0, 5), (0, 0)];
        let word_ids: Vec<Option<u32>> = vec![None, Some(0), None];
        let logits = make_bio_logits(3, &[(1, "I-CHARACTER")]);

        let entities = assemble_entity_spans(&logits, 3, &offsets, &word_ids, text);
        assert!(
            entities.is_empty(),
            "I-tag without B-tag should be ignored, got: {entities:?}"
        );
    }

    #[test]
    fn assemble_mismatched_i_tag_emits_previous() {
        // B-CHARACTER then I-OBJECT — should emit CHARACTER, drop the I-OBJECT
        let text = "Sarah stone";
        let offsets = vec![(0, 0), (0, 5), (6, 11), (0, 0)];
        let word_ids: Vec<Option<u32>> = vec![None, Some(0), Some(1), None];
        let logits = make_bio_logits(4, &[(1, "B-CHARACTER"), (2, "I-OBJECT")]);

        let entities = assemble_entity_spans(&logits, 4, &offsets, &word_ids, text);
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].text, "Sarah");
        assert_eq!(entities[0].category, NerCategory::Character);
    }
}

// ===========================================================================
// Integration tests — require real ONNX models on disk
// ===========================================================================

/// Tests that require real ONNX models and tokenizer on disk.
///
/// Run with: `cargo test --features test-ml-model`
///
/// Expects `STORYTELLER_MODEL_PATH` or `STORYTELLER_DATA_PATH` env var
/// pointing to a directory containing the event_classifier model directory.
#[cfg(all(test, feature = "test-ml-model"))]
mod integration_tests {
    use super::*;

    fn find_model_dir() -> Option<std::path::PathBuf> {
        if let Ok(model_path) = std::env::var("STORYTELLER_MODEL_PATH") {
            let p = std::path::PathBuf::from(&model_path).join("event_classifier");
            if p.join("event_classifier.onnx").exists()
                && p.join("ner_classifier.onnx").exists()
                && p.join("tokenizer.json").exists()
            {
                return Some(p);
            }
        }
        if let Ok(data_path) = std::env::var("STORYTELLER_DATA_PATH") {
            let p = std::path::PathBuf::from(&data_path).join("models/event_classifier");
            if p.join("event_classifier.onnx").exists()
                && p.join("ner_classifier.onnx").exists()
                && p.join("tokenizer.json").exists()
            {
                return Some(p);
            }
        }
        None
    }

    fn find_tokenizer_path() -> Option<std::path::PathBuf> {
        if let Ok(model_path) = std::env::var("STORYTELLER_MODEL_PATH") {
            let p = std::path::PathBuf::from(&model_path).join("event_classifier/tokenizer.json");
            if p.exists() {
                return Some(p);
            }
        }
        if let Ok(data_path) = std::env::var("STORYTELLER_DATA_PATH") {
            let p =
                std::path::PathBuf::from(&data_path).join("models/event_classifier/tokenizer.json");
            if p.exists() {
                return Some(p);
            }
        }
        None
    }

    #[test]
    fn load_real_tokenizer_and_encode() {
        let Some(tokenizer_path) = find_tokenizer_path() else {
            eprintln!(
                "skipping: no tokenizer.json found at \
                 $STORYTELLER_MODEL_PATH/event_classifier/ or \
                 $STORYTELLER_DATA_PATH/models/event_classifier/"
            );
            return;
        };

        let tokenizer = load_tokenizer(&tokenizer_path).expect("should load tokenizer from disk");

        let encoding = tokenizer
            .encode("I pick up the ancient stone from the riverbed", true)
            .expect("should tokenize text");

        let ids = encoding.get_ids();
        let offsets = encoding.get_offsets();
        let word_ids = encoding.get_word_ids();

        assert!(!ids.is_empty(), "token IDs should not be empty");
        assert_eq!(ids.len(), offsets.len());
        assert_eq!(ids.len(), word_ids.len());

        assert!(
            word_ids[0].is_none(),
            "first token should be special (None word_id)"
        );
        assert!(
            word_ids[ids.len() - 1].is_none(),
            "last token should be special (None word_id)"
        );
    }

    #[test]
    fn classify_text_produces_output() {
        let Some(model_dir) = find_model_dir() else {
            eprintln!(
                "skipping: models not found at \
                 $STORYTELLER_MODEL_PATH/event_classifier/ or \
                 $STORYTELLER_DATA_PATH/models/event_classifier/"
            );
            return;
        };

        let classifier = EventClassifier::load(&model_dir).expect("should load models");
        let output = classifier
            .classify_text("I pick up the ancient stone from the riverbed")
            .expect("classify_text should succeed");

        // Should have at least one event kind
        assert!(
            !output.event_kinds.is_empty(),
            "expected at least one event kind, got none"
        );

        // All confidences should be in (0, 1]
        for (label, conf) in &output.event_kinds {
            assert!(
                *conf > 0.0 && *conf <= 1.0,
                "confidence for {label} out of range: {conf}"
            );
            assert!(
                EVENT_KIND_LABELS.contains(&label.as_str()),
                "unknown event kind label: {label}"
            );
        }

        // Entity mentions should have valid spans
        let text = "I pick up the ancient stone from the riverbed";
        for entity in &output.entity_mentions {
            assert!(
                entity.end <= text.len(),
                "entity span exceeds text length: {}..{} in {}-char text",
                entity.start,
                entity.end,
                text.len()
            );
            assert!(
                entity.start < entity.end,
                "empty entity span: {}..{}",
                entity.start,
                entity.end
            );
            assert!(
                entity.confidence > 0.0 && entity.confidence <= 1.0,
                "entity confidence out of range: {}",
                entity.confidence
            );
        }

        eprintln!("classify_text output:");
        eprintln!("  event_kinds: {:?}", output.event_kinds);
        eprintln!("  entities: {:?}", output.entity_mentions);
    }

    #[test]
    fn classify_speech_text() {
        let Some(model_dir) = find_model_dir() else {
            eprintln!("skipping: models not found");
            return;
        };

        let classifier = EventClassifier::load(&model_dir).expect("should load models");
        let output = classifier
            .classify_text("I tell Sarah about the hidden path through the forest")
            .expect("classify_text should succeed");

        assert!(
            !output.event_kinds.is_empty(),
            "expected at least one event kind for speech text"
        );

        eprintln!("speech classify output:");
        eprintln!("  event_kinds: {:?}", output.event_kinds);
        eprintln!("  entities: {:?}", output.entity_mentions);
    }

    #[test]
    fn classify_emotional_text() {
        let Some(model_dir) = find_model_dir() else {
            eprintln!("skipping: models not found");
            return;
        };

        let classifier = EventClassifier::load(&model_dir).expect("should load models");
        let output = classifier
            .classify_text("Sarah trembles with fear as the wolf approaches")
            .expect("classify_text should succeed");

        // Model may or may not produce event kinds above threshold for
        // narrator-register prose — check structural validity, not specific labels.
        for (label, conf) in &output.event_kinds {
            assert!(
                EVENT_KIND_LABELS.contains(&label.as_str()),
                "unknown label: {label}"
            );
            assert!(
                *conf > 0.0 && *conf <= 1.0,
                "confidence out of range: {conf}"
            );
        }

        // Entity mentions should have valid spans
        let text = "Sarah trembles with fear as the wolf approaches";
        for entity in &output.entity_mentions {
            assert!(entity.end <= text.len());
            assert!(entity.start < entity.end);
        }

        eprintln!("emotional classify output:");
        eprintln!("  event_kinds: {:?}", output.event_kinds);
        eprintln!("  entities: {:?}", output.entity_mentions);
    }
}
