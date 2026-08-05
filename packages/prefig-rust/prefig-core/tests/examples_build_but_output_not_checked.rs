//! Every shared example diagram must build without crashing. This checks only
//! that the build runs to completion, NOT that the output is correct -- output
//! correctness is checked separately (tests/expected_svgs.rs compares the SVG
//! against Python's, tests/annotations.rs the annotation XML).
//!
//! Walks the language-neutral corpus at the repository root
//! (`tests/examples/**/*.xml` -- the same sources the Python suite and the
//! parity test use). Each is run through the SVG and tactile build pipelines
//! inside `catch_unwind`; the test fails listing every figure that crashes. A
//! graceful `Err` is acceptable -- a few sources are meant to be embedded in a
//! PreTeXt document and get their dimensions from that wrapper, so standalone
//! they legitimately report an error rather than crash. We only guard against
//! crashes (index-out-of-bounds, `unwrap` on `None`, etc.).
//!
//! Labels are rendered with fixed stub services so the test needs neither Node
//! (MathJax) nor cairo, yet still exercises the label-layout code paths.

mod common;

use common::{collect_xml, stub_labels};
use prefig_core::engine::build_from_string;
use std::panic::{self, AssertUnwindSafe};
use std::path::PathBuf;

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&str>().copied())
        .unwrap_or("<non-string panic>")
        .to_string()
}

#[test]
fn examples_build_but_output_not_checked() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/examples");
    let mut figures = Vec::new();
    collect_xml(&root, &mut figures);
    figures.sort();
    assert!(
        figures.len() >= 160,
        "expected the shared examples under {}, found only {}",
        root.display(),
        figures.len()
    );

    // Silence the default per-panic stderr dump; catch_unwind still reports the
    // panic to us. Restored afterwards.
    let prev_hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));

    let mut panicked: Vec<String> = Vec::new();
    let mut graceful_errors = 0usize;
    let mut built = 0usize;

    for path in &figures {
        let source = std::fs::read_to_string(path).expect("read figure");
        let name = path
            .strip_prefix(&root)
            .unwrap_or(path)
            .display()
            .to_string();

        for format in ["svg", "tactile"] {
            let result = panic::catch_unwind(AssertUnwindSafe(|| {
                build_from_string(format, &source, "pf_cli", stub_labels())
            }));
            match result {
                Err(payload) => {
                    panicked.push(format!("{name} [{format}]: {}", panic_message(&*payload)))
                }
                Ok(Ok(_)) => built += 1,
                Ok(Err(_)) => graceful_errors += 1,
            }
        }
    }

    panic::set_hook(prev_hook);

    eprintln!(
        "examples: {} files x 2 formats => {built} built, {graceful_errors} graceful errors, {} panics",
        figures.len(),
        panicked.len(),
    );

    assert!(
        panicked.is_empty(),
        "{} example build(s) panicked:\n{}",
        panicked.len(),
        panicked.join("\n"),
    );
}
