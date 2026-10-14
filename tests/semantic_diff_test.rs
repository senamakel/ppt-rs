//! Integration tests for semantic PPTX diff.

use ppt_rs::Presentation;
use ppt_rs::compare_pptx_paths;
use ppt_rs::compare_presentations;
use ppt_rs::generator::SlideContent;

#[test]
fn test_semantic_diff_detects_bullet_change() {
    let a =
        Presentation::with_title("Deck").add_slide(SlideContent::new("Slide").add_bullet("alpha"));
    let b =
        Presentation::with_title("Deck").add_slide(SlideContent::new("Slide").add_bullet("beta"));

    let diff = compare_presentations(&a, &b);
    assert!(!diff.is_equal());
    assert!(diff.summary().contains("modified"));
}

#[test]
fn test_semantic_diff_file_paths() {
    let old_path = "test_semantic_diff_old.pptx";
    let new_path = "test_semantic_diff_new.pptx";

    Presentation::with_title("Old")
        .add_slide(SlideContent::new("Slide 1"))
        .save(old_path)
        .unwrap();
    Presentation::with_title("New")
        .add_slide(SlideContent::new("Slide 1"))
        .save(new_path)
        .unwrap();

    let diff = compare_pptx_paths(old_path, new_path).unwrap();
    assert!(!diff.is_equal());
    assert!(diff.title_changed.is_some());

    std::fs::remove_file(old_path).unwrap();
    std::fs::remove_file(new_path).unwrap();
}
