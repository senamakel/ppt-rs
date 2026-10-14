//! End-to-end test for the placeholder API gap (python-pptx parity).
//!
//! Before v0.2.36 follow-up, the active slide XML emitter produced bare
//! `<p:nvPr/>` for title and body shapes — meaning PowerPoint saw them as
//! regular shapes, not as fills of the layout's intended title/body
//! placeholders. The layout's master title placeholder stayed empty even
//! when the user set `SlideContent::new("Title")`.
//!
//! These tests build a deck, unzip the slide XML, and assert that the
//! expected `<p:ph type="..."/>` elements are present.

use ppt_rs::{Presentation, SlideContent, SlideLayout, create_pptx_with_content};
use std::io::Read as _;

fn slide_xml(bytes: &[u8], slide_index: usize) -> String {
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).unwrap();
    let name = format!("ppt/slides/slide{}.xml", slide_index + 1);
    let mut s = String::new();
    archive
        .by_name(&name)
        .unwrap_or_else(|_| panic!("missing {name}"))
        .read_to_string(&mut s)
        .unwrap();
    s
}

#[test]
fn title_and_content_layout_emits_placeholders() {
    let slides = vec![SlideContent::new("Roadmap")
        .add_bullet("Q1")
        .add_bullet("Q2")];
    let bytes = create_pptx_with_content("Demo", slides).unwrap();
    let xml = slide_xml(&bytes, 0);
    assert!(
        xml.contains(r#"<p:ph type="title"/>"#),
        "title placeholder missing — got:\n{xml}"
    );
    assert!(
        xml.contains(r#"<p:ph type="body" idx="1"/>"#),
        "body placeholder missing — got:\n{xml}"
    );
}

#[test]
fn title_only_emits_only_title_placeholder() {
    let slides = vec![SlideContent::new("Section")
        .layout(SlideLayout::TitleOnly)];
    let bytes = create_pptx_with_content("Demo", slides).unwrap();
    let xml = slide_xml(&bytes, 0);
    assert!(xml.contains(r#"<p:ph type="title"/>"#));
    assert!(!xml.contains(r#"<p:ph type="body""#));
}

#[test]
fn centered_title_uses_ctr_title_placeholder() {
    let slides = vec![SlideContent::new("Big")
        .layout(SlideLayout::CenteredTitle)];
    let bytes = create_pptx_with_content("Demo", slides).unwrap();
    let xml = slide_xml(&bytes, 0);
    assert!(xml.contains(r#"<p:ph type="ctrTitle"/>"#));
}

#[test]
fn two_column_emits_two_body_placeholders() {
    let slides = vec![SlideContent::new("Compare")
        .add_bullet("A")
        .add_bullet("B")
        .add_bullet("C")
        .add_bullet("D")
        .layout(SlideLayout::TwoColumn)];
    let bytes = create_pptx_with_content("Demo", slides).unwrap();
    let xml = slide_xml(&bytes, 0);
    assert!(xml.contains(r#"<p:ph type="title"/>"#));
    assert!(xml.contains(r#"<p:ph type="body" idx="1"/>"#));
    assert!(xml.contains(r#"<p:ph type="body" idx="2"/>"#));
}

#[test]
fn presentation_builder_pipeline_also_emits_placeholders() {
    // The Presentation::build() path must use the same emitter as
    // create_pptx_with_content, so high-level users get placeholders too.
    let bytes = Presentation::new()
        .title("High-level demo")
        .add_slide(SlideContent::new("Hello").add_bullet("World"))
        .build()
        .unwrap();
    let xml = slide_xml(&bytes, 0);
    assert!(xml.contains(r#"<p:ph type="title"/>"#));
    assert!(xml.contains(r#"<p:ph type="body" idx="1"/>"#));
}

#[test]
fn package_validation_still_passes_with_placeholders() {
    // Adding `<p:ph type="..."/>` must not break the zero-repair validation gate.
    let bytes = create_pptx_with_content(
        "Validate me",
        vec![SlideContent::new("Title").add_bullet("Bullet")],
    )
    .unwrap();
    let report = ppt_rs::validate_package_bytes(&bytes);
    let errors: Vec<_> = report
        .issues
        .iter()
        .filter(|i| matches!(i.severity, ppt_rs::ValidationSeverity::Error))
        .collect();
    assert!(
        errors.is_empty(),
        "placeholder XML regressed package validation: {errors:?}"
    );
}