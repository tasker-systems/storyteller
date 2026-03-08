//! Dataset validation CLI.
//!
//! Reads an existing JSONL training dataset, decodes examples, runs
//! coherence statistics, and reports summary.
//!
//! Usage:
//!   cargo run --bin validate-dataset -- --input training_data.jsonl

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use clap::Parser;

use storyteller_ml::feature_schema;
use storyteller_ml::matrix::export::TrainingExample;

#[derive(Parser)]
#[command(name = "validate-dataset")]
#[command(about = "Validate an existing JSONL training dataset")]
struct Cli {
    /// Path to the JSONL file to validate.
    #[arg(long)]
    input: PathBuf,

    /// Show per-example details.
    #[arg(long, default_value_t = false)]
    verbose: bool,
}

fn main() {
    let cli = Cli::parse();

    let file = File::open(&cli.input).unwrap_or_else(|e| {
        eprintln!("Error opening {}: {e}", cli.input.display());
        std::process::exit(1);
    });

    let reader = BufReader::new(file);
    let mut total = 0usize;
    let mut valid = 0usize;
    let mut invalid_feature_len = 0usize;
    let mut invalid_label_len = 0usize;
    let mut parse_errors = 0usize;
    let mut coherence_sum = 0.0f32;
    let mut coherence_min = f32::MAX;
    let mut coherence_max = f32::MIN;
    let mut hash_set = std::collections::HashSet::new();
    let mut duplicate_hashes = 0usize;

    for (line_num, line_result) in reader.lines().enumerate() {
        let line = match line_result {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Error reading line {}: {e}", line_num + 1);
                parse_errors += 1;
                continue;
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        let example: TrainingExample = match serde_json::from_str(&line) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Parse error on line {}: {e}", line_num + 1);
                parse_errors += 1;
                continue;
            }
        };

        total += 1;

        // Check feature vector length
        if example.features.len() != feature_schema::TOTAL_INPUT_FEATURES {
            if cli.verbose {
                eprintln!(
                    "Line {}: feature length {} (expected {})",
                    line_num + 1,
                    example.features.len(),
                    feature_schema::TOTAL_INPUT_FEATURES
                );
            }
            invalid_feature_len += 1;
        }

        // Check label vector length
        if example.labels.len() != feature_schema::TOTAL_OUTPUT_FEATURES {
            if cli.verbose {
                eprintln!(
                    "Line {}: label length {} (expected {})",
                    line_num + 1,
                    example.labels.len(),
                    feature_schema::TOTAL_OUTPUT_FEATURES
                );
            }
            invalid_label_len += 1;
        }

        // Check for duplicate content hashes
        if !hash_set.insert(example.content_hash.clone()) {
            duplicate_hashes += 1;
            if cli.verbose {
                eprintln!(
                    "Line {}: duplicate content hash {}",
                    line_num + 1,
                    example.content_hash
                );
            }
        }

        // Coherence stats
        coherence_sum += example.coherence_score;
        coherence_min = coherence_min.min(example.coherence_score);
        coherence_max = coherence_max.max(example.coherence_score);

        if cli.verbose {
            valid += 1;
            eprintln!(
                "  #{} id={} archA={} archB={} dyn={} coherence={:.3}",
                total,
                example.id,
                example.cell.archetype_a,
                example.cell.archetype_b,
                example.cell.dynamic,
                example.coherence_score,
            );
        } else {
            valid += 1;
        }
    }

    // Report
    println!("=== Dataset Validation Report ===");
    println!("File: {}", cli.input.display());
    println!("Total examples: {total}");
    println!("Parse errors: {parse_errors}");
    println!("Invalid feature length: {invalid_feature_len}");
    println!("Invalid label length: {invalid_label_len}");
    println!("Duplicate content hashes: {duplicate_hashes}");
    println!("Valid examples: {valid}");

    if total > 0 {
        let mean = coherence_sum / total as f32;
        println!("Coherence — min: {coherence_min:.3}, max: {coherence_max:.3}, mean: {mean:.3}");
    }

    println!(
        "Expected feature length: {}",
        feature_schema::TOTAL_INPUT_FEATURES
    );
    println!(
        "Expected label length: {}",
        feature_schema::TOTAL_OUTPUT_FEATURES
    );

    if parse_errors > 0 || invalid_feature_len > 0 || invalid_label_len > 0 {
        std::process::exit(1);
    }
}
