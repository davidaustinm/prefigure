//! SVG-output parity tests (RUST_PORT_OUTLINE.md §12.3).
//!
//! Uses the shared, language-neutral corpus at the repository root: sources in
//! `tests/examples/<category>/` and reference snapshots (built by the Python
//! implementation) in `tests/snapshots/examples/<category>/`. The same assets
//! drive the Python suite (`tests/test_snapshots.py`); regenerate them with
//! `poetry run python tests/helpers/generate_snapshots.py`.
//!
//! `rust_output_matches_python` builds each snapshotted source with the Rust
//! pipeline and compares against the Python-produced SVG. Most figures are
//! compared as text (element for element, number for number). A few are
//! compared as pictures instead — see the three groups below. The text
//! comparator is exercised by the self-check tests.

mod raster_compare;
mod svg_compare;

use std::fs;
use std::path::{Path, PathBuf};

/// The shared test-asset root: `<repo>/tests`.
fn shared_tests_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests")
}

fn snapshots_dir() -> PathBuf {
    shared_tests_dir().join("snapshots/examples")
}

/// Every committed snapshot, sorted: (category, stem, snapshot path).
fn snapshot_files() -> Vec<(String, String, PathBuf)> {
    let mut files = Vec::new();
    if let Ok(categories) = fs::read_dir(snapshots_dir()) {
        for category in categories.flatten() {
            if !category.path().is_dir() {
                continue;
            }
            let cat = category.file_name().to_string_lossy().into_owned();
            if let Ok(entries) = fs::read_dir(category.path()) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_some_and(|e| e == "svg") {
                        let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
                        // `*-11.svg` are SVG 1.1 downgrade snapshots: not a
                        // separate source, and the parity here is SVG 2 output.
                        // The Rust svg11 downgrade has its own parity test
                        // (tests/svg11.rs); skip them here just as
                        // tests/test_snapshots.py does on the Python side.
                        if stem.ends_with("-11") {
                            continue;
                        }
                        files.push((cat.clone(), stem, path));
                    }
                }
            }
        }
    }
    files.sort();
    files
}

/// The comparator must accept every snapshot compared against itself.
#[test]
fn comparator_selfcheck_accepts_identical() {
    let files = snapshot_files();
    assert!(
        files.len() >= 160,
        "expected the shared snapshots at tests/snapshots to be present, found {}",
        files.len()
    );
    for (_, _, path) in files {
        let svg = fs::read_to_string(&path).unwrap();
        let diffs = svg_compare::compare(&svg, &svg, 1e-4);
        assert!(
            diffs.is_empty(),
            "{} does not match itself: {:?}",
            path.display(),
            diffs
        );
    }
}

/// ...and must reject clearly different documents.
#[test]
fn comparator_selfcheck_rejects_different() {
    let files = snapshot_files();
    let a = fs::read_to_string(&files[0].2).unwrap();
    let b = fs::read_to_string(&files.last().unwrap().2).unwrap();
    assert!(
        !svg_compare::compare(&a, &b, 1e-4).is_empty(),
        "comparator failed to distinguish {} from {}",
        files[0].2.display(),
        files.last().unwrap().2.display()
    );
}

/// ...and must tolerate sub-tolerance numeric jitter but flag real drift.
#[test]
fn comparator_selfcheck_numeric_tolerance() {
    let svg = r#"<svg width="310"><path d="M 5.0 305.0 L 47.9 5.0"/></svg>"#;
    let jittered = r#"<svg width="310"><path d="M 5.00003 305.0 L 47.9 5.0"/></svg>"#;
    let drifted = r#"<svg width="310"><path d="M 5.2 305.0 L 47.9 5.0"/></svg>"#;
    assert!(svg_compare::compare(svg, jittered, 1e-4).is_empty());
    assert!(!svg_compare::compare(svg, drifted, 1e-4).is_empty());
}

/// How each figure's Rust output is checked against Python's. Every figure
/// falls into exactly one of three groups:
///
/// - BYTE_IDENTICAL — the default for every figure not named in the two lists
///   below. The Rust and Python SVG files must hold the same elements and the
///   same numbers, down to a tiny rounding tolerance. This is the normal case
///   and the strongest check.
///
/// - RASTER_IDENTICAL — the figures in the list of that name. Their SVG files
///   are allowed to differ, but the pictures they draw must look the same.
///
/// - skipped — the figures in the SKIPPED list. Each must still build, but its
///   output is legitimately different and is not compared at all.
enum Check {
    ByteIdentical,
    RasterIdentical,
    Skipped,
}

/// Figures whose SVG text differs from Python's, but which draw the same
/// picture, so they are compared as pictures (tests/raster_compare.rs) rather
/// than as text.
///
/// Every figure here builds a `<shape>` out of other shapes with a boolean
/// operation (union, difference, intersection, exclusive-or, or a convex hull;
/// shape_clip clips against a union). The Rust port does this with the `geo`
/// library and Python does it with `shapely`. Both trace the same outline, but
/// they list its corner points starting from a different one and place them up
/// to about a seventh of a unit apart, so the numbers written into the SVG are
/// not the same even though the outline drawn from them is. At the figure's
/// real size that gap is well under one pixel, so the two pictures match. This
/// check only holds up because it always runs in the dev container, where the
/// same drawing library renders both files the same way.
const RASTER_IDENTICAL: &[&str] = &[
    "extracted_from_docs/shape_convex",
    "extracted_from_docs/shape_difference",
    "extracted_from_docs/shape_intersection",
    "extracted_from_docs/shape_union",
    "extracted_from_docs/shape_xor",
    "extracted_from_docs/shape_clip",
];

/// Figures the Rust port cannot (or should not) match at all. Each must still
/// build; its output is not compared.
/// - network-*: the automatic <network> layout is worked out differently than
///   networkx's, so the nodes land at different places on the page.
/// - judson-system: a long run of a differential-equation solver; the tiniest
///   rounding differences steer the solver's step sizes and slowly pull the
///   late part of the curve off Python's. The early part matches exactly.
/// - outline: Python's saved output has a bug here — the outlined <point>'s
///   pair of <use> marks is dropped (four SVG children where there should be
///   six). The Rust port draws it correctly, so it rightly produces more than
///   the saved file.
const SKIPPED: &[&str] = &[
    "extracted_from_docs/network-annotations",
    "extracted_from_docs/network-combination",
    "extracted_from_docs/network-intro",
    "extracted_from_docs/network-spanning-1",
    "extracted_from_docs/network-spanning-2",
    "extracted_from_docs/network-tree",
    "extracted_from_docs/network-verbose",
    "extracted_from_docs/judson-system",
    "extracted_from_docs/outline",
];

fn check_for(name: &str) -> Check {
    if RASTER_IDENTICAL.contains(&name) {
        Check::RasterIdentical
    } else if SKIPPED.contains(&name) {
        Check::Skipped
    } else {
        Check::ByteIdentical
    }
}

/// Every figure not in the SKIPPED list must match Python's output (as text or
/// as a picture); anything that regresses fails the build.
const MUST_PASS_ALL: bool = true;

/// The real parity test: build every snapshotted source with the Rust pipeline
/// and compare against the SVG that Python produced.
#[test]
fn rust_output_matches_python() {
    use prefig_core::core::label::LabelState;

    let examples_dir = shared_tests_dir().join("examples");
    let mut results: Vec<(String, Vec<String>)> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();

    let mut current_dir: Option<PathBuf> = None;
    for (category, stem, snapshot_path) in snapshot_files() {
        let name = format!("{category}/{stem}");
        let source_path = examples_dir.join(&category).join(format!("{stem}.xml"));
        let source = fs::read_to_string(&source_path)
            .unwrap_or_else(|e| panic!("snapshot {name} has no source: {e}"));
        let expected = fs::read_to_string(&snapshot_path).unwrap();

        // <read> and <image> resolve data/ relative to the working directory
        let dir = source_path.parent().unwrap().to_path_buf();
        if current_dir.as_ref() != Some(&dir) {
            let _ = std::env::set_current_dir(&dir);
            current_dir = Some(dir);
        }

        let built = prefig_core::engine::build_source(
            "svg",
            &source,
            &stem,
            "pretext",
            LabelState::local("svg"),
        );

        match check_for(&name) {
            Check::Skipped => {
                // must build, but the output is legitimately different
                match built {
                    Ok(_) => skipped.push(name),
                    Err(e) => results.push((name, vec![format!("build failed: {e}")])),
                }
            }
            Check::ByteIdentical => {
                let diffs = match built {
                    Ok((svg, _annotations)) => svg_compare::compare(&svg, &expected, 1e-2),
                    Err(e) => vec![format!("build failed: {e}")],
                };
                results.push((name, diffs));
            }
            Check::RasterIdentical => {
                let diffs = match built {
                    Ok((svg, _annotations)) => raster_compare::compare(&svg, &expected),
                    Err(e) => vec![format!("build failed: {e}")],
                };
                results.push((name, diffs));
            }
        }
    }

    let passing = results.iter().filter(|(_, d)| d.is_empty()).count();
    let failing: Vec<&(String, Vec<String>)> =
        results.iter().filter(|(_, d)| !d.is_empty()).collect();

    println!(
        "parity: {}/{} snapshots match Python ({} skipped, built ok)",
        passing,
        results.len(),
        skipped.len()
    );
    for (name, diffs) in &failing {
        println!("--- {name}: {} differences, first few:", diffs.len());
        for d in diffs.iter().take(4) {
            println!("    {d}");
        }
    }

    if MUST_PASS_ALL {
        let broken: Vec<&str> = failing.iter().map(|(n, _)| n.as_str()).collect();
        assert!(
            broken.is_empty(),
            "these examples no longer match Python: {broken:?}"
        );
    }
}
