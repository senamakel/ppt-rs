//! Integration tests for batch pipeline conversion.

use std::io::Write;
use std::path::Path;

use ppt_rs::{BatchJob, BatchOptions, InputFormat, convert_file, run_batch};

fn write_file(dir: &Path, name: &str, body: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::File::create(&path)
        .unwrap()
        .write_all(body.as_bytes())
        .unwrap();
    path
}

#[test]
fn batch_converts_html_file() {
    let root = std::env::temp_dir().join(format!("ppt_rs_pipe_html_{}", std::process::id()));
    let output = root.join("out");
    std::fs::create_dir_all(&root).unwrap();

    let input = write_file(
        &root,
        "page.html",
        "<h1>Title</h1><p>Body</p><h2>Section</h2><ul><li>One</li></ul>",
    );

    let job = BatchJob::new(&input);
    let results = run_batch(
        &[job],
        &BatchOptions::new(&output),
    )
    .unwrap();

    assert_eq!(results.len(), 1);
    assert!(results[0].output.exists());
    assert!(results[0].slide_count >= 1);
    assert!(results[0].bytes > 1000);

    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(feature = "cli")]
#[test]
fn batch_converts_markdown_file() {
    let root = std::env::temp_dir().join(format!("ppt_rs_pipe_md_{}", std::process::id()));
    let output = root.join("out");
    std::fs::create_dir_all(&root).unwrap();

    let input = write_file(
        &root,
        "deck.md",
        "# Slide A\n\n- Alpha\n\n# Slide B\n\n- Beta",
    );

    let result = convert_file(
        &BatchJob::new(&input),
        &BatchOptions::new(&output),
        InputFormat::Auto,
    )
    .unwrap();

    assert!(result.output.ends_with("deck.pptx"));
    assert_eq!(result.slide_count, 2);

    let _ = std::fs::remove_dir_all(&root);
}
