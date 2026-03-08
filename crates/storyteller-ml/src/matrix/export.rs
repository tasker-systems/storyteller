//! Training data export — JSONL format with content hashing and manifest.

use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::combinator::MatrixCell;

/// A single training example ready for export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingExample {
    /// Unique ID for this example.
    pub id: String,
    /// Matrix cell that generated this example.
    pub cell: MatrixCell,
    /// Variation index within the cell.
    pub variation: u32,
    /// Encoded input features (length = TOTAL_INPUT_FEATURES).
    pub features: Vec<f32>,
    /// Encoded output labels (length = TOTAL_OUTPUT_FEATURES).
    pub labels: Vec<f32>,
    /// Coherence score from validation.
    pub coherence_score: f32,
    /// SHA-256 hash of features + labels for deduplication.
    pub content_hash: String,
}

/// Dataset manifest — summary statistics written alongside the JSONL file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetManifest {
    /// Schema version.
    pub version: String,
    /// Total examples generated (before filtering).
    pub total_generated: usize,
    /// Valid examples exported.
    pub total_valid: usize,
    /// Rejected examples (below coherence threshold).
    pub total_rejected: usize,
    /// Mean coherence score of exported examples.
    pub mean_coherence: f32,
    /// Unique matrix cells covered.
    pub matrix_coverage: usize,
    /// Input feature vector length.
    pub input_features: usize,
    /// Output label vector length.
    pub output_features: usize,
    /// Generation timestamp.
    pub generated_at: String,
    /// RNG seed used (if any).
    pub seed: Option<u64>,
}

/// Compute SHA-256 hash of features + labels for deduplication.
pub fn content_hash(features: &[f32], labels: &[f32]) -> String {
    let mut hasher = Sha256::new();

    for &f in features {
        hasher.update(f.to_le_bytes());
    }
    for &l in labels {
        hasher.update(l.to_le_bytes());
    }

    format!("{:x}", hasher.finalize())
}

/// Generate a unique ID for a training example.
pub fn example_id() -> String {
    Uuid::now_v7().to_string()
}

/// Write training examples as JSONL to the given writer.
pub fn write_jsonl(examples: &[TrainingExample], writer: &mut impl Write) -> std::io::Result<()> {
    for example in examples {
        let line = serde_json::to_string(example).map_err(std::io::Error::other)?;
        writeln!(writer, "{}", line)?;
    }
    Ok(())
}

/// Write the dataset manifest as JSON.
pub fn write_manifest(manifest: &DatasetManifest, path: &Path) -> std::io::Result<()> {
    let content = serde_json::to_string_pretty(manifest).map_err(std::io::Error::other)?;
    std::fs::write(path, content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_hash_is_deterministic() {
        let features = vec![0.1, 0.2, 0.3];
        let labels = vec![0.4, 0.5];
        let hash1 = content_hash(&features, &labels);
        let hash2 = content_hash(&features, &labels);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn content_hash_differs_for_different_data() {
        let features_a = vec![0.1, 0.2, 0.3];
        let features_b = vec![0.1, 0.2, 0.4];
        let labels = vec![0.5];
        assert_ne!(
            content_hash(&features_a, &labels),
            content_hash(&features_b, &labels)
        );
    }

    #[test]
    fn example_id_is_unique() {
        let id1 = example_id();
        let id2 = example_id();
        assert_ne!(id1, id2);
    }

    #[test]
    fn write_jsonl_produces_valid_output() {
        let example = TrainingExample {
            id: "test".to_string(),
            cell: super::super::combinator::MatrixCell {
                archetype_a: "a".to_string(),
                archetype_b: "b".to_string(),
                dynamic: "d".to_string(),
                a_is_role_a: true,
                profile: "p".to_string(),
                genre: "g".to_string(),
            },
            variation: 0,
            features: vec![0.1, 0.2],
            labels: vec![0.3],
            coherence_score: 0.8,
            content_hash: "abc".to_string(),
        };

        let mut buf = Vec::new();
        write_jsonl(&[example], &mut buf).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("\"id\":\"test\""));
        assert!(output.ends_with('\n'));
    }
}
