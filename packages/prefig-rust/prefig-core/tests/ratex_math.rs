//! Tests for the `ratex` feature: math labels rendered by the pure-Rust RaTeX
//! engine — no `node`, no JavaScript engine, works on native and wasm.
//! Runs only when built with `--features ratex`.
#![cfg(feature = "ratex")]

use prefig_core::core::label_tools::LabelState;
use prefig_core::engine::build_source_with;

const MATH_DIAGRAM: &str = r#"
<diagram dimensions="(200,200)" margins="5">
  <coordinates bbox="(-3,-3,3,3)">
    <point p="(0,0)"/>
    <label p="(0,0)" alignment="east"><m>\frac{-b \pm \sqrt{b^2-4ac}}{2a}</m></label>
  </coordinates>
</diagram>"#;

#[test]
fn ratex_renders_selfcontained_svg_paths() {
    // LabelState::local uses RaTeX when the `ratex` feature is enabled.
    let labels = LabelState::local("svg");
    let (svg, _annotations) =
        build_source_with("svg", MATH_DIAGRAM, "test", "pf_cli", labels, None, false)
            .expect("diagram builds");

    // RaTeX emits glyph outlines as <path> and embeds fonts, so the output is
    // self-contained: no <text> elements referencing external fonts.
    assert!(
        svg.contains("<path"),
        "expected glyph <path>s in the output"
    );
    assert!(
        !svg.contains("<text"),
        "expected no <text> (fonts should be embedded as paths)"
    );
}
