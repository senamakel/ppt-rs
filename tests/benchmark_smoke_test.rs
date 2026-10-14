//! Smoke tests for benchmark code paths (generation + compression fixtures).

use ppt_rs::generator::memory_profile::sample_slides;
use ppt_rs::generator::create_pptx_with_content;
use ppt_rs::opc::compress::{CompressionOptions, compress_pptx_in_memory};

#[test]
fn benchmark_fixture_compresses_generated_deck() {
    let slides = sample_slides(20);
    let bytes = create_pptx_with_content("Benchmark Smoke", slides).expect("generation");

    let (compressed, result) =
        compress_pptx_in_memory(&bytes, &CompressionOptions::web()).expect("compression");

    assert!(!compressed.is_empty());
    assert_eq!(result.original_size, bytes.len());
    assert!(result.compressed_size > 0);
}
