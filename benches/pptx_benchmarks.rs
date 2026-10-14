//! Criterion benchmarks for PPTX generation, lazy loading, and compression.
//!
//! Run: `cargo bench`
//! HTML report: `target/criterion/report/index.html`

use std::io::Cursor;

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use ppt_rs::generator::memory_profile::sample_slides;
use ppt_rs::generator::{
    LazySlideSource, SlideContent, create_pptx_lazy_to_writer, create_pptx_with_content_to_writer,
};
use ppt_rs::opc::compress::{CompressionOptions, compress_pptx_in_memory};

const SLIDE_COUNTS: [usize; 3] = [10, 50, 100];

struct IndexedSlideSource {
    slides: Vec<SlideContent>,
}

impl LazySlideSource for IndexedSlideSource {
    fn slide_count(&self) -> usize {
        self.slides.len()
    }

    fn generate_slide(&self, index: usize) -> Option<SlideContent> {
        self.slides.get(index).cloned()
    }
}

fn generate_fixture(slide_count: usize) -> Vec<u8> {
    let slides = sample_slides(slide_count);
    let cursor = Cursor::new(Vec::new());
    create_pptx_with_content_to_writer(cursor, "Fixture", &slides, None)
        .expect("fixture generation")
        .into_inner()
}

fn bench_eager_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("eager_generation");
    for slide_count in SLIDE_COUNTS {
        let slides = sample_slides(slide_count);
        group.throughput(Throughput::Elements(slide_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(slide_count),
            &slides,
            |b, slides| {
                b.iter(|| {
                    let cursor = Cursor::new(Vec::new());
                    create_pptx_with_content_to_writer(
                        cursor,
                        black_box("Benchmark"),
                        black_box(slides),
                        None,
                    )
                    .expect("eager generation")
                    .into_inner()
                });
            },
        );
    }
    group.finish();
}

fn bench_lazy_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("lazy_generation");
    for slide_count in SLIDE_COUNTS {
        let slides = sample_slides(slide_count);
        group.throughput(Throughput::Elements(slide_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(slide_count),
            &slides,
            |b, slides| {
                b.iter(|| {
                    let cursor = Cursor::new(Vec::new());
                    let source = Box::new(IndexedSlideSource {
                        slides: slides.clone(),
                    });
                    create_pptx_lazy_to_writer(cursor, black_box("Benchmark"), source, None)
                        .expect("lazy generation")
                        .into_inner()
                });
            },
        );
    }
    group.finish();
}

fn bench_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression");
    let options = CompressionOptions::web();

    for slide_count in SLIDE_COUNTS {
        let fixture = generate_fixture(slide_count);
        group.throughput(Throughput::Bytes(fixture.len() as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(slide_count),
            &fixture,
            |b, data| {
                b.iter(|| {
                    compress_pptx_in_memory(black_box(data), black_box(&options))
                        .expect("compression")
                        .0
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_eager_generation,
    bench_lazy_generation,
    bench_compression
);
criterion_main!(benches);
