//! Comments round-trip: slide comments and comment authors packaged as
//! `ppt/comments/commentN.xml` + `ppt/commentAuthors.xml`.
//!
//! Verifies package structure (parts, `[Content_Types].xml`, slide rels,
//! presentation rels) and the zero-repair validation gate for decks with
//! comments, through both the eager and lazy generation paths.

use std::io::{Cursor, Read};

use ppt_rs::Presentation;
use ppt_rs::import::import_pptx;
use ppt_rs::core::validate_package_bytes;
use ppt_rs::generator::{
    CommentAuthorList, LazySlideSource, PresentationSettings, SlideContent,
    create_pptx_lazy_to_writer,
};
use zip::ZipArchive;

fn read_part(bytes: &[u8], path: &str) -> String {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).expect("valid zip");
    let mut s = String::new();
    archive
        .by_name(path)
        .unwrap_or_else(|e| panic!("part {path}: {e}"))
        .read_to_string(&mut s)
        .unwrap();
    s
}

fn has_part(bytes: &[u8], path: &str) -> bool {
    ZipArchive::new(Cursor::new(bytes))
        .unwrap()
        .by_name(path)
        .is_ok()
}

fn commented_deck() -> Presentation {
    let mut pres = Presentation::new().title("Comments");
    let alice = pres.add_comment_author("Alice Reviewer", "AR");
    let _bob = pres.add_comment_author("Bob Author", "BA");
    pres.add_slide(
        SlideContent::new("Slide 1")
            .add_bullet("Point")
            .add_comment(alice, "Fix wording", 100, 200)
            .add_comment(alice, "Add a source", 300, 400),
    )
    .add_slide(SlideContent::new("Slide 2").add_bullet("No comments here"))
}

#[test]
fn test_comments_package_structure() {
    let bytes = commented_deck().build().unwrap();

    // Comment part + authors part exist
    assert!(has_part(&bytes, "ppt/comments/comment1.xml"));
    assert!(has_part(&bytes, "ppt/commentAuthors.xml"));

    // Comment part content
    let comments = read_part(&bytes, "ppt/comments/comment1.xml");
    assert!(comments.contains("<p:cmLst"));
    assert!(comments.contains("Fix wording"));
    assert!(comments.contains("Add a source"));
    assert!(comments.contains("<p:pos x=\"100\" y=\"200\"/>"));

    // Authors part content
    let authors = read_part(&bytes, "ppt/commentAuthors.xml");
    assert!(authors.contains("<p:cmAuthorLst"));
    assert!(authors.contains("Alice Reviewer"));
    assert!(authors.contains("Bob Author"));

    // Content type overrides
    let content_types = read_part(&bytes, "[Content_Types].xml");
    assert!(content_types.contains("presentationml.comments+xml"));
    assert!(content_types.contains("/ppt/comments/comment1.xml"));
    assert!(content_types.contains("presentationml.commentAuthors+xml"));
    assert!(content_types.contains("/ppt/commentAuthors.xml"));

    // Slide 1 rels point at the comment part; slide 2 has none
    let slide1_rels = read_part(&bytes, "ppt/slides/_rels/slide1.xml.rels");
    assert!(slide1_rels.contains("officeDocument/2006/relationships/comments"));
    assert!(slide1_rels.contains("Target=\"../comments/comment1.xml\""));
    let slide2_rels = read_part(&bytes, "ppt/slides/_rels/slide2.xml.rels");
    assert!(!slide2_rels.contains("relationships/comments"));

    // Presentation rels reference the authors part
    let pres_rels = read_part(&bytes, "ppt/_rels/presentation.xml.rels");
    assert!(pres_rels.contains("officeDocument/2006/relationships/commentAuthors"));
    assert!(pres_rels.contains("Target=\"commentAuthors.xml\""));
}

#[test]
fn test_comments_deck_passes_package_validation() {
    let bytes = commented_deck().build().unwrap();
    let report = validate_package_bytes(&bytes);
    assert!(
        report.is_valid(),
        "comments deck failed validation: {:?}",
        report.issues
    );
}

#[test]
fn test_deck_without_comments_has_no_comment_parts() {
    let bytes = Presentation::new()
        .title("Plain")
        .add_slide(SlideContent::new("Slide 1").add_bullet("Point"))
        .build()
        .unwrap();

    assert!(!has_part(&bytes, "ppt/commentAuthors.xml"));
    let content_types = read_part(&bytes, "[Content_Types].xml");
    assert!(!content_types.contains("comment"));
    let pres_rels = read_part(&bytes, "ppt/_rels/presentation.xml.rels");
    assert!(!pres_rels.contains("commentAuthors"));
    assert!(
        validate_package_bytes(&bytes).is_valid(),
        "plain deck must stay valid"
    );
}

struct LazyCommentSlides {
    author_ids: CommentAuthorList,
}

impl LazyCommentSlides {
    fn first_author_id(&self) -> u32 {
        self.author_ids.authors().first().map(|a| a.id).unwrap_or(0)
    }
}

impl LazySlideSource for LazyCommentSlides {
    fn slide_count(&self) -> usize {
        2
    }

    fn generate_slide(&self, index: usize) -> Option<SlideContent> {
        match index {
            0 => Some(SlideContent::new("Lazy 1").add_bullet("Point").add_comment(
                self.first_author_id(),
                "lazy comment",
                42,
                84,
            )),
            1 => Some(SlideContent::new("Lazy 2")),
            _ => None,
        }
    }
}

#[test]
fn test_comments_lazy_path_packages_comment_parts() {
    let mut authors = CommentAuthorList::new();
    authors.get_or_add("Lazy Author", "LA");
    let slides = Box::new(LazyCommentSlides {
        author_ids: authors,
    });

    let settings = {
        let mut s = PresentationSettings::new();
        s.comment_authors.get_or_add("Lazy Author", "LA");
        Some(s)
    };

    let cursor = Cursor::new(Vec::new());
    let bytes = create_pptx_lazy_to_writer(cursor, "Lazy Comments", slides, settings)
        .unwrap()
        .into_inner();

    assert!(has_part(&bytes, "ppt/comments/comment1.xml"));
    assert!(has_part(&bytes, "ppt/commentAuthors.xml"));
    let content_types = read_part(&bytes, "[Content_Types].xml");
    assert!(content_types.contains("/ppt/commentAuthors.xml"));
    let comments = read_part(&bytes, "ppt/comments/comment1.xml");
    assert!(comments.contains("lazy comment"));
    let slide1_rels = read_part(&bytes, "ppt/slides/_rels/slide1.xml.rels");
    assert!(slide1_rels.contains("Target=\"../comments/comment1.xml\""));
    assert!(
        validate_package_bytes(&bytes).is_valid(),
        "lazy comments deck failed validation"
    );
}

#[test]
fn test_comments_import_round_trip() {
    let path = std::env::temp_dir().join(format!(
        "ppt_rs_comments_import_{}.pptx",
        std::process::id()
    ));
    commented_deck().save(&path).unwrap();

    let imported = import_pptx(path.to_str().unwrap()).unwrap();
    assert_eq!(imported.slides()[0].comments.len(), 2);
    assert_eq!(imported.slides()[0].comments.comments()[0].text, "Fix wording");
    assert_eq!(imported.slides()[0].comments.comments()[1].text, "Add a source");
    assert_eq!(imported.slides()[0].comments.comments()[0].x, 100);
    assert_eq!(imported.slides()[1].comments.len(), 0);

    let bytes = imported.build().unwrap();
    assert!(has_part(&bytes, "ppt/commentAuthors.xml"));
    assert!(has_part(&bytes, "ppt/comments/comment1.xml"));
    let comments = read_part(&bytes, "ppt/comments/comment1.xml");
    assert!(comments.contains("Fix wording"));
    assert!(comments.contains("Add a source"));
    assert!(validate_package_bytes(&bytes).is_valid());

    let _ = std::fs::remove_file(path);
}

#[test]
fn test_authors_without_slide_comments_still_package() {
    let mut pres = Presentation::new().title("Authors only");
    let _alice = pres.add_comment_author("Alice", "A");
    let bytes = pres
        .add_slide(SlideContent::new("Slide 1"))
        .build()
        .unwrap();

    assert!(has_part(&bytes, "ppt/commentAuthors.xml"));
    assert!(!has_part(&bytes, "ppt/comments/comment1.xml"));
    let content_types = read_part(&bytes, "[Content_Types].xml");
    assert!(content_types.contains("/ppt/commentAuthors.xml"));
    assert!(!content_types.contains("comments/comment1"));
    assert!(validate_package_bytes(&bytes).is_valid());
}
