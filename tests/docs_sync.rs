//! Doc-sync checks: make documentation drift a `cargo test` failure.
//!
//! When these tests fail, update the DOC (AGENTS.md structure / TODO.md test
//! counts) — not this file. AGENTS.md Step 9 (Update Documentation) is the
//! workflow that keeps them green.

use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(rel: &str) -> String {
    fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("cannot read {rel}: {e}"))
}

fn count_tests_in(dir: &Path) -> usize {
    let mut count = 0;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in fs::read_dir(&d).unwrap() {
            let p = entry.unwrap().path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs") {
                count += fs::read_to_string(&p)
                    .unwrap()
                    .lines()
                    .filter(|l| l.trim_start().starts_with("#[test]"))
                    .count();
            }
        }
    }
    count
}

/// Extract the integer immediately before `marker` (e.g. "732 inline" → 732).
fn leading_int(text: &str, marker: &str) -> usize {
    let idx = text
        .find(marker)
        .unwrap_or_else(|| panic!("doc missing marker '{marker}'"));
    let digits: String = text[..idx]
        .chars()
        .rev()
        .skip_while(|c| c.is_whitespace())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits
        .chars()
        .rev()
        .collect::<String>()
        .parse()
        .unwrap_or_else(|_| panic!("no integer before '{marker}'"))
}

#[test]
fn agents_md_structure_lists_every_module() {
    let lib = read("src/lib.rs");
    let agents = read("AGENTS.md");
    let missing: Vec<String> = lib
        .lines()
        .filter_map(|l| {
            let t = l.trim();
            t.strip_prefix("pub mod ")?
                .strip_suffix(';')
                .map(str::to_owned)
        })
        .filter(|m| !agents.contains(m))
        .collect();
    assert!(
        missing.is_empty(),
        "modules missing from the AGENTS.md project structure: {missing:?}"
    );
}

#[test]
fn architecture_md_mentions_every_module() {
    let lib = read("src/lib.rs");
    let arch = read("ARCHITECTURE.md");
    let missing: Vec<String> = lib
        .lines()
        .filter_map(|l| {
            let t = l.trim();
            t.strip_prefix("pub mod ")?
                .strip_suffix(';')
                .map(str::to_owned)
        })
        .filter(|m| !arch.contains(m) && !arch.contains(&format!("{m}::")))
        .collect();
    assert!(
        missing.is_empty(),
        "modules missing from ARCHITECTURE.md: {missing:?}"
    );
}

#[test]
fn todo_md_inline_test_count_matches_code() {
    let actual = count_tests_in(&root().join("src"));
    let claimed = leading_int(&read("TODO.md"), "inline /");
    assert_eq!(
        claimed, actual,
        "TODO.md inline-test count is stale (run `cargo test`, then update the header)"
    );
}

#[test]
fn todo_md_integration_test_count_matches_code() {
    let actual = count_tests_in(&root().join("tests"));
    let claimed = leading_int(&read("TODO.md"), "integration =");
    assert_eq!(
        claimed, actual,
        "TODO.md integration-test count is stale (run `cargo test`, then update the header)"
    );
}
