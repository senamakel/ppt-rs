//! Section-aware features: `SectionManager` → `presentation.xml` `<p14:sectionLst>`
//! (eager + lazy generation) and section grouping in Markdown/HTML export.

use std::io::{Cursor, Read};

use ppt_rs::Presentation;
use ppt_rs::core::validate_package_bytes;
use ppt_rs::export::html::{HtmlExportOptions, export_to_html_with_options};
use ppt_rs::export::md::{MarkdownOptions, export_to_markdown_with_options};
use ppt_rs::generator::{
    LazySlideSource, PresentationSettings, SlideContent, create_pptx_lazy_to_writer,
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

fn sectioned_deck() -> Presentation {
    Presentation::new()
        .title("Sections")
        .add_section("Intro", 0, 1)
        .unwrap()
        .add_section("Details", 1, 2)
        .unwrap()
        .add_slide(SlideContent::new("Welcome").add_bullet("Hello"))
        .add_slide(SlideContent::new("Detail A").add_bullet("A"))
        .add_slide(SlideContent::new("Detail B").add_bullet("B"))
}

#[test]
fn test_sections_in_presentation_xml() {
    let bytes = sectioned_deck().build().unwrap();
    let pres_xml = read_part(&bytes, "ppt/presentation.xml");
    assert!(pres_xml.contains("p14:sectionLst"));
    assert!(pres_xml.contains("name=\"Intro\""));
    assert!(pres_xml.contains("name=\"Details\""));
    assert!(pres_xml.contains(r#"<p14:sldId id="256"/>"#));
    // Ext lives inside the existing extLst, not as a second wrapper
    assert_eq!(pres_xml.matches("<p:extLst>").count(), 1);
    assert!(validate_package_bytes(&bytes).is_valid());
}

#[test]
fn test_deck_without_sections_has_no_section_lst() {
    let bytes = Presentation::new()
        .title("Plain")
        .add_slide(SlideContent::new("S1"))
        .build()
        .unwrap();
    let pres_xml = read_part(&bytes, "ppt/presentation.xml");
    assert!(!pres_xml.contains("sectionLst"));
    assert!(validate_package_bytes(&bytes).is_valid());
}

struct LazySectionSlides;

impl LazySlideSource for LazySectionSlides {
    fn slide_count(&self) -> usize {
        2
    }

    fn generate_slide(&self, index: usize) -> Option<SlideContent> {
        Some(SlideContent::new(&format!("Lazy {index}")))
    }
}

#[test]
fn test_sections_lazy_path() {
    let mut settings = PresentationSettings::new();
    settings.sections.add_section("Only", 0, 2).unwrap();

    let cursor = Cursor::new(Vec::new());
    let bytes = create_pptx_lazy_to_writer(
        cursor,
        "Lazy Sections",
        Box::new(LazySectionSlides),
        Some(settings),
    )
    .unwrap()
    .into_inner();

    let pres_xml = read_part(&bytes, "ppt/presentation.xml");
    assert!(pres_xml.contains("p14:sectionLst"));
    assert!(pres_xml.contains("name=\"Only\""));
    assert!(validate_package_bytes(&bytes).is_valid());
}

#[test]
fn test_add_section_overlap_errors() {
    let result = Presentation::new()
        .title("Bad")
        .add_section("A", 0, 2)
        .unwrap()
        .add_section("B", 1, 2);
    assert!(result.is_err());
}

#[test]
fn test_markdown_export_without_grouping_is_flat() {
    let pres = sectioned_deck();
    let md = export_to_markdown_with_options(&pres, &MarkdownOptions::new()).unwrap();
    assert!(md.contains("## Slide 1: Welcome"));
    assert!(!md.contains("### Slide"));
    assert!(!md.contains("## Intro"));
}

#[test]
fn test_markdown_export_with_section_grouping() {
    let pres = sectioned_deck();
    let md =
        export_to_markdown_with_options(&pres, &MarkdownOptions::new().with_section_grouping(true))
            .unwrap();
    assert!(md.contains("## Intro"));
    assert!(md.contains("## Details"));
    assert!(md.contains("### Slide 1: Welcome"));
    assert!(md.contains("### Slide 2: Detail A"));
    assert!(md.contains("### Slide 3: Detail B"));
}

#[test]
fn test_html_export_section_labels() {
    let pres = sectioned_deck();

    let plain = export_to_html_with_options(&pres, &HtmlExportOptions::new()).unwrap();
    assert!(!plain.contains("class=\"section-label\""));

    let grouped =
        export_to_html_with_options(&pres, &HtmlExportOptions::new().with_section_grouping(true))
            .unwrap();
    assert!(grouped.contains("class=\"section-label\""));
    assert!(grouped.contains(">Intro<"));
    assert!(grouped.contains(">Details<"));
    // Navigation indexing untouched: slides still carry data-slide 1..3
    assert!(grouped.contains("data-slide=\"3\""));
}

#[test]
fn test_sections_accessor_on_fresh_presentation() {
    let pres = Presentation::new();
    assert!(pres.sections().is_empty());
}
