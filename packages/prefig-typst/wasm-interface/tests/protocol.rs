//! Host-side tests for the metrics-injection protocol (Passes A and C), run
//! natively against `prefig_typst_plugin::{extract, build}` — no wasm, no Typst.
//! These lock down the two invariants the handshake depends on:
//!   * Pass A enumerates the runs the build will measure (font-mapped, deduped);
//!   * Pass C's keys line up with Pass A's, so every measurement is consumed.

use prefig_typst_plugin::{build, extract};
use serde_json::{json, Value};

const TEXT_DIAGRAM: &str = r#"
<diagram dimensions="(200,200)" margins="5">
  <coordinates bbox="[-5,-5,5,5]">
    <label p="(1,2)" alignment="east">Hello</label>
    <label p="(-2,-3)" alignment="north"><it>slanted</it> and <b>bold</b></label>
  </coordinates>
</diagram>
"#;

fn font_map() -> Value {
    json!({
        "serif": "New Computer Modern",
        "sans-serif": "DejaVu Sans",
        "monospace": "DejaVu Sans Mono",
    })
}

fn extract_measurables(source: &str) -> Vec<Value> {
    let payload = json!({ "source": source, "format": "svg", "font_map": font_map() });
    let out = extract(payload.to_string().as_bytes()).expect("extract failed");
    let parsed: Value = serde_json::from_slice(&out).unwrap();
    parsed["measurables"].as_array().unwrap().clone()
}

#[test]
fn pass_a_enumerates_font_mapped_runs() {
    let runs = extract_measurables(TEXT_DIAGRAM);
    let texts: Vec<&str> = runs.iter().map(|m| m["text"].as_str().unwrap()).collect();
    assert_eq!(texts, ["Hello", "slanted", "and", "bold"]);

    // The generic "sans-serif" must have been mapped to the concrete family, or
    // Typst would measure one font and render another (§4.1).
    for m in &runs {
        assert_eq!(m["family"], "DejaVu Sans");
    }

    // Styles are carried through: <it> italic, <b> bold.
    let slanted = runs.iter().find(|m| m["text"] == "slanted").unwrap();
    assert_eq!(slanted["italic"], true);
    let bold = runs.iter().find(|m| m["text"] == "bold").unwrap();
    assert_eq!(bold["bold"], true);
}

#[test]
fn pass_a_dedupes_identical_runs() {
    let src = r#"
    <diagram dimensions="(200,200)" margins="5">
      <coordinates bbox="[-5,-5,5,5]">
        <label p="(0,0)">same</label>
        <label p="(1,1)">same</label>
      </coordinates>
    </diagram>"#;
    let runs = extract_measurables(src);
    assert_eq!(runs.len(), 1, "identical (text, font) runs should collapse");
}

#[test]
fn pass_c_consumes_every_pass_a_key() {
    // Simulate Typst: measure each enumerated run with a stand-in metric, feed
    // it back to the build, and confirm the build reports no missing keys and
    // emits each run's text into the SVG.
    let runs = extract_measurables(TEXT_DIAGRAM);
    let metrics: Vec<Value> = runs
        .iter()
        .map(|m| {
            let mut m = m.clone();
            let obj = m.as_object_mut().unwrap();
            obj.insert("width".into(), json!(20.0));
            obj.insert("above".into(), json!(10.0));
            obj.insert("below".into(), json!(2.0));
            m
        })
        .collect();

    let payload = json!({
        "source": TEXT_DIAGRAM,
        "format": "svg",
        "font_map": font_map(),
        "metrics": metrics,
    });
    let svg_bytes = build(payload.to_string().as_bytes()).expect("build failed");
    let svg = String::from_utf8(svg_bytes).unwrap();

    // The concrete family — not the generic name — must reach the SVG.
    assert!(svg.contains("DejaVu Sans"), "concrete font-family missing from SVG");
    assert!(!svg.contains("font-family=\"sans-serif\""), "generic family leaked into SVG");
    for t in ["Hello", "slanted", "and", "bold"] {
        assert!(svg.contains(t), "run {t:?} missing from built SVG");
    }
}

#[test]
fn native_mode_omits_text_and_returns_placements() {
    // Measure, then build in native mode: the SVG must carry no <text>, and each
    // run must come back as a placement with the concrete family and a baseline
    // point inside the viewport.
    let runs = extract_measurables(TEXT_DIAGRAM);
    let metrics: Vec<Value> = runs
        .iter()
        .map(|m| {
            let mut m = m.clone();
            let obj = m.as_object_mut().unwrap();
            obj.insert("width".into(), json!(20.0));
            obj.insert("above".into(), json!(10.0));
            obj.insert("below".into(), json!(2.0));
            m
        })
        .collect();

    let payload = json!({
        "source": TEXT_DIAGRAM,
        "format": "svg",
        "labels": "native",
        "font_map": font_map(),
        "metrics": metrics,
    });
    let out = build(payload.to_string().as_bytes()).expect("native build failed");
    let resp: Value = serde_json::from_slice(&out).unwrap();

    let svg = resp["svg"].as_str().unwrap();
    assert!(!svg.contains("<text"), "native mode must omit <text> from the SVG");

    let (w, h) = (resp["width"].as_f64().unwrap(), resp["height"].as_f64().unwrap());
    assert!(w > 0.0 && h > 0.0, "viewport must be reported");

    let placements = resp["labels"].as_array().unwrap();
    let texts: Vec<&str> = placements.iter().map(|p| p["text"].as_str().unwrap()).collect();
    assert_eq!(texts, ["Hello", "slanted", "and", "bold"]);
    for p in placements {
        assert_eq!(p["family"], "DejaVu Sans");
        let (x, y) = (p["x"].as_f64().unwrap(), p["y"].as_f64().unwrap());
        assert!(x >= 0.0 && x <= w && y >= 0.0 && y <= h, "placement {x},{y} outside viewport");
    }
}

#[test]
fn native_math_returns_placement_and_omits_glyphs() {
    // A label whose math is a host sentinel with supplied dims: the build must
    // report it as a math placement (not draw it), keyed by the sentinel.
    let src = r#"
    <diagram dimensions="(200,200)" margins="5">
      <coordinates bbox="[-5,-5,5,5]">
        <label p="(0,0)"><m>⟦math-0⟧</m></label>
      </coordinates>
    </diagram>"#;
    let payload = json!({
        "source": src,
        "format": "svg",
        "math_dims": { "⟦math-0⟧": [18.0, 9.0, 3.0] },  // implies native
    });
    let out = build(payload.to_string().as_bytes()).expect("native-math build failed");
    let resp: Value = serde_json::from_slice(&out).unwrap();

    let math: Vec<&Value> = resp["labels"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["math"] == true)
        .collect();
    assert_eq!(math.len(), 1, "expected one math placement");
    assert_eq!(math[0]["text"], "⟦math-0⟧", "placement must carry the sentinel");
}

#[test]
fn native_legend_math_is_laid_out_in_the_box() {
    // A <legend> with two math items. In native mode each item's math is handed
    // back as a placement; the legend must re-anchor them to its own layout, so
    // the two land at *different* points inside the viewport — not stacked on top
    // of each other at the legend's raw anchor (the pre-fix behaviour), and drawn
    // at the legend's scale.
    let src = r#"
    <diagram dimensions="(300,300)" margins="5">
      <coordinates bbox="[-1,-3,6,3]">
        <point at="x" p="(1,1)"/>
        <point at="xprime" p="(2,2)"/>
        <legend at="legend" anchor="(bbox[2], bbox[3])"
                alignment="sw" scale="0.9">
          <item ref="x"><m>x(t)</m></item>
          <item ref="xprime"><m>x'(t)</m></item>
        </legend>
      </coordinates>
    </diagram>"#;
    let payload = json!({
        "source": src,
        "format": "svg",
        "math_dims": {
            "x(t)":  [18.0, 9.0, 3.0],
            "x'(t)": [20.0, 9.0, 3.0],
        },
    });
    let out = build(payload.to_string().as_bytes()).expect("native legend build failed");
    let resp: Value = serde_json::from_slice(&out).unwrap();

    let (w, h) = (resp["width"].as_f64().unwrap(), resp["height"].as_f64().unwrap());
    let math: Vec<&Value> = resp["labels"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["math"] == true)
        .collect();
    assert_eq!(math.len(), 2, "expected two math placements");

    let pt = |p: &Value| (p["x"].as_f64().unwrap(), p["y"].as_f64().unwrap());
    let (x0, y0) = pt(math[0]);
    let (x1, y1) = pt(math[1]);

    // The two items stack vertically in the legend, so their baselines differ.
    // Before the fix both were recorded at the legend anchor → identical points.
    assert!(
        (y0 - y1).abs() > 1.0,
        "legend math items must be laid out at different rows, got y0={y0}, y1={y1}"
    );
    // Both land inside the viewport.
    for (x, y) in [(x0, y0), (x1, y1)] {
        assert!(x >= 0.0 && x <= w && y >= 0.0 && y <= h, "placement {x},{y} outside viewport");
    }
    // The legend's scale reaches the host-drawn math.
    for p in &math {
        assert!(
            (p["scale"].as_f64().unwrap() - 0.9).abs() < 1e-9,
            "legend scale must apply to native math, got {}",
            p["scale"]
        );
    }
}

#[test]
fn build_reports_xml_errors() {
    let payload = json!({ "source": "<diagram><unclosed>", "format": "svg" });
    assert!(build(payload.to_string().as_bytes()).is_err());
}
