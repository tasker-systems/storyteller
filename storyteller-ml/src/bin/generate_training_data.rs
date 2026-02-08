//! Training data generation CLI.
//!
//! Generates labeled training examples from the combinatorial matrix:
//! archetype × dynamic × profile → heuristic predictions → JSONL export.
//!
//! Usage:
//!   cargo run --bin generate-training-data -- --count 50 --seed 42
//!   cargo run --bin generate-training-data -- --count 5000 --output data.jsonl

use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;

use clap::Parser;

use storyteller_ml::feature_schema::{self, PredictionInput};
use storyteller_ml::matrix::combinator::generate_matrix;
use storyteller_ml::matrix::descriptors::{self, DescriptorSet};
use storyteller_ml::matrix::export::{self, DatasetManifest, TrainingExample};
use storyteller_ml::matrix::labels::generate_labels;
use storyteller_ml::matrix::validation::validate_example;

#[derive(Parser)]
#[command(name = "generate-training-data")]
#[command(about = "Generate training data from the combinatorial matrix")]
struct Cli {
    /// Genre ID to generate for.
    #[arg(long, default_value = "low_fantasy_folklore")]
    genre: String,

    /// Number of matrix cells to sample.
    #[arg(long, default_value_t = 5000)]
    count: usize,

    /// Number of stochastic variations per cell.
    #[arg(long, default_value_t = 3)]
    variations: u32,

    /// Output JSONL file path.
    #[arg(long, default_value = "training_data.jsonl")]
    output: PathBuf,

    /// RNG seed for reproducibility.
    #[arg(long)]
    seed: Option<u64>,

    /// Minimum coherence score (examples below this are rejected).
    #[arg(long, default_value_t = 0.6)]
    min_coherence: f32,

    /// Path to descriptor data directory.
    /// Falls back to STORYTELLER_DATA_PATH env var or docs/storybook symlink.
    #[arg(long)]
    data_path: Option<PathBuf>,
}

fn main() {
    let cli = Cli::parse();

    // Load descriptors
    let data_path = cli
        .data_path
        .unwrap_or_else(|| match descriptors::resolve_data_path() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Error resolving data path: {e}");
                eprintln!("Set STORYTELLER_DATA_PATH or ensure docs/storybook symlink exists");
                std::process::exit(1);
            }
        });

    let descriptors = match DescriptorSet::load(&data_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error loading descriptors: {e}");
            std::process::exit(1);
        }
    };

    let genre = match descriptors.genre(&cli.genre) {
        Some(g) => g,
        None => {
            eprintln!("Unknown genre: {}", cli.genre);
            std::process::exit(1);
        }
    };

    // Set up RNG
    let mut rng: rand::rngs::StdRng = match cli.seed {
        Some(seed) => {
            use rand::SeedableRng;
            rand::rngs::StdRng::seed_from_u64(seed)
        }
        None => {
            use rand::SeedableRng;
            rand::rngs::StdRng::from_os_rng()
        }
    };

    eprintln!(
        "Generating matrix: genre={}, count={}, variations={}",
        cli.genre, cli.count, cli.variations
    );

    // Generate matrix
    let skeletons = generate_matrix(&descriptors, genre, cli.count, cli.variations, &mut rng);
    eprintln!("Generated {} skeletons", skeletons.len());

    // Label, encode, validate, collect
    let mut examples: Vec<TrainingExample> = Vec::new();
    let mut total_generated = 0usize;
    let mut total_rejected = 0usize;

    for skeleton in &skeletons {
        total_generated += 1;

        // Generate heuristic labels
        let prediction = generate_labels(skeleton, &mut rng);

        // Validate
        let validation = validate_example(skeleton, &prediction, cli.min_coherence);
        if !validation.passes {
            total_rejected += 1;
            continue;
        }

        // Encode features
        let input = PredictionInput {
            character: &skeleton.character_a,
            edges: std::slice::from_ref(&skeleton.edge_a_to_b),
            target_roles: &[skeleton.topology_b],
            scene: skeleton.scene,
            event: skeleton.event,
            history: &[],
        };
        let features = feature_schema::encode_features(&input);
        let labels = feature_schema::encode_labels(&prediction);

        let hash = export::content_hash(&features, &labels);

        examples.push(TrainingExample {
            id: export::example_id(),
            cell: skeleton.cell.clone(),
            variation: skeleton.variation_index,
            features,
            labels,
            coherence_score: validation.coherence_score,
            content_hash: hash,
        });
    }

    eprintln!(
        "Valid: {}, Rejected: {}, Total: {}",
        examples.len(),
        total_rejected,
        total_generated
    );

    // Write JSONL
    let file = File::create(&cli.output).unwrap_or_else(|e| {
        eprintln!("Error creating output file: {e}");
        std::process::exit(1);
    });
    let mut writer = BufWriter::new(file);
    export::write_jsonl(&examples, &mut writer).unwrap_or_else(|e| {
        eprintln!("Error writing JSONL: {e}");
        std::process::exit(1);
    });

    // Compute stats
    let mean_coherence = if examples.is_empty() {
        0.0
    } else {
        examples.iter().map(|e| e.coherence_score).sum::<f32>() / examples.len() as f32
    };

    // Write manifest
    let manifest_path = cli.output.with_extension("manifest.json");
    let manifest = DatasetManifest {
        version: "0.1.0".to_string(),
        total_generated,
        total_valid: examples.len(),
        total_rejected,
        mean_coherence,
        matrix_coverage: examples
            .iter()
            .map(|e| {
                format!(
                    "{}:{}:{}",
                    e.cell.archetype_a, e.cell.archetype_b, e.cell.dynamic
                )
            })
            .collect::<std::collections::HashSet<_>>()
            .len(),
        input_features: feature_schema::TOTAL_INPUT_FEATURES,
        output_features: feature_schema::TOTAL_OUTPUT_FEATURES,
        generated_at: chrono::Utc::now().to_rfc3339(),
        seed: cli.seed,
    };

    export::write_manifest(&manifest, &manifest_path).unwrap_or_else(|e| {
        eprintln!("Error writing manifest: {e}");
        std::process::exit(1);
    });

    eprintln!("Output: {}", cli.output.display());
    eprintln!("Manifest: {}", manifest_path.display());
    eprintln!(
        "Mean coherence: {:.3}, Coverage: {} unique cells",
        mean_coherence, manifest.matrix_coverage
    );
}
