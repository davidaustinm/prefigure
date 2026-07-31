//! PreFigure core as a Typst plugin (`wasm-minimal-protocol` ABI).
//!
//! Typst drives; this module is called *by* Typst. Because a Typst plugin has no
//! way to call back into Typst (only two host imports exist, and no async — see
//! TYPST_PLUGIN_PLAN.md §2), text measurement is inverted into a metrics
//! handshake: Typst first asks which runs need measuring (`extract_measurables`),
//! measures them with its own layout engine, then hands the measurements back in
//! the build call (`build`). Math, in this POC build, is rendered entirely inside
//! the wasm by the embedded RaTeX engine, so Typst supplies only text metrics.
//!
//! The protocol logic (`extract`, `build`) is plain Rust and unit-tested on the
//! host (see `tests/`); the wasm-only [`abi`] module wraps it in the plugin ABI.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use prefig_core::core::label_tools::{
    measure_key, FontData, LabelMode, LabelState, MathLabel, MathLabels, NoBrailleTranslator,
    SuppliedMathLabels, SuppliedMathMetrics, SuppliedTextMeasurements, TextPlacement,
};
use prefig_core::engine;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Payload shapes (shared with packages/prefig-typst/src/*.typ)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct ExtractRequest {
    source: String,
    #[serde(default = "default_format")]
    format: String,
    #[serde(default)]
    font_map: HashMap<String, String>,
}

#[derive(Serialize)]
struct ExtractResponse {
    measurables: Vec<MeasurableJson>,
    /// Distinct `<m>` bodies the build produces (xmlit sentinels *and* math
    /// PreFigure generates itself, e.g. axis labels / tick numbers). The host
    /// decides which of these it will render (see lib.typ).
    math: Vec<String>,
}

#[derive(Deserialize)]
struct BuildRequest {
    source: String,
    #[serde(default = "default_format")]
    format: String,
    #[serde(default)]
    font_map: HashMap<String, String>,
    /// Where text labels go: `"svg"` (default) bakes `<text>` into the SVG;
    /// `"native"` omits them and returns their placements for the host to
    /// render natively (geometry and math stay in the SVG either way).
    #[serde(default = "default_labels")]
    labels: String,
    /// Text metrics measured by Typst, one per distinct run from Pass A.
    #[serde(default)]
    metrics: Vec<MetricJson>,
    /// Full-milestone only: math rendered by Typst, keyed by the `<m>` element
    /// id. Absent in the POC (math comes from the embedded RaTeX engine).
    #[serde(default)]
    math_svg: HashMap<String, String>,
    /// Typst-rendered math (native-labels): the [w, above, below] dimensions of
    /// each equation, keyed by the math *sentinel* the host wrote as the `<m>`
    /// body. When present, math is measured by Typst and drawn as native content
    /// overlaid on the SVG (like text), not baked in. Implies `labels: "native"`.
    #[serde(default)]
    math_dims: HashMap<String, [f64; 3]>,
}

/// A run to be measured (Pass A output / Pass B input).
#[derive(Serialize, Deserialize, Clone)]
struct MeasurableJson {
    text: String,
    family: String,
    size: f64,
    italic: bool,
    bold: bool,
}

/// A measured run (Pass B output / Pass C input): a measurable plus its
/// [advance-width, height-above-baseline, depth-below-baseline], in that order,
/// matching PreFigure's ink-extent triple.
#[derive(Deserialize)]
struct MetricJson {
    text: String,
    family: String,
    size: f64,
    italic: bool,
    bold: bool,
    width: f64,
    above: f64,
    below: f64,
}

fn default_format() -> String {
    "svg".to_string()
}

fn default_labels() -> String {
    "svg".to_string()
}

/// Native-mode response: the SVG (geometry + math, text omitted), the SVG
/// viewport size (SVG user units), and where each text run should be placed.
#[derive(Serialize)]
struct NativeResponse {
    svg: String,
    width: f64,
    height: f64,
    labels: Vec<PlacementJson>,
}

#[derive(Serialize)]
struct PlacementJson {
    text: String,
    family: String,
    size: f64,
    italic: bool,
    bold: bool,
    color: Option<String>,
    x: f64,
    y: f64,
    angle: f64,
    scale: f64,
    math: bool,
}

impl From<&TextPlacement> for PlacementJson {
    fn from(p: &TextPlacement) -> Self {
        PlacementJson {
            text: p.text.clone(),
            family: p.family.clone(),
            size: p.size,
            italic: p.italic,
            bold: p.bold,
            color: p.color.clone(),
            x: p.x,
            y: p.y,
            angle: p.angle,
            scale: p.scale,
            math: p.math,
        }
    }
}

/// Read the root `<svg>`'s width/height attributes (the viewport, in user units)
/// from a built document, so the host can size the native-text overlay to match.
fn svg_viewport(svg: &str) -> (f64, f64) {
    let parse = |attr: &str| -> Option<f64> {
        prefig_core::xml::parse_str(svg).ok()?.borrow().get(attr)?.parse().ok()
    };
    (parse("width").unwrap_or(0.0), parse("height").unwrap_or(0.0))
}

// ---------------------------------------------------------------------------
// Protocol logic (portable — no ABI, host-testable)
// ---------------------------------------------------------------------------

/// Pass A — enumerate the distinct text runs a build of `source` will measure.
pub fn extract(payload: &[u8]) -> Result<Vec<u8>, String> {
    let req: ExtractRequest =
        serde_json::from_slice(payload).map_err(|e| format!("bad extract payload: {e}"))?;

    // Run a real build with recording backends; the SVG is discarded, but every
    // measure_text request and every <m> body is captured — the same code path
    // the real build takes, so the enumerated set cannot disagree.
    let text_sink: Rc<RefCell<Vec<(String, FontData)>>> = Rc::new(RefCell::new(Vec::new()));
    let math_sink: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let labels = LabelState::collecting(text_sink.clone(), math_sink.clone(), req.font_map);
    engine::build_source_with(&req.format, &req.source, "typst", "typst", labels, None, false)?;

    // Order-preserving dedupe of text runs by the same key measure_text uses.
    let mut seen: HashSet<String> = HashSet::new();
    let mut measurables: Vec<MeasurableJson> = Vec::new();
    for (text, font) in text_sink.borrow().iter() {
        let key = measure_key(text, &font.family, font.size, font.italic, font.bold);
        if seen.insert(key) {
            measurables.push(MeasurableJson {
                text: text.clone(),
                family: font.family.clone(),
                size: font.size,
                italic: font.italic,
                bold: font.bold,
            });
        }
    }

    // Order-preserving dedupe of math bodies.
    let mut math_seen: HashSet<String> = HashSet::new();
    let mut math: Vec<String> = Vec::new();
    for body in math_sink.borrow().iter() {
        if math_seen.insert(body.clone()) {
            math.push(body.clone());
        }
    }

    serde_json::to_vec(&ExtractResponse { measurables, math })
        .map_err(|e| format!("encoding measurables: {e}"))
}

/// Pass C — build `source` into SVG, answering measurements from `metrics`.
pub fn build(payload: &[u8]) -> Result<Vec<u8>, String> {
    let req: BuildRequest =
        serde_json::from_slice(payload).map_err(|e| format!("bad build payload: {e}"))?;

    let mut table: HashMap<String, [f64; 3]> = HashMap::new();
    for m in &req.metrics {
        let key = measure_key(&m.text, &m.family, m.size, m.italic, m.bold);
        table.insert(key, [m.width, m.above, m.below]);
    }

    // Typst-rendered math (math_dims supplied) forces native labels: the math
    // is measured and drawn by Typst, so text must be too, for one coherent pass.
    let native = req.labels == "native" || !req.math_dims.is_empty();
    let placements: Rc<RefCell<Vec<TextPlacement>>> = Rc::new(RefCell::new(Vec::new()));

    // A `<m>` that no backend can draw is dropped from its row by the build and
    // disappears from the diagram without a trace, so watch for that and report
    // it below rather than silently losing the author's math.
    let unrenderable: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let math = Box::new(TrackUnrenderable {
        inner: build_math_backend(&req),
        bodies: HashMap::new(),
        missing: unrenderable.clone(),
    });
    let labels = LabelState {
        math,
        text: Box::new(SuppliedTextMeasurements { table }),
        braille: Box::new(NoBrailleTranslator),
        font_map: req.font_map,
        label_mode: if native { LabelMode::Native } else { LabelMode::Svg },
        placements: placements.clone(),
    };

    let (svg, _annotations) =
        engine::build_source_with(&req.format, &req.source, "typst", "typst", labels, None, false)?;

    let missing = unrenderable.borrow();
    if !missing.is_empty() {
        return Err(format!(
            "no way to render the math {:?}: it has no supplied dimensions in `math_dims` \
             and this build embeds no math engine, so it would be dropped from the diagram",
            missing
        ));
    }
    drop(missing);

    if !native {
        return Ok(svg.into_bytes());
    }

    // Native mode: return SVG + viewport + the text placements as JSON.
    let (width, height) = svg_viewport(&svg);
    let labels: Vec<PlacementJson> = placements.borrow().iter().map(PlacementJson::from).collect();
    serde_json::to_vec(&NativeResponse { svg, width, height, labels })
        .map_err(|e| format!("encoding native response: {e}"))
}

/// Wraps a math backend and records every `<m>` it can render *no* way at all —
/// neither as host-drawn native math (supplied dimensions) nor as SVG glyphs.
///
/// The build drops such an element from its row and carries on, which is the
/// right behaviour for a viewer missing an optional service but far too quiet
/// for this plugin: the equation vanishes and the surrounding text closes up as
/// if the author had never written it. [`build`] turns anything recorded here
/// into an error instead.
struct TrackUnrenderable {
    inner: Box<dyn MathLabels>,
    /// `<m>` id -> body, kept so a report can name the math that was lost.
    bodies: HashMap<String, String>,
    missing: Rc<RefCell<Vec<String>>>,
}

impl MathLabels for TrackUnrenderable {
    fn add_macros(&mut self, macros: &str) {
        self.inner.add_macros(macros);
    }

    fn register_math_label(&mut self, id: &str, text: &str) {
        self.bodies.insert(id.to_string(), text.to_string());
        self.inner.register_math_label(id, text);
    }

    fn process_math_labels(&mut self) -> Result<(), String> {
        self.inner.process_math_labels()
    }

    fn get_math_label(&self, id: &str) -> Option<MathLabel> {
        let label = self.inner.get_math_label(id);
        // `None` is legitimate for native math — the host draws it — so only an
        // element that is neither drawn here nor placed natively is lost.
        if label.is_none() && self.inner.native_math(id).is_none() {
            let body = self.bodies.get(id).cloned().unwrap_or_else(|| id.to_string());
            let mut missing = self.missing.borrow_mut();
            if !missing.contains(&body) {
                missing.push(body);
            }
        }
        label
    }

    fn native_math(&self, id: &str) -> Option<(String, [f64; 3])> {
        self.inner.native_math(id)
    }
}

/// Choose the math backend, in priority order: Typst-measured native math
/// (`math_dims`, drawn as overlaid content), then injected Typst SVG
/// (`math_svg`), then the embedded RaTeX engine (the POC path).
fn build_math_backend(req: &BuildRequest) -> Box<dyn MathLabels> {
    if !req.math_dims.is_empty() {
        // Host-rendered math for the sentinels; non-sentinel math (axis labels,
        // tick numbers PreFigure generates) still renders via the RaTeX fallback.
        return Box::new(SuppliedMathMetrics::new(
            req.math_dims.clone(),
            ratex_backend(&req.format),
        ));
    }
    if !req.math_svg.is_empty() {
        let mut svg = HashMap::new();
        for (id, s) in &req.math_svg {
            if let Ok(el) = prefig_core::xml::parse_str(s) {
                svg.insert(id.clone(), el);
            }
        }
        return Box::new(SuppliedMathLabels { svg });
    }
    ratex_backend(&req.format)
}

#[cfg(feature = "ratex-math")]
fn ratex_backend(format: &str) -> Box<dyn MathLabels> {
    Box::new(prefig_core::core::ratex_math::RatexMathLabels::new(format))
}

#[cfg(not(feature = "ratex-math"))]
fn ratex_backend(_format: &str) -> Box<dyn MathLabels> {
    Box::new(prefig_core::core::label_tools::NoMathLabels)
}

// ---------------------------------------------------------------------------
// wasm-minimal-protocol ABI (wasm target only)
// ---------------------------------------------------------------------------

/// The Typst plugin ABI. Two linked host functions in module `typst_env`; each
/// export takes N `bytes` arguments (lengths arrive as `usize` params) and
/// returns its result via `send`. A non-zero return marks the sent bytes as an
/// error string. Arity is fixed per export, so each export takes exactly one
/// bytes argument holding a JSON payload.
#[cfg(target_arch = "wasm32")]
mod abi {
    use super::extract;

    #[link(wasm_import_module = "typst_env")]
    extern "C" {
        fn wasm_minimal_protocol_write_args_to_buffer(ptr: *mut u8);
        fn wasm_minimal_protocol_send_result_to_host(ptr: *const u8, len: usize);
    }

    fn read_arg(len: usize) -> Vec<u8> {
        let mut buf = vec![0u8; len];
        unsafe { wasm_minimal_protocol_write_args_to_buffer(buf.as_mut_ptr()) };
        buf
    }

    fn send(bytes: &[u8]) {
        unsafe { wasm_minimal_protocol_send_result_to_host(bytes.as_ptr(), bytes.len()) };
    }

    fn dispatch(len: usize, f: impl FnOnce(&[u8]) -> Result<Vec<u8>, String>) -> i32 {
        match f(&read_arg(len)) {
            Ok(out) => {
                send(&out);
                0
            }
            Err(e) => {
                send(e.as_bytes());
                1
            }
        }
    }

    /// Pass A export.
    #[no_mangle]
    pub extern "C" fn extract_measurables(len: usize) -> i32 {
        dispatch(len, extract)
    }

    /// Pass C export.
    #[no_mangle]
    pub extern "C" fn build(len: usize) -> i32 {
        dispatch(len, super::build)
    }
}
