//! Bridge between prefig-core's label traits and the browser's
//! `PrefigBrowserApi` object (see
//! packages/playground/src/worker/compat-api.ts). All calls are
//! synchronous, matching how the Python version used the same object.

use js_sys::{Function, Reflect};
use prefig_core::core::label_tools::{
    BrailleTranslator, FontData, LabelState, MathLabels, TextMeasurements,
};
// Only the host math backend (below) turns a host `processMath` string into an
// SVG label; the `ratex` build renders math in-module and needs neither.
#[cfg(not(feature = "ratex"))]
use prefig_core::core::label_tools::MathLabel;
#[cfg(not(feature = "ratex"))]
use prefig_core::xml;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

thread_local! {
    static HOST_API: RefCell<Option<JsValue>> = const { RefCell::new(None) };
}

pub fn set_host_api(api: JsValue) {
    HOST_API.with(|h| *h.borrow_mut() = Some(api));
}

fn host() -> Option<JsValue> {
    HOST_API.with(|h| h.borrow().clone())
}

/// Call `api.method(args...)` and return the result, or None on any failure.
fn call_method(method: &str, args: &[JsValue]) -> Option<JsValue> {
    let api = host()?;
    let f = Reflect::get(&api, &JsValue::from_str(method)).ok()?;
    let f: Function = f.dyn_into().ok()?;
    let js_args = js_sys::Array::new();
    for a in args {
        js_args.push(a);
    }
    Reflect::apply(&f, &api, &js_args).ok()
}

/// The label services for a WASM build, all backed by the host object.
pub fn label_state(format: &str) -> LabelState {
    // `ratex` (pure Rust) renders math inside the wasm module, so the host need
    // not provide `processMath`. Without it, math comes from the host callback.
    // (There is no in-wasm JS-engine math backend: `mathjax-js`/QuickJS is
    // native-only.) Text measurement and braille always come from the host.
    #[cfg(feature = "ratex")]
    let math: Box<dyn MathLabels> =
        Box::new(prefig_core::core::ratex_math::RatexMathLabels::new(format));
    #[cfg(not(feature = "ratex"))]
    let math: Box<dyn MathLabels> = Box::new(HostMathLabels::new(format));

    LabelState {
        math,
        text: Box::new(HostTextMeasurements),
        braille: Box::new(HostBrailleTranslator),
        font_map: Default::default(),
        label_mode: Default::default(),
        placements: Default::default(),
    }
}

/// Mirrors PyodideMathLabels: register TeX, render each on demand via the host.
/// Only built when no in-module math engine is selected — with `ratex`, math is
/// rendered inside the wasm module and the host is never asked for
/// `processMath`/`processBraille`.
#[cfg(not(feature = "ratex"))]
struct HostMathLabels {
    registered: Vec<(String, String)>,
    format: String,
}

#[cfg(not(feature = "ratex"))]
impl HostMathLabels {
    fn new(format: &str) -> HostMathLabels {
        HostMathLabels {
            registered: Vec::new(),
            format: format.to_string(),
        }
    }
}

#[cfg(not(feature = "ratex"))]
impl MathLabels for HostMathLabels {
    fn add_macros(&mut self, _macros: &str) {}

    fn register_math_label(&mut self, id: &str, text: &str) {
        self.registered.push((id.to_string(), text.to_string()));
    }

    fn process_math_labels(&mut self) -> Result<(), String> {
        // rendering happens lazily in get_math_label, like the Pyodide path
        Ok(())
    }

    fn get_math_label(&self, id: &str) -> Option<MathLabel> {
        let (_, tex) = self.registered.iter().find(|(rid, _)| rid == id)?;
        if self.format == "tactile" {
            let braille = call_method("processBraille", &[JsValue::from_str(tex)])?;
            return Some(MathLabel::Braille(braille.as_string()?));
        }
        let svg = call_method("processMath", &[JsValue::from_str(tex)])?;
        let svg_string = svg.as_string()?;
        // the host returns an <mjx-container>…<svg>…; extract the <svg>
        let doc = xml::parse_str(&svg_string).ok()?;
        let svg_el = if doc.borrow().tag == "svg" {
            doc
        } else {
            xml::find_descendants(&doc, "svg").into_iter().next()?
        };
        Some(MathLabel::Svg(svg_el))
    }
}

/// Mirrors PyodideTextMeasurements: build the CSS font string and call the host.
struct HostTextMeasurements;

impl TextMeasurements for HostTextMeasurements {
    fn measure_text(&self, text: &str, font: &FontData) -> Option<[f64; 3]> {
        let mut font_string = String::new();
        if font.italic {
            font_string.push_str("italic ");
        }
        if font.bold {
            font_string.push_str("bold ");
        }
        font_string.push_str(&format!("{}px {}", font.size, font.family));

        let result = call_method(
            "measure_text",
            &[JsValue::from_str(text), JsValue::from_str(&font_string)],
        )?;
        let array: js_sys::Array = result.dyn_into().ok()?;
        Some([
            array.get(0).as_f64()?,
            array.get(1).as_f64()?,
            array.get(2).as_f64()?,
        ])
    }
}

/// Mirrors PyodideBrailleTranslator.
struct HostBrailleTranslator;

impl BrailleTranslator for HostBrailleTranslator {
    fn initialized(&self) -> bool {
        host().is_some()
    }

    fn translate(&self, text: &str, typeform: &[u8]) -> Option<String> {
        let typeform_array = js_sys::Array::new();
        for t in typeform {
            typeform_array.push(&JsValue::from_f64(*t as f64));
        }
        let result = call_method(
            "translate_text",
            &[JsValue::from_str(text), typeform_array.into()],
        )?;
        result.as_string()
    }
}
