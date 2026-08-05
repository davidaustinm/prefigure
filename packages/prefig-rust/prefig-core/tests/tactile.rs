//! Tactile output is structurally distinct from the ordinary SVG build.
//!
//! Uses fixed stub label services (no Node/cairo/liblouis), so it runs anywhere
//! the rest of the suite does. It checks the two invariants that define tactile
//! layout and require no braille backend:
//!   - every tactile diagram is laid out on the fixed 828x792 emboss page,
//!     regardless of the source's own dimensions (which set the SVG size); and
//!   - tactile output therefore differs from the SVG build of the same source.
//!
//! Braille *content* (labels translated to Unicode Braille) is deliberately not
//! asserted here: it needs a real liblouis backend (the `braille-liblouis`
//! feature), and requiring that at link time would break the suite wherever
//! liblouis is absent -- the same reason the corpus has no tactile snapshot.

mod common;

use common::{collect_xml, stub_labels};
use prefig_core::engine::build_from_string;
use std::path::{Path, PathBuf};

fn examples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/examples")
}

/// (width, height) of the root `<svg>` element, from its opening tag.
fn root_dimensions(svg: &str) -> Option<(String, String)> {
    let open = &svg[..svg.find('>')?];
    let attr = |name: &str| -> Option<String> {
        let key = format!("{name}=\"");
        let start = open.find(&key)? + key.len();
        let rest = &open[start..];
        Some(rest[..rest.find('"')?].to_string())
    };
    Some((attr("width")?, attr("height")?))
}

#[test]
fn tactile_output_is_distinct_and_emboss_paged() {
    let dir = examples_dir();
    let mut figures = Vec::new();
    collect_xml(&dir, &mut figures);
    figures.sort();
    assert!(
        figures.len() >= 160,
        "expected the shared examples, found {}",
        figures.len()
    );

    let mut checked = 0usize;
    let mut wrong_page: Vec<String> = Vec::new();
    let mut identical: Vec<String> = Vec::new();

    for path in &figures {
        let source = std::fs::read_to_string(path).expect("read figure");
        let name = path
            .strip_prefix(&dir)
            .unwrap_or(path)
            .display()
            .to_string();

        // <read>/<image> resolve data files relative to the source directory
        let _ = std::env::set_current_dir(path.parent().unwrap());

        let svg = build_from_string("svg", &source, "pf_cli", stub_labels());
        let tactile = build_from_string("tactile", &source, "pf_cli", stub_labels());
        let (Ok((svg, _)), Ok((tactile, _))) = (svg, tactile) else {
            continue; // a graceful build error (e.g. a fragment) -- not our concern
        };
        checked += 1;

        // Invariant 1: tactile is laid out on the fixed 828x792 emboss page.
        match root_dimensions(&tactile) {
            Some((w, h)) if w == "828" && h == "792" => {}
            other => wrong_page.push(format!("{name}: tactile page {other:?}, expected 828x792")),
        }

        // Invariant 2: tactile output is not identical to the SVG build.
        if tactile == svg {
            identical.push(name.clone());
        }
    }

    eprintln!("tactile: {checked} examples built as both svg and tactile");

    assert!(
        wrong_page.is_empty(),
        "tactile page geometry wrong:\n{}",
        wrong_page.join("\n")
    );
    assert!(
        identical.is_empty(),
        "tactile output was byte-identical to the SVG build for:\n{}",
        identical.join("\n")
    );
    assert!(
        checked >= 100,
        "expected to build many examples both ways, only {checked}"
    );
}
