//! Example: batch-convert Markdown files to separate `.pptx` decks.
//!
//! Run with: `cargo run --example batch_generator --features cli`

use std::path::Path;

use ppt_rs::{BatchOptions, run_directory};

fn main() -> ppt_rs::Result<()> {
    let input_dir = Path::new("examples/batch_inputs");
    let output_dir = Path::new("examples/output/batch");

    println!("Batch converting files in {}...\n", input_dir.display());

    let results = run_directory(input_dir, &BatchOptions::new(output_dir))?;

    for result in &results {
        println!(
            "✓ {} → {} ({} slides, {} bytes)",
            result.input.display(),
            result.output.display(),
            result.slide_count,
            result.bytes
        );
    }

    println!("\n✓ Generated {} presentation(s)", results.len());
    Ok(())
}
