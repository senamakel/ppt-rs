//! End-to-end test for the TextFrame runtime editing API (python-pptx parity).
//!
//! python-pptx users can write `tf.add_paragraph()` / `tf.text = "…"` /
//! `p.level = 1` on imported decks. This test builds a deck, opens it via
//! `PresentationEditor`, mutates shapes through the high-level API, and
//! round-trips the result to verify the XML is well-formed and the text
//! actually lands where expected.

use ppt_rs::oxml::ParagraphEdit;
use ppt_rs::{SlideContent, create_pptx_with_content};
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn fixture_dir_for(test_name: &str) -> std::path::PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "ppt_rs_text_editor_test_{}_{}_{}",
        std::process::id(),
        n,
        test_name.replace("::", "_")
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_deck(name: &str, slides: Vec<SlideContent>) -> Vec<u8> {
    create_pptx_with_content(name, slides).unwrap()
}

fn slide_part(editor: &ppt_rs::oxml::PresentationEditor, slide_idx: usize) -> String {
    let path = format!("ppt/slides/slide{}.xml", slide_idx + 1);
    let bytes = editor.package().get_part(&path).unwrap();
    std::str::from_utf8(bytes).unwrap().to_owned()
}

fn open_in(bytes: &[u8], scratch: &std::path::Path) -> ppt_rs::oxml::PresentationEditor {
    let path = scratch.join("deck.pptx");
    std::fs::write(&path, bytes).unwrap();
    ppt_rs::oxml::PresentationEditor::open(path.to_str().unwrap()).unwrap()
}

#[test]
fn shape_has_text_frame_true_for_title_and_body() {
    let scratch = fixture_dir_for("has_text_frame_true");
    let bytes = write_deck(
        "Demo",
        vec![SlideContent::new("Title").add_bullet("Body 1")],
    );
    let editor = open_in(&bytes, &scratch);
    assert!(editor.shape_has_text_frame(0, 0).unwrap());
    assert!(editor.shape_has_text_frame(0, 1).unwrap());
}

#[test]
fn shape_has_text_frame_errors_for_out_of_range() {
    let scratch = fixture_dir_for("has_text_frame_out_of_range");
    let bytes = write_deck(
        "Demo",
        vec![SlideContent::new("Title").add_bullet("Body 1")],
    );
    let editor = open_in(&bytes, &scratch);
    let result = editor.shape_has_text_frame(0, 99);
    assert!(
        result.is_err(),
        "out-of-range shape index must error, got {result:?}"
    );
}

#[test]
fn set_shape_text_replaces_paragraphs() {
    let scratch = fixture_dir_for("set_shape_text");
    let bytes = write_deck(
        "Demo",
        vec![SlideContent::new("Title")
            .add_bullet("Old 1")
            .add_bullet("Old 2")],
    );
    let mut editor = open_in(&bytes, &scratch);
    editor.set_shape_text(0, 1, "Single replacement").unwrap();

    let xml = slide_part(&editor, 0);
    assert!(xml.contains("Single replacement"));
    assert!(!xml.contains("Old 1"));
    assert!(!xml.contains("Old 2"));
}

#[test]
fn set_title_text_updates_only_first_paragraph() {
    let scratch = fixture_dir_for("set_title");
    let bytes = write_deck(
        "Demo",
        vec![SlideContent::new("Title").add_bullet("Body")],
    );
    let mut editor = open_in(&bytes, &scratch);
    editor.set_title_text(0, "New Title").unwrap();

    let xml = slide_part(&editor, 0);
    assert!(xml.contains("New Title"));
    // Body should still be intact.
    assert!(xml.contains("Body"));
}

#[test]
fn add_paragraph_appends_at_end() {
    let scratch = fixture_dir_for("add_paragraph");
    let bytes = write_deck(
        "Demo",
        vec![SlideContent::new("Title").add_bullet("Original")],
    );
    let mut editor = open_in(&bytes, &scratch);
    editor.add_paragraph(0, 1, "Second").unwrap();
    editor.add_paragraph(0, 1, "Third").unwrap();

    let xml = slide_part(&editor, 0);
    let pos_orig = xml.find("Original").unwrap();
    let pos_second = xml.find("Second").unwrap();
    let pos_third = xml.find("Third").unwrap();
    assert!(pos_orig < pos_second);
    assert!(pos_second < pos_third);
}

#[test]
fn add_paragraph_with_format_emits_level_and_bold() {
    let scratch = fixture_dir_for("add_paragraph_with_format");
    let bytes = write_deck(
        "Demo",
        vec![SlideContent::new("Title").add_bullet("First")],
    );
    let mut editor = open_in(&bytes, &scratch);

    let para = ParagraphEdit::new("Bold nested")
        .with_level(2)
        .with_bold(true)
        .with_font_size(28);
    editor.add_paragraph_with_format(0, 1, para).unwrap();

    let xml = slide_part(&editor, 0);
    assert!(xml.contains("Bold nested"));
    assert!(xml.contains(r#"lvl="2""#));
    assert!(xml.contains(r#"b="1""#));
    assert!(xml.contains(r#"sz="2800""#));
}

#[test]
fn shape_paragraphs_reads_back_what_was_written() {
    let scratch = fixture_dir_for("shape_paragraphs_readback");
    let bytes = write_deck(
        "Demo",
        vec![SlideContent::new("Title")
            .add_bullet("Bullet A")
            .add_bullet("Bullet B")],
    );
    let mut editor = open_in(&bytes, &scratch);
    editor
        .add_paragraph_with_format(0, 1, ParagraphEdit::new("Bullet C"))
        .unwrap();

    let paragraphs = editor.shape_paragraphs(0, 1).unwrap();
    assert_eq!(paragraphs, vec!["Bullet A", "Bullet B", "Bullet C"]);
}

#[test]
fn edit_round_trips_through_save() {
    let scratch = fixture_dir_for("round_trip");
    let bytes = write_deck(
        "Demo",
        vec![SlideContent::new("Hello").add_bullet("World")],
    );
    let path = scratch.join("round_trip.pptx");
    std::fs::write(&path, &bytes).unwrap();
    let mut editor =
        ppt_rs::oxml::PresentationEditor::open(path.to_str().unwrap()).unwrap();
    editor.set_title_text(0, "Edited").unwrap();
    editor.save(path.to_str().unwrap()).unwrap();

    let reader = ppt_rs::oxml::PresentationReader::open(path.to_str().unwrap()).unwrap();
    let slide = reader.get_slide(0).unwrap();
    assert_eq!(slide.title.as_deref(), Some("Edited"));
}

#[test]
fn edit_does_not_break_package_validation() {
    let scratch = fixture_dir_for("package_validation");
    let bytes = write_deck(
        "Demo",
        vec![SlideContent::new("Title")
            .add_bullet("Bullet 1")
            .add_bullet("Bullet 2")],
    );
    let mut editor = open_in(&bytes, &scratch);
    editor.set_title_text(0, "Edited Title").unwrap();
    editor.add_paragraph(0, 1, "Bullet 3").unwrap();

    // Save the deck, then validate the full ZIP (not just slide XML).
    let path = scratch.join("validated.pptx");
    editor.save(path.to_str().unwrap()).unwrap();
    let saved = std::fs::read(&path).unwrap();
    let report = ppt_rs::validate_package_bytes(&saved);
    let errors: Vec<_> = report
        .issues
        .iter()
        .filter(|i| matches!(i.severity, ppt_rs::ValidationSeverity::Error))
        .collect();
    assert!(
        errors.is_empty(),
        "text-frame edit regressed package validation: {errors:?}"
    );
}

#[test]
fn has_text_frame_predicate_via_parsed_slide() {
    let scratch = fixture_dir_for("parsed_predicate");
    let bytes = write_deck(
        "Demo",
        vec![SlideContent::new("Title").add_bullet("Body")],
    );
    let path = scratch.join("parsed.pptx");
    std::fs::write(&path, &bytes).unwrap();
    let reader = ppt_rs::oxml::PresentationReader::open(path.to_str().unwrap()).unwrap();
    let slide = reader.get_slide(0).unwrap();
    assert!(slide.shapes[0].has_text_frame());
    assert!(slide.shapes[1].has_text_frame());
}

#[test]
fn edit_returns_error_for_shape_without_txbody() {
    let scratch = fixture_dir_for("shape_without_txbody");
    // A TitleOnly layout has shape 0 (title) with a txBody, but no body
    // shape. Asking for shape 1 (which doesn't exist) must return
    // NotFound("shape") — the same error users see when they reach past
    // the end of the shape list.
    let bytes = write_deck(
        "Demo",
        vec![SlideContent::new("Title")],
    );
    let mut editor = open_in(&bytes, &scratch);

    // Title (index 0) succeeds.
    editor.set_shape_text(0, 0, "Edited").unwrap();
    let xml = slide_part(&editor, 0);
    assert!(xml.contains("Edited"));

    // Out-of-range shape index errors with a clear NotFound.
    let err = editor.set_shape_text(0, 1, "Body").unwrap_err();
    let msg = format!("{err:?}");
    assert!(
        msg.contains("shape") || msg.contains("not"),
        "expected shape-related error, got {msg}"
    );
}

#[test]
fn edit_handles_unicode_text() {
    let scratch = fixture_dir_for("unicode_text");
    let bytes = write_deck(
        "Demo",
        vec![SlideContent::new("タイトル").add_bullet("• 国際化")],
    );
    let mut editor = open_in(&bytes, &scratch);
    editor
        .set_shape_text(0, 1, "🎉 Unicode round-trip")
        .unwrap();

    let xml = slide_part(&editor, 0);
    assert!(xml.contains("🎉 Unicode round-trip"));
    assert!(!xml.contains("国際化"));
}