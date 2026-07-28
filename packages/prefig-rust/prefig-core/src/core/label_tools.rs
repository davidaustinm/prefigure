//! Port of prefig/core/label_tools.py: the seam between PreFigure and the
//! outside services it needs for labels — math rendering (MathJax), text
//! measurement, and braille translation. Native builds shell out to Node for
//! MathJax exactly like the Python version; the WebAssembly build implements
//! these traits over the browser's PrefigBrowserApi instead.

use crate::xml::{self, El};

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
        //   `ratex`      -> pure-Rust RaTeX (no node, no JS engine; KaTeX-styled)
        //   `mathjax-js` -> embedded JS engine running MathJax (no node)
        //   otherwise    -> shell out to node/MathJax, like Python's LocalMathLabels
        // `ratex` wins because it needs the least (no JS engine). Once a
        // wasm-capable JS engine can run MathJax directly (see mathjax_js.rs),
        // `mathjax-js` could take over for MathJax-identical output everywhere.
        #[cfg(feature = "ratex")]
        let math: Box<dyn MathLabels> = Box::new(crate::core::ratex_math::RatexMathLabels::new(format));
        #[cfg(all(feature = "mathjax-js", not(feature = "ratex")))]
        let math: Box<dyn MathLabels> = Box::new(crate::core::mathjax_js::JsMathLabels::new(format));
        #[cfg(not(any(feature = "mathjax-js", feature = "ratex")))]
        let math: Box<dyn MathLabels> = Box::new(LocalMathLabels::new(format));

        LabelState { math, text, braille }
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
        fn cairo_text_extents(
            cr: *mut c_void,
            utf8: *const c_char,
            extents: *mut CairoTextExtents,
        );
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
        let mj_dir =
            find_mj_dir().ok_or("MathJax bundle not found: set PREFIG_MJ_SRE or run `prefig init`")?;

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
