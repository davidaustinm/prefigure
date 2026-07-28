//! Math-label rendering with the pure-Rust **RaTeX** engine (`ratex` feature).
//!
//! Unlike the node/MathJax backend (`LocalMathLabels`) and the embedded-JS
//! backend (`mathjax_js`), this needs no external process, no host callback,
//! and *no JavaScript engine at all*: `ratex-parser` + `ratex-layout` +
//! `ratex-svg` turn a LaTeX string straight into self-contained SVG `<path>`
//! glyphs (KaTeX fonts embedded at build time). It compiles and runs
//! identically on native and `wasm32-unknown-unknown`, so a WASM build can
//! render math with nothing from the host environment.
//!
//! Trade-off: the output is KaTeX-styled, not MathJax-styled, so labels look a
//! little different from the node/MathJax path.
//!
//! ## When is this backend safe to remove?
//!
//! `ratex` exists because MathJax cannot be run in a wasm-capable JavaScript
//! engine today: QuickJS (used by the native `mathjax-js` backend) does not
//! compile to `wasm32-unknown-unknown` (it is C and that target has no libc),
//! and Boa — the only pure-Rust engine that targets wasm — cannot run MathJax
//! (its TeX *and* MathML→SVG paths hit an internal "not a callable function"
//! bug; see `mathjax_js.rs`). RaTeX side-steps the whole problem by rendering
//! LaTeX→SVG in Rust.
//!
//! This backend can be dropped (folding wasm back onto `mathjax-js` for output
//! identical to the native/node path) once **either** holds:
//!   1. Boa can run the MathJax bundle end-to-end (superscripts, fractions,
//!      MathML→SVG) — retest with
//!      `packages/prefig-rust/prefig-core/assets/mathjax_headless.js`
//!      in a Boa `Context`; if `x^2` yields a `<svg>` with glyph paths, the
//!      `mathjax-js` wasm path is viable and RaTeX is redundant; or
//!   2. QuickJS (or another complete engine) gains a `wasm32-unknown-unknown`
//!      build that needs nothing from the host.
//!
//! Until then RaTeX is the only way to render math in a host-free wasm build,
//! and it doubles as a zero-dependency native backend.

use crate::core::label_tools::{MathLabel, MathLabels};
use crate::xml;

/// Maps RaTeX's em-based metrics onto the `ex` units PreFigure's label placer
/// expects (`mk_m_element` treats 1ex as 8px). ~2.25 keeps rendered math close
/// to the size the MathJax path produced. Tune here if labels look off.
const EX_PER_EM: f64 = 2.25;

pub struct RatexMathLabels {
    registered: Vec<(String, String)>,
    rendered: Vec<(String, String)>,
}

impl RatexMathLabels {
    pub fn new(_format: &str) -> RatexMathLabels {
        RatexMathLabels {
            registered: Vec::new(),
            rendered: Vec::new(),
        }
    }
}

/// LaTeX -> a PreFigure-ready SVG string (root carries `ex` width/height and a
/// `vertical-align` style so `mk_m_element` can size and baseline-align it).
fn render(tex: &str) -> Result<String, String> {
    let nodes = ratex_parser::parse(tex).map_err(|e| format!("{e:?}"))?;
    let layout = ratex_layout::engine::layout(&nodes, &ratex_layout::layout_options::LayoutOptions::default());
    let list = ratex_layout::to_display::to_display_list(&layout);

    // Metrics are in em; convert to the ex units PreFigure places labels in.
    let width = list.width * EX_PER_EM;
    let depth = list.depth * EX_PER_EM;
    let total_height = (list.height + list.depth) * EX_PER_EM;

    let mut opts = ratex_svg::SvgOptions {
        embed_glyphs: true,
        padding: 0.0,
        ..Default::default()
    };
    // font_size only scales RaTeX's internal viewBox; the on-page size is set by
    // the ex attributes below, which PreFigure scales the viewBox into.
    opts.font_size = 40.0;
    let svg = ratex_svg::render_to_svg(&list, &opts);

    // Override the root <svg>'s pt width/height with ex dimensions and add the
    // baseline offset, matching what MathJax's SVG output carries.
    let doc = xml::parse_str(&svg)?;
    {
        let mut root = doc.borrow_mut();
        root.set("width", &format!("{width:.3}ex"));
        root.set("height", &format!("{total_height:.3}ex"));
        root.set("style", &format!("vertical-align: {:.3}ex", -depth));
    }
    Ok(xml::to_string(&doc))
}

impl MathLabels for RatexMathLabels {
    fn add_macros(&mut self, _macros: &str) {
        // RaTeX has no \newcommand preamble hook; custom macros are unsupported.
    }

    fn register_math_label(&mut self, id: &str, text: &str) {
        self.registered.push((id.to_string(), text.to_string()));
    }

    fn process_math_labels(&mut self) -> Result<(), String> {
        for (id, tex) in &self.registered {
            match render(tex) {
                Ok(svg) => self.rendered.push((id.clone(), svg)),
                Err(e) => log::warn!("RaTeX failed to render label {id} ({tex:?}): {e}"),
            }
        }
        Ok(())
    }

    fn get_math_label(&self, id: &str) -> Option<MathLabel> {
        let (_, svg) = self.rendered.iter().find(|(rid, _)| rid == id)?;
        let doc = xml::parse_str(svg).ok()?;
        let svg = if doc.borrow().tag == "svg" {
            doc
        } else {
            xml::find_descendants(&doc, "svg").into_iter().next()?
        };
        Some(MathLabel::Svg(svg))
    }
}
