//! Corpus integrity tests: every "match" case must really pass through the
//! compiler seams, and the committed report must account for every case.
//!
//! Metadata schema validation lives in the harness (`scripts/compat_core.py`);
//! these tests only check the files the Rust-facing assertions depend on.

use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../")
}

fn case_directories() -> Vec<PathBuf> {
    fs::read_dir(root().join("tests/compatibility/corpus"))
        .expect("read corpus")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_dir())
        .collect()
}

fn manifest(directory: &Path) -> String {
    let name = directory.file_name().expect("case name").to_string_lossy();
    fs::read_to_string(directory.join("case.json"))
        .unwrap_or_else(|error| panic!("{name}: case.json: {error}"))
}

/// Read one string field from the small hand-written case manifests. Keeping
/// this reader local avoids pulling a JSON dependency into the workspace for a
/// handful of flat metadata files.
fn manifest_string(metadata: &str, field: &str) -> Option<String> {
    let key = format!("\"{field}\"");
    let offset = metadata.find(&key)? + key.len();
    let rest = &metadata[offset..];
    let colon = rest.find(':')?;
    let value = rest[colon + 1..].trim_start();
    let opening = value.strip_prefix('"')?;
    let end = opening.find('"')?;
    Some(opening[..end].to_owned())
}

fn case_id(metadata: &str) -> String {
    manifest_string(metadata, "id").expect("case id")
}

fn expectation(metadata: &str) -> String {
    let id = case_id(metadata);
    manifest_string(metadata, "expect").unwrap_or_else(|| panic!("{id}: no expectation"))
}

#[test]
fn every_case_is_well_formed() {
    let mut ids = Vec::new();
    for directory in case_directories() {
        let metadata = manifest(&directory);
        let id = case_id(&metadata);
        let expect = expectation(&metadata);
        for field in ["title", "category", "expect"] {
            assert!(
                manifest_string(&metadata, field).is_some_and(|value| !value.is_empty()),
                "{id}: missing {field}"
            );
        }
        if expect != "match" {
            assert!(
                manifest_string(&metadata, "reason").is_some_and(|value| !value.is_empty()),
                "{id}: missing reason"
            );
        }
        assert!(
            ["match", "differ", "skip"].contains(&expect.as_str()),
            "{id}: bad expectation"
        );
        assert!(
            directory.join("program.ts").is_file(),
            "{id}: no program.ts"
        );
        assert!(
            directory.join("program.ns").is_file(),
            "{id}: no program.ns"
        );
        ids.push(id);
    }
    let mut sorted = ids.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), ids.len(), "case ids must be unique");
    assert!(!ids.is_empty(), "corpus must not be empty");
}

#[test]
fn match_cases_reach_valid_ir() {
    for directory in case_directories() {
        let metadata = manifest(&directory);
        if expectation(&metadata) != "match" {
            continue;
        }
        let id = case_id(&metadata);
        let loaded = navascript_driver::modules::load(&directory.join("program.ns"))
            .unwrap_or_else(|error| panic!("{id}: module load failed: {error}"));
        let parsed = navascript_parser::parse(&loaded.source);
        assert!(
            !parsed.diagnostics.has_errors(),
            "{id}: parse: {}",
            parsed.diagnostics.render(&loaded.source)
        );
        let checked = navascript_typechecker::check(&parsed.program, &loaded.source);
        assert!(
            !checked.diagnostics.has_errors(),
            "{id}: check: {}",
            checked.diagnostics.render(&loaded.source)
        );
        let lowered = navascript_ir::lower(&parsed.program, &checked.typed, &loaded.source);
        assert!(
            !lowered.diagnostics.has_errors(),
            "{id}: lower: {}",
            lowered.diagnostics.render(&loaded.source)
        );
    }
}

#[test]
fn committed_report_covers_every_case_without_failures() {
    let report = fs::read_to_string(root().join("docs/COMPATIBILITY_REPORT.md"))
        .expect("committed compatibility report");
    let directories = case_directories();
    for directory in &directories {
        let id = case_id(&manifest(directory));
        assert!(
            report.contains(&format!("| `{id}` |")),
            "report misses {id}"
        );
    }
    assert_eq!(report.matches("| `").count(), directories.len());
    assert!(
        !report.contains("| fail |"),
        "report contains failing cases"
    );
}
