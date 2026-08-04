//! Port of prefig/core/label_tools.py: the seam between PreFigure and the
//! outside services it needs for labels — math rendering (MathJax), text
//! measurement, and braille translation. Native builds shell out to Node for
//! MathJax exactly like the Python version; the WebAssembly build implements
//! these traits over the browser's PrefigBrowserApi instead.

use crate::xml::{self, El};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// (family, size, italic, bold, color)
#[derive(Clone, Debug)]
pub struct FontData {
    pub family: String,
    pub size: f64,
    pub italic: bool,
    pub bold: bool,
    pub color: Option<String>,
}

pub enum MathLabel {
    Svg(El),
    Braille(String),
}

pub trait MathLabels {
    fn add_macros(&mut self, macros: &str);
    fn register_math_label(&mut self, id: &str, text: &str);
    fn process_math_labels(&mut self) -> Result<(), String>;
    fn get_math_label(&self, id: &str) -> Option<MathLabel>;
    /// Native-math hook (`LabelMode::Native` with host-rendered math): if this
    /// backend supplies *dimensions only* for the `<m>` element `id` — leaving
    /// the glyphs for the host to draw — return its (sentinel, [w, above,
    /// below]). Default `None`: the backend draws math into the SVG itself.
    fn native_math(&self, _id: &str) -> Option<(String, [f64; 3])> {
        None
    }
}

pub trait TextMeasurements {
    /// Returns [advance_width, height_above_baseline, depth_below_baseline].
    fn measure_text(&self, text: &str, font: &FontData) -> Option<[f64; 3]>;
}

pub trait BrailleTranslator {
    fn initialized(&self) -> bool;
    fn translate(&self, text: &str, typeform: &[u8]) -> Option<String>;
}

pub struct LabelState {
    pub math: Box<dyn MathLabels>,
    pub text: Box<dyn TextMeasurements>,
    pub braille: Box<dyn BrailleTranslator>,
    /// Maps PreFigure's generic font names (`serif`, `sans-serif`, `monospace`)
    /// to concrete font families. Empty in every native/browser build, so the
    /// generic name is used unchanged. The Typst plugin build (see
    /// `packages/prefig-typst`) fills it in because Typst's SVG font resolver
    /// does not support generic families, and because the family Typst *measures*
    /// with must be the family it later *renders* with. Applied in
    /// `label::position_svg_label`, so both `measure_text` and the emitted
    /// `font-family` attribute see the concrete family.
    pub font_map: HashMap<String, String>,
    /// How text labels leave the build. `Svg` (the default, and the only mode
    /// native/browser builds use) draws each run as an SVG `<text>` element.
    /// `Native` instead omits the `<text>` and records where it *would* have
    /// gone in `placements`, so the host (the Typst plugin) can stamp its own
    /// native text there — live host fonts, at PreFigure-computed positions.
    /// Math and geometry are unaffected either way.
    pub label_mode: LabelMode,
    /// In `Native` mode, the absolute placement of every omitted text run
    /// (shared handle so the caller can read it after the build). Empty in
    /// `Svg` mode.
    pub placements: Rc<RefCell<Vec<TextPlacement>>>,
}

/// Where text labels are rendered: baked into the SVG, or handed back for the
/// host to render natively. See [`LabelState::label_mode`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LabelMode {
    #[default]
    Svg,
    Native,
}

/// One text run the build placed but did not draw (`LabelMode::Native`). `x`/`y`
/// are the run's baseline origin in absolute SVG user units — the anchor point
/// SVG `<text>` uses — after the label group's translate/scale/rotate have been
/// folded in. `angle` (degrees, SVG sense) and `scale` are the group's residual
/// linear transform, to be applied to the glyphs; both are usually 0 and 1.
#[derive(Clone, Debug)]
pub struct TextPlacement {
    pub text: String,
    pub family: String,
    pub size: f64,
    pub italic: bool,
    pub bold: bool,
    pub color: Option<String>,
    pub x: f64,
    pub y: f64,
    pub angle: f64,
    pub scale: f64,
    /// When true, this is a math label: `text` holds the host's math sentinel
    /// (which the host maps back to the equation to render), not literal text.
    pub math: bool,
}

/// The canonical key identifying a text-measurement request, shared by the
/// injection table and every `measure_text` lookup so the two always agree.
/// `size` is formatted to a fixed precision so `14` and `14.0` (which JSON
/// round-tripping through Typst may interconvert) map to the same key.
pub fn measure_key(text: &str, family: &str, size: f64, italic: bool, bold: bool) -> String {
    format!(
        "{text}\u{1f}{family}\u{1f}{size:.4}\u{1f}{}\u{1f}{}",
        italic as u8, bold as u8
    )
}

/// A text backend that measures nothing but *records* every request, returning a
/// fixed placeholder so the build still completes. Running a full build with
/// this backend and discarding the SVG yields exactly the set of runs the real
/// build will measure — the same code path, so the two cannot drift (this is why
/// the plugin enumerates by building rather than by a hand-written label walk).
pub struct CollectingTextMeasurements {
    pub sink: Rc<RefCell<Vec<(String, FontData)>>>,
    pub placeholder: [f64; 3],
}

impl TextMeasurements for CollectingTextMeasurements {
    fn measure_text(&self, text: &str, font: &FontData) -> Option<[f64; 3]> {
        self.sink
            .borrow_mut()
            .push((text.to_string(), font.clone()));
        Some(self.placeholder)
    }
}

/// The math counterpart of `CollectingTextMeasurements`: records every `<m>`
/// element's body (the text that would be typeset) during a Pass A build, so the
/// host can decide which it will render. Draws nothing.
pub struct CollectingMathLabels {
    pub sink: Rc<RefCell<Vec<String>>>,
}

impl MathLabels for CollectingMathLabels {
    fn add_macros(&mut self, _macros: &str) {}
    fn register_math_label(&mut self, _id: &str, text: &str) {
        self.sink.borrow_mut().push(text.to_string());
    }
    fn process_math_labels(&mut self) -> Result<(), String> {
        Ok(())
    }
    fn get_math_label(&self, _id: &str) -> Option<MathLabel> {
        None
    }
}

/// A text backend that answers every `measure_text` from a pre-supplied table
/// (Typst measured these with its own layout engine, §4 of the plugin plan).
pub struct SuppliedTextMeasurements {
    pub table: HashMap<String, [f64; 3]>,
}

impl TextMeasurements for SuppliedTextMeasurements {
    fn measure_text(&self, text: &str, font: &FontData) -> Option<[f64; 3]> {
        let key = measure_key(text, &font.family, font.size, font.italic, font.bold);
        let hit = self.table.get(&key).copied();
        if hit.is_none() {
            log::warn!("no supplied measurement for text run {text:?} (key {key:?})");
        }
        hit
    }
}

/// A math backend that answers `get_math_label` from pre-rendered SVG fragments
/// (Typst rendered and measured the math, §3 full milestone). Registration and
/// processing are no-ops because the SVG is supplied ready-made.
pub struct SuppliedMathLabels {
    pub svg: HashMap<String, El>,
}

impl MathLabels for SuppliedMathLabels {
    fn add_macros(&mut self, _macros: &str) {}
    fn register_math_label(&mut self, _id: &str, _text: &str) {}
    fn process_math_labels(&mut self) -> Result<(), String> {
        Ok(())
    }
    fn get_math_label(&self, id: &str) -> Option<MathLabel> {
        self.svg
            .get(id)
            .map(|el| MathLabel::Svg(xml::deep_copy(el)))
    }
}

/// A math backend for `LabelMode::Native` with host-rendered math: it supplies
/// only each `<m>`'s *dimensions* (measured by the host from the real equation),
/// leaving the glyphs for the host to draw natively. `dims` is keyed by the
/// host's math *sentinel* — the text the host wrote as the `<m>` body — and
/// `register_math_label` records the `<m>` id → sentinel mapping so placement
/// can look the dimensions up. `get_math_label` returns `None`: no SVG is drawn;
/// `label::position_svg_label` reads `native_math` and records a placement.
pub struct SuppliedMathMetrics {
    /// sentinel -> [advance-width, above-baseline, below-baseline]
    pub dims: HashMap<String, [f64; 3]>,
    id_to_sentinel: HashMap<String, String>,
    /// Backend for `<m>` elements that are *not* host sentinels — e.g. the math
    /// PreFigure generates itself (axis labels, tick numbers). Those have no
    /// supplied dims, so they still render into the SVG here (typically RaTeX).
    fallback: Box<dyn MathLabels>,
}

impl SuppliedMathMetrics {
    pub fn new(
        dims: HashMap<String, [f64; 3]>,
        fallback: Box<dyn MathLabels>,
    ) -> SuppliedMathMetrics {
        SuppliedMathMetrics {
            dims,
            id_to_sentinel: HashMap::new(),
            fallback,
        }
    }
}

impl MathLabels for SuppliedMathMetrics {
    fn add_macros(&mut self, macros: &str) {
        self.fallback.add_macros(macros);
    }
    fn register_math_label(&mut self, id: &str, text: &str) {
        self.id_to_sentinel.insert(id.to_string(), text.to_string());
        // Also register with the fallback: if this turns out not to be a host
        // sentinel, the fallback will need to render it.
        self.fallback.register_math_label(id, text);
    }
    fn process_math_labels(&mut self) -> Result<(), String> {
        self.fallback.process_math_labels()
    }
    fn get_math_label(&self, id: &str) -> Option<MathLabel> {
        // Sentinels are drawn natively (via native_math); everything else falls
        // back to an SVG-drawing backend.
        if self.native_math(id).is_some() {
            return None;
        }
        self.fallback.get_math_label(id)
    }
    fn native_math(&self, id: &str) -> Option<(String, [f64; 3])> {
        let sentinel = self.id_to_sentinel.get(id)?;
        let dims = self.dims.get(sentinel)?;
        Some((sentinel.clone(), *dims))
    }
}

/// Placeholders used when a service is unavailable; labels needing it are
/// skipped with a log message, like Python without pycairo/louis installed.
pub struct NoTextMeasurements;

impl TextMeasurements for NoTextMeasurements {
    fn measure_text(&self, _text: &str, _font: &FontData) -> Option<[f64; 3]> {
        log::info!("no text measurement available; skipping a text label");
        None
    }
}

pub struct NoBrailleTranslator;

impl BrailleTranslator for NoBrailleTranslator {
    fn initialized(&self) -> bool {
        false
    }

    fn translate(&self, _text: &str, _typeform: &[u8]) -> Option<String> {
        None
    }
}

pub struct NoMathLabels;

impl MathLabels for NoMathLabels {
    fn add_macros(&mut self, _macros: &str) {}
    fn register_math_label(&mut self, _id: &str, _text: &str) {}
    fn process_math_labels(&mut self) -> Result<(), String> {
        Ok(())
    }
    fn get_math_label(&self, _id: &str) -> Option<MathLabel> {
        log::info!("no math rendering available; skipping a math label");
        None
    }
}

impl LabelState {
    /// A state with no label services at all (labels are skipped).
    pub fn none() -> LabelState {
        LabelState {
            math: Box::new(NoMathLabels),
            text: Box::new(NoTextMeasurements),
            braille: Box::new(NoBrailleTranslator),
            font_map: HashMap::new(),
            label_mode: LabelMode::Svg,
            placements: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// The native setup: MathJax via Node, like Python's LocalMathLabels,
    /// and cairo text measurement / liblouis braille when those features are on.
    #[cfg(all(feature = "xml-parse", not(target_arch = "wasm32")))]
    pub fn local(format: &str) -> LabelState {
        #[cfg(feature = "text-cairo")]
        let text: Box<dyn TextMeasurements> = Box::new(cairo::CairoTextMeasurements);
        #[cfg(not(feature = "text-cairo"))]
        let text: Box<dyn TextMeasurements> = Box::new(NoTextMeasurements);

        #[cfg(feature = "braille-liblouis")]
        let braille: Box<dyn BrailleTranslator> = Box::new(louis::LocalLouisBrailleTranslator);
        #[cfg(not(feature = "braille-liblouis"))]
        let braille: Box<dyn BrailleTranslator> = Box::new(NoBrailleTranslator);

        // Math backend, in priority order:
        //   `ratex`   -> pure-Rust RaTeX (no node, no JS engine; KaTeX-styled)
        //   otherwise -> shell out to node/MathJax, like Python's LocalMathLabels
        // `ratex` wins because it needs the least: no external node process.
        #[cfg(feature = "ratex")]
        let math: Box<dyn MathLabels> =
            Box::new(crate::core::ratex_math::RatexMathLabels::new(format));
        #[cfg(not(feature = "ratex"))]
        let math: Box<dyn MathLabels> = Box::new(LocalMathLabels::new(format));

        LabelState {
            math,
            text,
            braille,
            font_map: HashMap::new(),
            label_mode: LabelMode::Svg,
            placements: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Pass A of the Typst plugin protocol: a state that records the set of text
    /// runs (via `CollectingTextMeasurements`) and `<m>` bodies (via
    /// `CollectingMathLabels`) a build would produce, drawing nothing. Build with
    /// it, discard the SVG, then read the two sinks.
    pub fn collecting(
        text_sink: Rc<RefCell<Vec<(String, FontData)>>>,
        math_sink: Rc<RefCell<Vec<String>>>,
        font_map: HashMap<String, String>,
    ) -> LabelState {
        LabelState {
            math: Box::new(CollectingMathLabels { sink: math_sink }),
            text: Box::new(CollectingTextMeasurements {
                sink: text_sink,
                placeholder: [10.0, 8.0, 2.0],
            }),
            braille: Box::new(NoBrailleTranslator),
            font_map,
            label_mode: LabelMode::Svg,
            placements: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

/// Braille translation through the system liblouis, matching Python's `louis`
/// path (en-ueb-g2 grade-2 contracted braille). Untested in CI (liblouis is not
/// installed); enabled with the `braille-liblouis` feature.
#[cfg(feature = "braille-liblouis")]
mod louis {
    use super::BrailleTranslator;
    use std::ffi::CString;
    use std::os::raw::{c_char, c_int, c_void};

    // liblouis widechar is 16-bit unless it was built for 32-bit; the common
    // packaged build is 16-bit (UCS-2).
    type Widechar = u16;
    type Formtype = u16;

    #[link(name = "louis")]
    extern "C" {
        fn lou_translateString(
            table_list: *const c_char,
            inbuf: *const Widechar,
            inlen: *mut c_int,
            outbuf: *mut Widechar,
            outlen: *mut c_int,
            typeform: *mut Formtype,
            spacing: *mut c_char,
            mode: c_int,
        ) -> c_int;
        fn lou_free() -> c_void;
    }

    pub struct LocalLouisBrailleTranslator;

    impl BrailleTranslator for LocalLouisBrailleTranslator {
        fn initialized(&self) -> bool {
            true
        }

        fn translate(&self, text: &str, typeform: &[u8]) -> Option<String> {
            if text.is_empty() {
                return Some(String::new());
            }
            let table = CString::new("en-ueb-g2.ctb").ok()?;
            let input: Vec<Widechar> = text.encode_utf16().collect();
            let mut in_len = input.len() as c_int;
            let mut out: Vec<Widechar> = vec![0; input.len() * 4 + 16];
            let mut out_len = out.len() as c_int;
            // one typeform entry per UTF-16 code unit
            let mut forms: Vec<Formtype> = if typeform.iter().all(|&t| t == 0) {
                Vec::new()
            } else {
                let mut f = Vec::with_capacity(input.len());
                for (i, ch) in text.chars().enumerate() {
                    let units = ch.len_utf16();
                    let tf = typeform.get(i).copied().unwrap_or(0) as Formtype;
                    for _ in 0..units {
                        f.push(tf);
                    }
                }
                f
            };
            let forms_ptr = if forms.is_empty() {
                std::ptr::null_mut()
            } else {
                forms.as_mut_ptr()
            };

            let ok = unsafe {
                lou_translateString(
                    table.as_ptr(),
                    input.as_ptr(),
                    &mut in_len,
                    out.as_mut_ptr(),
                    &mut out_len,
                    forms_ptr,
                    std::ptr::null_mut(),
                    0,
                )
            };
            if ok == 0 {
                return None;
            }
            out.truncate(out_len.max(0) as usize);
            let result = String::from_utf16_lossy(&out);
            Some(result.trim_end().to_string())
        }
    }

    impl Drop for LocalLouisBrailleTranslator {
        fn drop(&mut self) {
            unsafe {
                lou_free();
            }
        }
    }
}

/// Text measurement through libcairo, matching Python's pycairo path so label
/// dimensions agree exactly.
#[cfg(feature = "text-cairo")]
mod cairo {
    use super::{FontData, TextMeasurements};
    use std::ffi::CString;
    use std::os::raw::{c_char, c_double, c_int, c_void};

    #[repr(C)]
    #[derive(Default)]
    struct CairoTextExtents {
        x_bearing: c_double,
        y_bearing: c_double,
        width: c_double,
        height: c_double,
        x_advance: c_double,
        y_advance: c_double,
    }

    #[link(name = "cairo")]
    extern "C" {
        fn cairo_svg_surface_create(
            filename: *const c_char,
            width_in_points: c_double,
            height_in_points: c_double,
        ) -> *mut c_void;
        fn cairo_create(surface: *mut c_void) -> *mut c_void;
        fn cairo_select_font_face(
            cr: *mut c_void,
            family: *const c_char,
            slant: c_int,
            weight: c_int,
        );
        fn cairo_set_font_size(cr: *mut c_void, size: c_double);
        fn cairo_text_extents(cr: *mut c_void, utf8: *const c_char, extents: *mut CairoTextExtents);
        fn cairo_destroy(cr: *mut c_void);
        fn cairo_surface_destroy(surface: *mut c_void);
    }

    pub struct CairoTextMeasurements;

    impl TextMeasurements for CairoTextMeasurements {
        fn measure_text(&self, text: &str, font: &FontData) -> Option<[f64; 3]> {
            let family = CString::new(font.family.as_str()).ok()?;
            let text_c = CString::new(text).ok()?;
            let slant = if font.italic { 1 } else { 0 };
            let weight = if font.bold { 1 } else { 0 };
            unsafe {
                let surface = cairo_svg_surface_create(std::ptr::null(), 200.0, 200.0);
                let cr = cairo_create(surface);
                cairo_select_font_face(cr, family.as_ptr(), slant, weight);
                cairo_set_font_size(cr, font.size);
                let mut extents = CairoTextExtents::default();
                cairo_text_extents(cr, text_c.as_ptr(), &mut extents);
                cairo_destroy(cr);
                cairo_surface_destroy(surface);
                // [advance, above baseline, below baseline], as in Python
                Some([
                    extents.x_advance,
                    -extents.y_bearing,
                    extents.height + extents.y_bearing,
                ])
            }
        }
    }
}

/// Locate the MathJax bundle (prefig/core/mj_sre). Checked in order:
/// the PREFIG_MJ_SRE environment variable, then prefig/core/mj_sre walking up
/// from the current directory, then relative to this crate in the repo.
#[cfg(all(feature = "xml-parse", not(target_arch = "wasm32")))]
fn find_mj_dir() -> Option<std::path::PathBuf> {
    if let Ok(dir) = std::env::var("PREFIG_MJ_SRE") {
        let path = std::path::PathBuf::from(dir);
        if path.join("mj-sre-page.js").exists() {
            return Some(path);
        }
    }
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let candidate = dir.join("prefig/core/mj_sre");
        if candidate.join("mj-sre-page.js").exists() {
            return Some(candidate);
        }
        if !dir.pop() {
            break;
        }
    }
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../prefig/core/mj_sre");
    if repo.join("mj-sre-page.js").exists() {
        return Some(repo);
    }
    None
}

/// Python's LocalMathLabels: batch all math through one Node/MathJax run.
#[cfg(all(feature = "xml-parse", not(target_arch = "wasm32")))]
pub struct LocalMathLabels {
    format: String,
    macros: Option<String>,
    registered: Vec<(String, String)>,
    label_tree: Option<El>,
}

#[cfg(all(feature = "xml-parse", not(target_arch = "wasm32")))]
impl LocalMathLabels {
    pub fn new(format: &str) -> LocalMathLabels {
        LocalMathLabels {
            format: format.to_string(),
            macros: None,
            registered: Vec::new(),
            label_tree: None,
        }
    }
}

#[cfg(all(feature = "xml-parse", not(target_arch = "wasm32")))]
impl MathLabels for LocalMathLabels {
    fn add_macros(&mut self, macros: &str) {
        self.macros = Some(macros.to_string());
    }

    fn register_math_label(&mut self, id: &str, text: &str) {
        self.registered.push((id.to_string(), text.to_string()));
    }

    fn process_math_labels(&mut self) -> Result<(), String> {
        if self.registered.is_empty() {
            return Ok(());
        }
        let mj_dir = find_mj_dir()
            .ok_or("MathJax bundle not found: set PREFIG_MJ_SRE or run `prefig init`")?;

        // assemble the HTML input file
        let html = xml::new_element("html");
        let body = xml::sub_element(&html, "body");
        if let Some(macros) = &self.macros {
            let div = xml::sub_element(&body, "div");
            div.borrow_mut().set("id", "latex-macros");
            div.borrow_mut().text = Some(format!("\\({macros}\\)"));
        }
        for (id, text) in &self.registered {
            let div = xml::sub_element(&body, "div");
            div.borrow_mut().set("id", id);
            div.borrow_mut().text = Some(format!("\\({text}\\)"));
        }

        let workdir = std::env::temp_dir().join(format!(
            "prefig-mj-{}-{:p}",
            std::process::id(),
            self as *const _
        ));
        std::fs::create_dir_all(&workdir).map_err(|e| e.to_string())?;
        let input = workdir.join("prefigure-labels.html");
        let output = workdir.join(format!("prefigure-{}.html", self.format));
        std::fs::write(&input, xml::to_pretty_string(&html)).map_err(|e| e.to_string())?;

        let mut command = std::process::Command::new("node");
        command.arg(mj_dir.join("mj-sre-page.js"));
        if self.format == "tactile" {
            command.arg("--braille");
        } else {
            command.args(["--svg", "--svgenhanced", "--depth", "deep"]);
        }
        command.arg(&input);
        let result = command.output().map_err(|e| format!("running node: {e}"))?;
        if !result.status.success() {
            return Err(format!(
                "MathJax failed: {}",
                String::from_utf8_lossy(&result.stderr)
            ));
        }
        std::fs::write(&output, &result.stdout).map_err(|e| e.to_string())?;

        let text = String::from_utf8_lossy(&result.stdout).into_owned();
        self.label_tree = Some(xml::parse_str(&text)?);
        let _ = std::fs::remove_dir_all(&workdir);
        Ok(())
    }

    fn get_math_label(&self, id: &str) -> Option<MathLabel> {
        let tree = self.label_tree.as_ref()?;
        let div = xml::find_descendants(tree, "div")
            .into_iter()
            .find(|d| d.borrow().get("id").as_deref() == Some(id))?;

        if self.format == "tactile" {
            let data = xml::find(&div, "mjx-data")?;
            let braille = xml::find(&data, "mjx-braille")?;
            let text = braille.borrow().text.clone()?;
            return Some(MathLabel::Braille(text));
        }

        let data = xml::find(&div, "mjx-data")?;
        let container = xml::find(&data, "mjx-container")?;
        let svg = xml::find(&container, "svg")?;
        Some(MathLabel::Svg(svg))
    }
}
