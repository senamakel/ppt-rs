//! Video & audio embedding: media parts, rels, slide XML, and content types
//! through the eager and lazy generation paths, plus compression interaction.

use std::io::{Cursor, Read};
use std::path::PathBuf;

use ppt_rs::core::validate_package_bytes;
use ppt_rs::generator::media_registry::MediaRegistry;
use ppt_rs::generator::{
    Audio, LazySlideSource, SlideContent, Video, VideoFormat, create_pptx_lazy_to_writer,
};
use ppt_rs::{Presentation, VideoOptions};
use zip::ZipArchive;

/// Canonical 1×1 PNG (same bytes as the generator's built-in placeholder).
const TINY_PNG: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 2, 0,
    0, 0, 144, 119, 83, 222, 0, 0, 0, 12, 73, 68, 65, 84, 120, 156, 99, 112, 112, 112, 0, 0, 1,
    132, 0, 193, 131, 95, 105, 148, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

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

/// Write fake media files (package structure is what we validate, not codecs).
fn media_fixture_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ppt_rs_media_{tag}_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("video.mp4"), b"fake-mp4-bytes").unwrap();
    std::fs::write(dir.join("audio.mp3"), b"fake-mp3-bytes").unwrap();
    std::fs::write(dir.join("poster.png"), TINY_PNG).unwrap();
    dir
}

fn media_deck(dir: &PathBuf) -> Presentation {
    let video = Video::new(
        dir.join("video.mp4").to_str().unwrap(),
        VideoFormat::Mp4,
        1_000_000,
        500_000,
        4_000_000,
        3_000_000,
    )
    .with_options(VideoOptions::auto_play())
    .with_poster(dir.join("poster.png").to_str().unwrap());

    let audio = Audio::new(
        dir.join("audio.mp3").to_str().unwrap(),
        ppt_rs::AudioFormat::Mp3,
        1_000_000,
        4_000_000,
        1_000_000,
        1_000_000,
    );

    Presentation::new()
        .title("Media Deck")
        .add_slide(
            SlideContent::new("With Media")
                .add_bullet("Talk over this")
                .add_video(video)
                .add_audio(audio),
        )
        .add_slide(SlideContent::new("No Media"))
}

#[test]
fn test_media_package_structure() {
    let dir = media_fixture_dir("eager");
    let bytes = media_deck(&dir).build().unwrap();

    // Media parts + poster image part exist
    assert!(has_part(&bytes, "ppt/media/media1.mp4"));
    assert!(has_part(&bytes, "ppt/media/media2.mp3"));

    // Slide XML carries the video/audio picture elements
    let slide_xml = read_part(&bytes, "ppt/slides/slide1.xml");
    assert!(slide_xml.contains("<a:videoFile"));
    assert!(slide_xml.contains("<a:audioFile"));
    assert!(slide_xml.contains("p14:media"));
    assert!(slide_xml.contains("ppaction://media"));

    // Two rels per media element: link (video|audio type) + embed (media type)
    let rels = read_part(&bytes, "ppt/slides/_rels/slide1.xml.rels");
    assert!(rels.contains("officeDocument/2006/relationships/video"));
    assert!(rels.contains("officeDocument/2006/relationships/audio"));
    assert!(rels.contains("officeDocument/2006/relationships/media"));
    assert!(rels.contains("Target=\"../media/media1.mp4\""));
    assert!(rels.contains("Target=\"../media/media2.mp3\""));
    let slide2_rels = read_part(&bytes, "ppt/slides/_rels/slide2.xml.rels");
    assert!(!slide2_rels.contains("relationships/media"));

    // Content type Defaults for media extensions
    let content_types = read_part(&bytes, "[Content_Types].xml");
    assert!(content_types.contains("Extension=\"mp4\" ContentType=\"video/mp4\""));
    assert!(content_types.contains("Extension=\"mp3\" ContentType=\"audio/mpeg\""));
    assert!(content_types.contains("Extension=\"png\""));

    assert!(
        validate_package_bytes(&bytes).is_valid(),
        "media deck failed validation"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_media_missing_source_errors() {
    let pres = Presentation::new().add_slide(SlideContent::new("Broken").add_video(Video::new(
        "/nonexistent/clip.mp4",
        VideoFormat::Mp4,
        0,
        0,
        1,
        1,
    )));
    let err = pres.build().unwrap_err();
    assert!(err.to_string().contains("media file"), "{err}");
}

#[test]
fn test_media_unsupported_extension_errors() {
    let dir = media_fixture_dir("badext");
    std::fs::write(dir.join("clip.txt"), b"not a video").unwrap();
    let pres = Presentation::new().add_slide(SlideContent::new("Bad").add_video(Video::new(
        dir.join("clip.txt").to_str().unwrap(),
        VideoFormat::Mp4,
        0,
        0,
        1,
        1,
    )));
    let err = pres.build().unwrap_err();
    assert!(
        err.to_string().contains("Unsupported media extension"),
        "{err}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

struct LazyMediaSlides {
    video_path: String,
}

impl LazySlideSource for LazyMediaSlides {
    fn slide_count(&self) -> usize {
        1
    }

    fn generate_slide(&self, index: usize) -> Option<SlideContent> {
        (index == 0).then(|| {
            SlideContent::new("Lazy Media").add_video(Video::new(
                &self.video_path,
                VideoFormat::Mp4,
                0,
                0,
                2_000_000,
                1_500_000,
            ))
        })
    }
}

#[test]
fn test_media_lazy_path_packages_media() {
    let dir = media_fixture_dir("lazy");
    let slides = Box::new(LazyMediaSlides {
        video_path: dir.join("video.mp4").to_string_lossy().into_owned(),
    });
    let cursor = Cursor::new(Vec::new());
    let bytes = create_pptx_lazy_to_writer(cursor, "Lazy Media", slides, None)
        .unwrap()
        .into_inner();

    assert!(has_part(&bytes, "ppt/media/media1.mp4"));
    let rels = read_part(&bytes, "ppt/slides/_rels/slide1.xml.rels");
    assert!(rels.contains("Target=\"../media/media1.mp4\""));
    assert!(
        validate_package_bytes(&bytes).is_valid(),
        "lazy media deck failed validation"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_compression_preserves_referenced_media() {
    let dir = media_fixture_dir("compress");
    let pres = media_deck(&dir);
    let options = ppt_rs::CompressionOptions::new(); // remove_unused_media defaults on
    let result = pres
        .compress("test_output/media_compressed.pptx", &options)
        .unwrap();

    let bytes = std::fs::read("test_output/media_compressed.pptx").unwrap();
    assert!(
        has_part(&bytes, "ppt/media/media1.mp4"),
        "video part must survive compression"
    );
    assert!(
        has_part(&bytes, "ppt/media/media2.mp3"),
        "audio part must survive compression"
    );
    assert!(validate_package_bytes(&bytes).is_valid());
    assert!(result.compressed_size > 0);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_file("test_output/media_compressed.pptx");
}

#[test]
fn test_media_registry_dedupes_across_slides() {
    let mut registry = MediaRegistry::default();
    let a = registry.media_number(b"same-bytes", "mp4");
    let b = registry.media_number(b"same-bytes", "mp4");
    assert_eq!(a, b);
    assert_eq!(registry.media_files().len(), 1);
}
