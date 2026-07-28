//! Port of `Diagram.svg11_conversion` (prefig/core/diagram.py): rewrite a
//! finished SVG 2 output tree into SVG 1.1, for the PreTeXt/EPUB toolchain
//! whose renderers lack SVG 2's `auto-start-reverse` markers and bare `href`
//! on `<use>`/`<image>`.
//!
//! The tree is built exactly as for the `"svg"` format; this pass runs only at
//! serialization time when the output format is `"svg11"` (see
//! `Diagram::end_figure_to_string`). It mutates the element tree in place, so
//! callers hand it a throwaway deep copy of the document root.

use crate::xml::{self, deep_copy, El};

const XLINK_NS: &str = "http://www.w3.org/1999/xlink";

/// Convert `root` (an `<svg>` element tree) in place from SVG 2 to SVG 1.1.
pub fn convert(root: &El) {
    href_to_xlink(root);
    make_start_markers(root);
    retarget_marker_start(root);
    demote_remaining_auto_start_reverse(root);
}

/// `href` -> `xlink:href` on every `<use>`/`<image>`. If anything was rewritten
/// and the root does not already carry the xlink namespace, declare it there so
/// the prefix resolves. (Python relies on lxml to declare the namespace; here
/// we make it explicit on the root, which is valid SVG 1.1.)
fn href_to_xlink(root: &El) {
    let mut rewrote = false;
    for el in xml::iter_subtree(root) {
        let tag = el.borrow().tag.clone();
        if tag != "use" && tag != "image" {
            continue;
        }
        let href = el.borrow_mut().pop_attr("href");
        if let Some(value) = href {
            el.borrow_mut().set("xlink:href", &value);
            rewrote = true;
        }
    }
    if rewrote && root.borrow().get("xmlns:xlink").is_none() {
        root.borrow_mut().set("xmlns:xlink", XLINK_NS);
    }
}

/// For every `arrow-head-end` marker: drop it to `orient="auto"` and append a
/// matching `arrow-head-start` marker whose glyph is rotated 180° about the
/// marker centre, with the reference point mirrored (`refX = markerWidth - refX`).
/// This reproduces, in SVG 1.1, what `auto-start-reverse` did for a backward
/// arrowhead referenced by `marker-start`.
fn make_start_markers(root: &El) {
    for defs in xml::find_descendants(root, "defs") {
        // snapshot the children first, like Python's `for marker in list(defs)`,
        // so the markers we append are not themselves revisited
        let children = defs.borrow().children.clone();
        let mut starts: Vec<El> = Vec::new();
        for marker in &children {
            if marker.borrow().tag != "marker" {
                continue;
            }
            let id = marker.borrow().get_or("id", "");
            if !id.contains("arrow-head-end") {
                continue;
            }
            marker.borrow_mut().set("orient", "auto");

            let start = deep_copy(marker);
            start
                .borrow_mut()
                .set("id", &id.replace("arrow-head-end", "arrow-head-start"));

            let ref_x = num_attr(&start, "refX");
            let marker_width = num_attr(&start, "markerWidth");
            let marker_height = num_attr(&start, "markerHeight");

            // wrap the marker's glyph in a 180°-rotated group about the centre
            let g = xml::new_element("g");
            g.borrow_mut().set(
                "transform",
                &format!(
                    "rotate(180, {}, {})",
                    coord(marker_width / 2.0),
                    coord(marker_height / 2.0)
                ),
            );
            let glyph = start.borrow().children.clone();
            start.borrow_mut().children.clear();
            for child in glyph {
                xml::append(&g, &child);
            }
            xml::append(&start, &g);

            start.borrow_mut().set("refX", &refx(marker_width - ref_x));
            starts.push(start);
        }
        for start in starts {
            xml::append(&defs, &start);
        }
    }
}

/// Point `marker-start` references at the new `arrow-head-start` markers.
fn retarget_marker_start(root: &El) {
    for el in xml::iter_subtree(root) {
        let value = el.borrow().get("marker-start");
        if let Some(value) = value {
            if value.contains("arrow-head-end") {
                el.borrow_mut().set(
                    "marker-start",
                    &value.replace("arrow-head-end", "arrow-head-start"),
                );
            }
        }
    }
}

/// Any marker still carrying SVG 2's `auto-start-reverse` (e.g. the mid-path
/// arrowheads, referenced only by `marker-mid` where the two orient values
/// render identically) falls back to plain `auto`, which SVG 1.1 understands.
fn demote_remaining_auto_start_reverse(root: &El) {
    for marker in xml::find_descendants(root, "marker") {
        if marker.borrow().get_or("orient", "") == "auto-start-reverse" {
            marker.borrow_mut().set("orient", "auto");
        }
    }
}

fn num_attr(el: &El, name: &str) -> f64 {
    el.borrow().get_or(name, "0").parse().unwrap_or(0.0)
}

/// Mirror Python's f-string float formatting for a coordinate: integral values
/// print with a trailing `.0` (`str(4.0) == "4.0"`), others print plainly.
/// (The parity comparator compares these numerically, so this only needs to be
/// close; matching Python keeps the raw output identical too.)
fn coord(v: f64) -> String {
    if v == v.trunc() {
        format!("{v:.1}")
    } else {
        format!("{v}")
    }
}

/// `refX` formatting, matching Python: an int when the value is integral,
/// otherwise the float (`str(int(x))` vs `str(x)`).
fn refx(v: f64) -> String {
    if v == v.trunc() {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xml::{append, new_element};

    fn attr(el: &El, name: &str) -> Option<String> {
        el.borrow().get(name)
    }

    #[test]
    fn converts_a_backward_arrow_document() {
        // <svg>
        //   <defs><marker id="p-arrow-head-end-k" ... orient="auto-start-reverse"
        //                 markerWidth=9 markerHeight=8 refX=6.5 refY=4>
        //           <path d="..."/></marker></defs>
        //   <use href="#glyph"/>
        //   <path marker-start="url(#p-arrow-head-end-k)"/>
        //   <marker orient="auto-start-reverse"/>   (stray mid-path marker)
        // </svg>
        let svg = new_element("svg");
        let defs = new_element("defs");
        let end = new_element("marker");
        {
            let mut m = end.borrow_mut();
            m.set("id", "p-arrow-head-end-k");
            m.set("orient", "auto-start-reverse");
            m.set("markerWidth", "9.0");
            m.set("markerHeight", "8.0");
            m.set("refX", "6.5");
            m.set("refY", "4.0");
        }
        let glyph = new_element("path");
        glyph.borrow_mut().set("d", "M 9 4 L 0 8 Z");
        append(&end, &glyph);
        append(&defs, &end);
        append(&svg, &defs);

        let use_el = new_element("use");
        use_el.borrow_mut().set("href", "#glyph");
        append(&svg, &use_el);

        let path = new_element("path");
        path.borrow_mut()
            .set("marker-start", "url(#p-arrow-head-end-k)");
        append(&svg, &path);

        let stray = new_element("marker");
        stray.borrow_mut().set("orient", "auto-start-reverse");
        append(&svg, &stray);

        convert(&svg);

        // href -> xlink:href, xmlns declared on root
        assert_eq!(attr(&use_el, "href"), None);
        assert_eq!(attr(&use_el, "xlink:href").as_deref(), Some("#glyph"));
        assert_eq!(attr(&svg, "xmlns:xlink").as_deref(), Some(XLINK_NS));

        // the end marker falls back to plain auto
        assert_eq!(attr(&end, "orient").as_deref(), Some("auto"));

        // a matching start marker was appended, refX mirrored (9 - 6.5 = 2.5)
        let start = defs
            .borrow()
            .children
            .iter()
            .find(|c| attr(c, "id").as_deref() == Some("p-arrow-head-start-k"))
            .cloned()
            .expect("arrow-head-start marker created");
        assert_eq!(attr(&start, "orient").as_deref(), Some("auto"));
        assert_eq!(attr(&start, "refX").as_deref(), Some("2.5"));

        // its glyph is wrapped in a 180° rotation about the marker centre
        let g = start.borrow().children[0].clone();
        assert_eq!(g.borrow().tag, "g");
        assert_eq!(
            attr(&g, "transform").as_deref(),
            Some("rotate(180, 4.5, 4.0)")
        );
        assert_eq!(g.borrow().children[0].borrow().tag, "path");

        // the marker-start reference is retargeted, stray marker demoted
        assert_eq!(
            attr(&path, "marker-start").as_deref(),
            Some("url(#p-arrow-head-start-k)")
        );
        assert_eq!(attr(&stray, "orient").as_deref(), Some("auto"));
    }
}
