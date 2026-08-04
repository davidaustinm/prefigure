//! Annotation-XML parity (the Rust analogue of the Python suite's
//! `test_annotations_match_snapshot`).
//!
//! Every committed `snapshots/examples/<category>/<stem>.xml` is the diagcess
//! annotation tree the Python pipeline produced for `<stem>.xml`. We build the
//! same source with the Rust pipeline and compare the annotation XML it returns
//! against that snapshot. Annotation content is derived from the source (element
//! structure and attribute speech), not from rendered labels, so fixed stub
//! label services suffice -- no Node/MathJax or cairo needed.

mod common;
mod svg_compare;

use common::stub_labels;
use std::fs;
use std::path::{Path, PathBuf};

fn shared_tests_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests")
}

/// (category, stem, snapshot path) for each committed annotation snapshot.
fn annotation_snapshots() -> Vec<(String, String, PathBuf)> {
    let dir = shared_tests_dir().join("snapshots/examples");
    let mut out = Vec::new();
    if let Ok(cats) = fs::read_dir(&dir) {
        for cat in cats.flatten() {
            if !cat.path().is_dir() {
                continue;
            }
            let category = cat.file_name().to_string_lossy().into_owned();
            if let Ok(entries) = fs::read_dir(cat.path()) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_none_or(|e| e != "xml") {
                        continue;
                    }
                    let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
                    out.push((category.clone(), stem, path));
                }
            }
        }
    }
    out.sort();
    out
}

#[test]
fn annotations_match_python() {
    let examples_dir = shared_tests_dir().join("examples");
    let cases = annotation_snapshots();
    assert!(
        cases.len() >= 15,
        "expected the committed annotation snapshots to be present, found {}",
        cases.len()
    );

    let mut current_dir: Option<PathBuf> = None;
    let mut failures: Vec<(String, Vec<String>)> = Vec::new();

    for (category, stem, snapshot) in &cases {
        let name = format!("{category}/{stem}");
        let source_path = examples_dir.join(category).join(format!("{stem}.xml"));
        let Ok(source) = fs::read_to_string(&source_path) else {
            failures.push((name, vec!["snapshot has no source".to_string()]));
            continue;
        };

        // <read>/<image> resolve data files relative to the source directory
        let dir = source_path.parent().unwrap().to_path_buf();
        if current_dir.as_ref() != Some(&dir) {
            let _ = std::env::set_current_dir(&dir);
            current_dir = Some(dir);
        }

        let built =
            prefig_core::engine::build_source("svg", &source, stem, "pretext", stub_labels());
        let expected = fs::read_to_string(snapshot).unwrap();
        let diffs = match built {
            Ok((_svg, Some(annotations))) => svg_compare::compare(&annotations, &expected, 1e-2),
            Ok((_svg, None)) => {
                vec!["built no annotations, but a snapshot exists".to_string()]
            }
            Err(e) => vec![format!("build failed: {e}")],
        };
        if !diffs.is_empty() {
            failures.push((name, diffs));
        }
    }

    println!(
        "annotations: {}/{} match Python",
        cases.len() - failures.len(),
        cases.len()
    );
    for (name, diffs) in &failures {
        println!("--- {name}: {} differences, first few:", diffs.len());
        for d in diffs.iter().take(5) {
            println!("    {d}");
        }
    }

    let broken: Vec<&str> = failures.iter().map(|(n, _)| n.as_str()).collect();
    assert!(
        broken.is_empty(),
        "annotation XML no longer matches Python: {broken:?}"
    );
}
