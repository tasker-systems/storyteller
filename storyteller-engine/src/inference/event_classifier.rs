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
use tokenizers::Tokenizer;

use storyteller_core::errors::StorytellerError;

/// Loads ONNX classification model(s) and tokenizer, runs inference to
/// classify text into event kinds and extract entity mentions.
///
/// # Architecture
///
/// Initially wraps separate ONNX models for event classification and entity
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
    #[expect(
        dead_code,
        reason = "used in C.3 classify_text() — not yet implemented"
    )]
    event_session: Mutex<Session>,
    /// Shared tokenizer loaded from `tokenizer.json`.
    tokenizer: Tokenizer,
    /// Dedicated thread pool for CPU-bound inference.
    #[expect(
        dead_code,
        reason = "used in C.3 classify_text() — not yet implemented"
    )]
    pool: rayon::ThreadPool,
}

impl EventClassifier {
    /// Load the ONNX model and tokenizer from a directory.
    ///
    /// Expects:
    /// - `model_dir/event_classifier.onnx` — event classification model
    /// - `model_dir/tokenizer.json` — HuggingFace tokenizer configuration
    ///
    /// # Errors
    ///
    /// Returns `StorytellerError::Inference` if model or tokenizer files
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

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .thread_name(|i| format!("event-classify-{i}"))
            .build()
            .map_err(|e| StorytellerError::Inference(e.to_string()))?;

        Ok(Self {
            event_session: Mutex::new(event_session),
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
    /// # Returns
    ///
    /// `(token_ids, attention_mask, offsets)` where offsets map each token
    /// back to `(start, end)` character positions in the original text.
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
}

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

#[cfg(test)]
mod tests {
    use super::*;

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
        // Special tokens have None word_ids
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
}

/// Tests that require a real tokenizer file on disk.
///
/// Run with: `cargo test --features test-ml-model`
///
/// Expects `STORYTELLER_MODEL_PATH` or `STORYTELLER_DATA_PATH` env var
/// pointing to a directory containing a `tokenizer.json` file (e.g.,
/// exported alongside a HuggingFace model via `AutoTokenizer.save_pretrained()`).
#[cfg(all(test, feature = "test-ml-model"))]
mod integration_tests {
    use super::*;

    fn find_tokenizer_path() -> Option<std::path::PathBuf> {
        // Check for event classifier tokenizer first
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

        // Basic sanity: non-empty, lengths match
        assert!(!ids.is_empty(), "token IDs should not be empty");
        assert_eq!(ids.len(), offsets.len());
        assert_eq!(ids.len(), word_ids.len());

        // First and last tokens should be special ([CLS] and [SEP])
        // with None word_ids
        assert!(
            word_ids[0].is_none(),
            "first token should be special (None word_id)"
        );
        assert!(
            word_ids[ids.len() - 1].is_none(),
            "last token should be special (None word_id)"
        );

        // Interior tokens should have Some word_ids
        let interior_word_ids: Vec<_> = word_ids[1..ids.len() - 1]
            .iter()
            .filter(|w| w.is_some())
            .collect();
        assert!(
            !interior_word_ids.is_empty(),
            "interior tokens should have word_ids"
        );

        eprintln!(
            "tokenizer test passed: {} tokens from {} chars",
            ids.len(),
            "I pick up the ancient stone from the riverbed".len()
        );
    }
}
