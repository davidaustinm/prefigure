//! SVG-output parity tests (RUST_PORT_OUTLINE.md §12.3).
//!
//! Uses the shared, language-neutral corpus at the repository root: sources in
//! `tests/examples/<category>/` and reference snapshots (built by the Python
//! implementation) in `tests/snapshots/examples/<category>/`. The same assets
//! drive the Python suite (`tests/test_snapshots.py`); regenerate them with
//! `poetry run python tests/helpers/generate_snapshots.py`.
//!
//! `rust_output_matches_python` builds each snapshotted source with the Rust
//! pipeline and compares against the Python-produced SVG with numeric
//! tolerance. The comparator itself is exercised by the self-check tests.

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

/// Snapshots the Rust port cannot (or should not) match, each still required
/// to *build* — only the comparison is skipped:
/// - network-*: automatic <network> layouts are deterministic but use
///   different coordinates than networkx's PRNG (rust/PORTING.md).
/// - shape_*: boolean <shape> ops via the geo crate are geometrically equal
///   but not vertex-identical to shapely (rust/PORTING.md).
/// - judson-system: a 100-time-unit Lotka-Volterra integration; last-ulp fp
///   differences steer RK45 step selection, drifting the late trajectory
///   slightly beyond tolerance. Early points match exactly.
/// - outline, shape_clip: the reference snapshots capture Python BUGS — the
///   outlined <point>'s <use> pair is dropped, and everything after the first
///   <shape> inside <clip shape=...> is missing. The Rust port renders these
///   correctly, so it legitimately produces MORE than the snapshot.
const KNOWN_NON_PARITY: &[&str] = &[
    "extracted_from_docs/network-annotations",
    "extracted_from_docs/network-combination",
    "extracted_from_docs/network-intro",
    "extracted_from_docs/network-spanning-1",
    "extracted_from_docs/network-spanning-2",
    "extracted_from_docs/network-tree",
    "extracted_from_docs/network-verbose",
    "extracted_from_docs/shape_convex",
    "extracted_from_docs/shape_difference",
    "extracted_from_docs/shape_intersection",
    "extracted_from_docs/shape_union",
    "extracted_from_docs/shape_xor",
    "extracted_from_docs/judson-system",
    "extracted_from_docs/outline",
    "extracted_from_docs/shape_clip",
];

/// Everything not in KNOWN_NON_PARITY must match Python's output; anything that
/// regresses fails the build.
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

        if KNOWN_NON_PARITY.contains(&name.as_str()) {
            // must build, but the output is legitimately different
            if let Err(e) = built {
                results.push((name, vec![format!("build failed: {e}")]));
            } else {
                skipped.push(name);
            }
            continue;
        }

        let diffs = match built {
            Ok((svg, _annotations)) => svg_compare::compare(&svg, &expected, 1e-2),
            Err(e) => vec![format!("build failed: {e}")],
        };
        results.push((name, diffs));
    }

    let passing = results.iter().filter(|(_, d)| d.is_empty()).count();
    let failing: Vec<&(String, Vec<String>)> =
        results.iter().filter(|(_, d)| !d.is_empty()).collect();

    println!(
        "parity: {}/{} snapshots match Python ({} known-non-parity built ok)",
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
