//! Generate event classification training data.
//!
//! Produces JSONL files with annotated text examples for training
//! event classification and entity extraction models.
//!
//! Usage:
//! ```sh
//! cargo run --bin generate-event-training-data -- \
//!     --output event_training_data.jsonl \
//!     --count 500 \
//!     --seed 42
//! ```

use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;

use clap::Parser;

use storyteller_ml::event_templates;

/// Generate event classification training data from combinatorial templates.
#[derive(Parser, Debug)]
#[command(name = "generate-event-training-data")]
struct Args {
    /// Output JSONL file path.
    #[arg(long, default_value = "event_training_data.jsonl")]
    output: PathBuf,

    /// Target number of examples per event kind.
    #[arg(long, default_value_t = 500)]
    count: usize,

    /// Random seed for reproducibility.
    #[arg(long)]
    seed: Option<u64>,
}

fn main() {
    let args = Args::parse();

    eprintln!(
        "Generating event classification training data: {} examples/kind, seed={:?}",
        args.count, args.seed
    );

    let (examples, manifest) = event_templates::generate(args.count, args.seed);

    eprintln!(
        "Generated {} valid examples ({} rejected)",
        manifest.total_valid, manifest.total_rejected
    );
    eprintln!("Event kind distribution:");
    for (kind, count) in &manifest.per_event_kind {
        eprintln!("  {kind}: {count}");
    }
    eprintln!("Register distribution:");
    for (register, count) in &manifest.per_register {
        eprintln!("  {register}: {count}");
    }

    // Write JSONL
    let file = File::create(&args.output).expect("failed to create output file");
    let mut writer = BufWriter::new(file);
    event_templates::export::write_jsonl(&examples, &mut writer).expect("failed to write JSONL");

    eprintln!(
        "Wrote {} examples to {}",
        examples.len(),
        args.output.display()
    );

    // Write manifest alongside the JSONL
    let manifest_path = args.output.with_extension("manifest.json");
    event_templates::export::write_manifest(&manifest, &manifest_path)
        .expect("failed to write manifest");

    eprintln!("Wrote manifest to {}", manifest_path.display());
}
