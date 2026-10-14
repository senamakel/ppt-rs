//! End-to-end tests for `extract_slide_texts_from_bytes` (the high-level
//! "give me the text" helper that python-pptx users reach for in their
//! quickstart).

use ppt_rs::extract_slide_texts_from_bytes;
use ppt_rs::oxml::PresentationReader;
use ppt_rs::{Presentation, SlideContent};

fn build_deck() -> Vec<u8> {
    let slides = [
        SlideContent::new("Welcome")
            .add_bullet("First point")
            .add_bullet("Second point"),
        SlideContent::new("Roadmap").add_bullet("Q1 ships"),
        SlideContent::new("Thanks"),
    ];
    Presentation::new()
        .title("Extract Text Demo")
        .add_slide(slides[0].clone())
        .add_slide(slides[1].clone())
        .add_slide(slides[2].clone())
        .build()
        .expect("build deck")
}

#[test]
fn returns_one_inner_vec_per_slide() {
    let bytes = build_deck();
    let texts = extract_slide_texts_from_bytes(&bytes).unwrap();
    assert_eq!(texts.len(), 3);
}

#[test]
fn preserves_title_and_bullets_per_slide() {
    let bytes = build_deck();
    let texts = extract_slide_texts_from_bytes(&bytes).unwrap();
    assert!(texts[0].iter().any(|t| t.contains("Welcome")));
    assert!(texts[0].iter().any(|t| t.contains("First point")));
    assert!(texts[0].iter().any(|t| t.contains("Second point")));
    assert!(texts[1].iter().any(|t| t.contains("Roadmap")));
    assert!(texts[2].iter().any(|t| t.contains("Thanks")));
}

#[test]
fn does_not_leak_xml_to_caller() {
    let bytes = build_deck();
    let texts = extract_slide_texts_from_bytes(&bytes).unwrap();
    for slide_text in &texts {
        for t in slide_text {
            assert!(!t.contains("<a:"), "raw OOXML leaked: {t}");
            assert!(!t.contains("<p:"), "raw OOXML leaked: {t}");
            assert!(!t.contains("xmlns"), "namespace leaked: {t}");
        }
    }
}

#[test]
fn reader_api_parity() {
    // The reader's `extract_slide_texts` and the free function must return
    // the same content for the same deck — both go through the same code path.
    let bytes = build_deck();
    let from_bytes = extract_slide_texts_from_bytes(&bytes).unwrap();
    let tmp = std::env::temp_dir().join("ppt_rs_extract_text_reader_parity.pptx");
    std::fs::write(&tmp, &bytes).unwrap();
    let reader = PresentationReader::open(tmp.to_str().unwrap()).unwrap();
    let from_reader = reader.extract_slide_texts().unwrap();
    std::fs::remove_file(&tmp).ok();
    assert_eq!(from_bytes, from_reader);
}

#[test]
fn invalid_bytes_produce_error_not_panic() {
    let bad = b"\x00\x01\x02 not a real zip";
    let result = extract_slide_texts_from_bytes(bad);
    assert!(result.is_err());
}