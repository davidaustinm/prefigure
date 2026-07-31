//! Shared helpers for the integration tests: fixed stub label services (so a
//! test needs neither Node/MathJax, cairo, nor liblouis) and corpus walking.

use prefig_core::core::label_tools::{
    BrailleTranslator, FontData, LabelState, MathLabel, MathLabels, TextMeasurements,
};
use prefig_core::xml;
use std::path::{Path, PathBuf};

struct StubMath;
impl MathLabels for StubMath {
    fn add_macros(&mut self, _macros: &str) {}
    fn register_math_label(&mut self, _id: &str, _text: &str) {}
    fn process_math_labels(&mut self) -> Result<(), String> {
        Ok(())
    }
    fn get_math_label(&self, _id: &str) -> Option<MathLabel> {
        // A well-formed MathJax-like placeholder: `ex`-unit width/height/style
        // and a <defs>, so the real label-insertion path runs (ex->px, glyph
        // id prefixing) instead of being skipped.
        let svg = xml::parse_str(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="1.5ex" height="1.5ex" viewBox="0 -1 1.5 1.5" style="vertical-align: -0.25ex"><defs></defs><g></g></svg>"#,
        )
        .ok()?;
        Some(MathLabel::Svg(svg))
    }
}

struct StubText;
impl TextMeasurements for StubText {
    fn measure_text(&self, text: &str, font: &FontData) -> Option<[f64; 3]> {
        let w = text.chars().count() as f64 * font.size * 0.5;
        Some([w, font.size * 0.75, font.size * 0.25])
    }
}

/// Translates each character to a single Braille glyph, so tactile output is
/// deterministic and recognisably Braille without a real liblouis backend.
struct StubBraille;
impl BrailleTranslator for StubBraille {
    fn initialized(&self) -> bool {
        true
    }
    fn translate(&self, text: &str, _typeform: &[u8]) -> Option<String> {
        Some(text.chars().map(|_| '\u{283F}').collect())
    }
}

/// A `LabelState` wired to the fixed stubs above (no external services).
pub fn stub_labels() -> LabelState {
    LabelState {
        math: Box::new(StubMath),
        text: Box::new(StubText),
        braille: Box::new(StubBraille),
        font_map: Default::default(),
        label_mode: Default::default(),
        placements: Default::default(),
    }
}

/// Recursively collect every `*.xml` file under `dir` into `out`.
// Shared by several test binaries; not every one uses it (this module is compiled
// separately into each), so allow it to be unused in some.
#[allow(dead_code)]
pub fn collect_xml(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_xml(&path, out);
        } else if path.extension().is_some_and(|x| x == "xml") {
            out.push(path);
        }
    }
}
