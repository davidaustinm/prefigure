//! Port of prefig/core/label.py: label registration during parsing and
//! placement after MathJax processing.

use crate::core::ctm::{self, CTM};
use crate::core::diagram::Diagram;
use crate::core::label_tools::{FontData, LabelMode, MathLabel, TextPlacement};
use crate::core::utilities::{self as util, float2str};
use crate::evaluator::ExpressionContext;
use crate::value::py_str;
use crate::xml::{self, El};

pub use crate::core::label_tools::LabelState;

pub const NEMETH_ON: &str = "⠸⠩ ";
pub const NEMETH_OFF: &str = "⠸⠱ ";
#[allow(dead_code)]
pub const GRADE1_INDICATOR: &str = "⠰";

const ALLOWED_FONTS: [&str; 3] = ["serif", "sans-serif", "monospace"];

/// Nominal size (px) at which native host-rendered math is measured and drawn,
/// matching PreFigure's default label `font-size`. Native math is placed at this
/// size regardless of the label's own `font-size` (a known limitation).
const MATH_LABEL_SIZE: f64 = 14.0;

pub fn is_label_tag(tag: &str) -> bool {
    matches!(tag, "it" | "b" | "newline")
}

/// Is there a label associated with this element?
pub fn has_label(element: &El) -> bool {
    let el = element.borrow();
    let has_text = el.text.as_ref().is_some_and(|t| !t.trim().is_empty());
    has_text || !el.children.is_empty()
}

pub fn alignment_displacement(alignment: &str) -> Option<[f64; 2]> {
    Some(match alignment {
        "southeast" | "se" => [0.0, 0.0],
        "east" | "e" => [0.0, 0.5],
        "northeast" | "ne" => [0.0, 1.0],
        "north" | "n" => [-0.5, 1.0],
        "northwest" | "nw" => [-1.0, 1.0],
        "west" | "w" => [-1.0, 0.5],
        "southwest" | "sw" => [-1.0, 0.0],
        "south" | "s" => [-0.5, 0.0],
        "center" | "c" => [-0.5, 0.5],
        "xaxis-label" | "ha" => [-0.5, 0.0],
        "va" => [-1.0, 0.5],
        "xl" => [-1.0, 1.0],
        _ => return None,
    })
}

/// Displacement of a braille label relative to its anchor (label.py).
pub fn braille_displacement(alignment: &str) -> Option<[f64; 2]> {
    Some(match alignment {
        "southeast" | "se" => [0.0, -1.0],
        "east" | "e" => [0.0, -0.5],
        "northeast" | "ne" => [0.0, 0.0],
        "north" | "n" => [-0.5, 0.0],
        "northwest" | "nw" => [-1.0, 0.0],
        "west" | "w" => [-1.0, -0.5],
        "southwest" | "sw" => [-1.0, -1.0],
        "south" | "s" => [-0.5, -1.0],
        "center" | "c" => [-0.5, -0.5],
        "xaxis-label" | "ha" => [0.0, -1.0],
        "hat" => [0.0, 0.0],
        "va" => [-1.0, -0.5],
        "var" => [0.0, -0.5],
        "xl" => [-1.0, 0.0],
        _ => return None,
    })
}

const ALIGNMENT_CIRCLE: [&str; 8] = [
    "east",
    "northeast",
    "north",
    "northwest",
    "west",
    "southwest",
    "south",
    "southeast",
];

pub fn get_alignment_from_direction(direction: [f64; 2]) -> String {
    let angle = direction[1].atan2(direction[0]).to_degrees();
    let align = ((angle / 45.0).round_ties_even() as i64).rem_euclid(8);
    ALIGNMENT_CIRCLE[align as usize].to_string()
}

/// Substitute ${...} expressions in label text from the author's namespace.
pub fn evaluate_text(text: &str, ctx: &mut ExpressionContext) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find('}') {
            Some(end) => {
                let expr = &after[..end];
                match ctx.valid_eval(expr) {
                    Ok(value) => out.push_str(&value.to_py_str()),
                    Err(_) => {
                        log::error!("Error in label evaluating {text}");
                        return String::new();
                    }
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push_str(&rest[start..]);
                return out;
            }
        }
    }
    out.push_str(rest);
    out
}

/// The <label> handler: register the label; placement happens after MathJax.
pub fn label(element: &El, diagram: &mut Diagram, parent: &El, _outline_group: Option<&El>) {
    let group = xml::new_element("g");
    diagram.add_label(element, &group);
    let id = element.borrow().get("id");
    diagram.add_id(element, id.as_deref());
    group
        .borrow_mut()
        .set("id", &element.borrow().get_or("id", "none"));
    diagram.register_svg_element(element, &group);

    if diagram.output_format() != "tactile" {
        xml::append(parent, &group);
    }

    // evaluate ${...} substitutions in all text under the label
    for child in xml::iter_subtree(element) {
        let text = child.borrow().text.clone();
        if let Some(text) = text {
            child.borrow_mut().text = Some(evaluate_text(&text, &mut diagram.ctx));
        }
        let tail = child.borrow().tail.clone();
        if let Some(tail) = tail {
            child.borrow_mut().tail = Some(evaluate_text(&tail, &mut diagram.ctx));
        }
    }

    // queue the <m> elements for MathJax
    for math in xml::find_all(element, "m") {
        diagram.add_id(&math, None);
        let math_id = math.borrow().get_or("id", "none");
        let text = math.borrow().text.clone().unwrap_or_default();
        diagram.labels.math.register_math_label(&math_id, &text);
    }

    let mut align = util::get_attr(element, "alignment", "c", &mut diagram.ctx);
    // 'e' evaluates to Euler's number; catch that here
    if align.starts_with('2') || align == "e" {
        align = "east".to_string();
    }
    element.borrow_mut().set("alignment", &align);
    let anchor = element.borrow().get("anchor");
    if let Some(anchor) = anchor {
        element.borrow_mut().set("p", &anchor);
    }
    let p = util::get_attr(element, "p", "[0,0]", &mut diagram.ctx);
    element.borrow_mut().set("p", &p);
}

pub fn place_labels(diagram: &mut Diagram) {
    if diagram.label_group_dict.is_empty() {
        return;
    }

    if let Err(e) = diagram.labels.math.process_math_labels() {
        log::error!("Production of mathematical labels failed: {e}");
    }

    // for braille output, a group holds all the labels and their clear
    // backgrounds, added at the end of the diagram
    let (background_group, braille_group) = if diagram.output_format() == "tactile" {
        if !diagram.labels.braille.initialized() {
            return;
        }
        let bg = xml::sub_element(&diagram.root, "g");
        bg.borrow_mut().set("id", "background-group");
        let br = xml::sub_element(&diagram.root, "g");
        br.borrow_mut().set("id", "braille-group");
        (Some(bg), Some(br))
    } else {
        (None, None)
    };

    let labels: Vec<(El, El, CTM)> = diagram.label_group_dict.clone();
    for (label, group, ctm) in labels {
        if diagram.output_format() == "tactile" {
            position_braille_label(
                &label,
                diagram,
                &ctm,
                background_group.as_ref().unwrap(),
                braille_group.as_ref().unwrap(),
            );
        } else {
            position_svg_label(&label, diagram, &ctm, &group);
        }
    }
}

enum RowItem {
    Text(String, FontData),
    Math(El),
}

/// A laid-out component: (element, width, above-baseline, below-baseline).
type Measured = (El, f64, f64, f64);

fn position_svg_label(element: &El, diagram: &mut Diagram, ctm: &CTM, group: &El) {
    let label_group = xml::new_element("g");

    // anchor point
    let p_attr = element.borrow().get_or("p", "[0,0]");
    let p_value = match diagram.ctx.valid_eval(&p_attr) {
        Ok(v) => v,
        Err(_) => {
            log::error!("Error in label parsing anchor={p_attr}");
            return;
        }
    };
    let p_user = match p_value.as_vec_f64() {
        Ok(v) if v.len() >= 2 => [v[0], v[1]],
        _ => {
            log::error!("Error in label parsing anchor={p_attr}");
            return;
        }
    };
    let p = if element.borrow().get_or("user-coords", "yes") == "yes" {
        ctm.transform(p_user)
    } else {
        p_user
    };

    let alignment = util::get_attr(element, "alignment", "center", &mut diagram.ctx);
    let Some(displacement) = alignment_displacement(&alignment) else {
        log::error!("Unknown alignment in label: {alignment}");
        return;
    };

    let offset_attr = util::get_attr(element, "abs-offset", "none", &mut diagram.ctx);
    let mut offset = if offset_attr == "none" {
        [
            8.0 * (displacement[0] + 0.5),
            8.0 * (displacement[1] - 0.5),
        ]
    } else {
        match diagram
            .ctx
            .valid_eval(&offset_attr)
            .ok()
            .and_then(|v| v.as_vec_f64().ok())
        {
            Some(v) if v.len() >= 2 => [v[0], v[1]],
            _ => {
                log::error!("Error in label parsing abs-offset={offset_attr}");
                return;
            }
        }
    };

    let relative = element.borrow().get("offset");
    if let Some(relative) = relative {
        match diagram
            .ctx
            .valid_eval(&relative)
            .ok()
            .and_then(|v| v.as_vec_f64().ok())
        {
            Some(v) if v.len() >= 2 => {
                offset = [offset[0] + v[0], offset[1] + v[1]];
            }
            _ => {
                log::error!("Error in label parsing offset={relative}");
                return;
            }
        }
    }

    let label_color = element.borrow().get("color");
    diagram.apply_defaults("label", element);

    let mut font_family = element
        .borrow()
        .get_or("font", "sans-serif")
        .to_lowercase()
        .trim()
        .to_string();
    if !ALLOWED_FONTS.contains(&font_family.as_str()) {
        font_family = "sans-serif".to_string();
    }
    // Map the generic family to a concrete one when a font_map is supplied (the
    // Typst plugin build; empty otherwise). Applied here so the family that gets
    // measured (`measure_text` below) is the same family written into the
    // `font-family` attribute in `mk_text_element` — Typst can only render a
    // family it can measure, and vice versa (§4.1 of the plugin plan).
    if let Some(concrete) = diagram.labels.font_map.get(&font_family) {
        font_family = concrete.clone();
    }
    let font_size_attr = element.borrow().get_or("font-size", "14");
    let font_size = diagram
        .ctx
        .valid_eval(&font_size_attr)
        .ok()
        .and_then(|v| v.as_num().ok())
        .unwrap_or(14.0);

    let face = |italic: bool, bold: bool, color: &Option<String>| FontData {
        family: font_family.clone(),
        size: font_size,
        italic,
        bold,
        color: color.clone(),
    };

    // gather rows of text pieces and <m> elements
    let mut rows: Vec<Vec<RowItem>> = Vec::new();
    let mut row: Vec<RowItem> = Vec::new();
    if let Some(text) = element.borrow().text.clone() {
        row.push(RowItem::Text(text, face(false, false, &label_color)));
    }
    rows.push(row);

    let children: Vec<El> = element.borrow().children.clone();
    for el in &children {
        let tag = el.borrow().tag.clone();
        let current = rows.last_mut().expect("rows is non-empty");
        match tag.as_str() {
            "newline" => {
                rows.push(Vec::new());
            }
            "m" => {
                let m_color = el.borrow().get("color").or_else(|| label_color.clone());
                if let Some(c) = &m_color {
                    el.borrow_mut().set("color", c);
                }
                rows.last_mut().unwrap().push(RowItem::Math(el.clone()));
            }
            "plain" => {
                let color = el.borrow().get("color").or_else(|| label_color.clone());
                if let Some(text) = el.borrow().text.clone() {
                    current.push(RowItem::Text(text, face(false, false, &color)));
                }
            }
            "it" | "b" => {
                let italic = tag == "it";
                let color = el.borrow().get("color").or_else(|| label_color.clone());
                if let Some(text) = el.borrow().text.clone() {
                    current.push(RowItem::Text(text, face(italic, !italic, &color)));
                }
                let grandchildren: Vec<El> = el.borrow().children.clone();
                for child in &grandchildren {
                    let child_tag = child.borrow().tag.clone();
                    let expected = if italic { "b" } else { "it" };
                    if child_tag != expected {
                        log::error!("<{child_tag}> is not allowed inside a <{tag}>");
                        continue;
                    }
                    let child_color = child.borrow().get("color").or_else(|| color.clone());
                    if let Some(text) = child.borrow().text.clone() {
                        current.push(RowItem::Text(text, face(true, true, &child_color)));
                    }
                    if let Some(tail) = child.borrow().tail.clone() {
                        current.push(RowItem::Text(tail, face(italic, !italic, &color)));
                    }
                }
            }
            _ => {}
        }
        if let Some(tail) = el.borrow().tail.clone() {
            rows.last_mut()
                .unwrap()
                .push(RowItem::Text(tail, face(false, false, &label_color)));
        }
    }

    // build and measure the components of each row
    let mut measured_rows: Vec<Vec<Measured>> = Vec::new();
    for row in rows {
        let mut out_row: Vec<Measured> = Vec::new();
        for item in row {
            match item {
                RowItem::Text(text, font) => {
                    let text = text.trim();
                    if text.is_empty() {
                        continue;
                    }
                    if let Some(measured) =
                        mk_text_element(text, &font, &label_group, diagram)
                    {
                        out_row.push(measured);
                    }
                }
                RowItem::Math(m_tag) => {
                    let m_id = m_tag.borrow().get_or("id", "none");
                    if let Some((sentinel, dims)) = diagram.labels.math.native_math(&m_id) {
                        // Native host-rendered math: a placeholder holds the
                        // sentinel and its supplied dimensions for layout; the
                        // native block below records a placement and drops it.
                        let ph = xml::sub_element(&label_group, "g");
                        ph.borrow_mut().set("data-pf-math", &sentinel);
                        if let Some(color) = m_tag.borrow().get("color") {
                            ph.borrow_mut().set("data-pf-color", &color);
                        }
                        out_row.push((ph, dims[0], dims[1], dims[2]));
                    } else if let Some(measured) = mk_m_element(&m_tag, diagram, &label_group) {
                        out_row.push(measured);
                    }
                }
            }
        }
        measured_rows.push(out_row);
    }

    // find the dimensions of each row and the whole label
    let space = 4.45; // width of a space, from pycairo
    let interline_attr = element.borrow().get_or("interline", "3");
    let mut interline = diagram
        .ctx
        .valid_eval(&interline_attr)
        .ok()
        .and_then(|v| v.as_num().ok())
        .unwrap_or(3.0);

    struct RowDims {
        width: f64,
        height_with_interline: f64,
        above: f64,
    }
    let mut dims: Vec<RowDims> = Vec::new();
    let row_count = measured_rows.len();
    for (num, row) in measured_rows.iter().enumerate() {
        let mut width = 0.0;
        let mut above: f64 = 0.0;
        let mut below: f64 = 0.0;
        for (_, w, a, b) in row {
            width += w;
            above = above.max(*a);
            below = below.max(*b);
        }
        let height = above + below;
        width += (row.len().saturating_sub(1)) as f64 * space;
        if num == row_count - 1 {
            interline = 0.0;
        }
        dims.push(RowDims {
            width,
            height_with_interline: height + interline,
            above,
        });
    }

    let width = dims.iter().map(|d| d.width).fold(0.0, f64::max);
    let height: f64 = dims.iter().map(|d| d.height_with_interline).sum();
    diagram.register_label_dims(element, (width, height));

    // position every component
    let justify = element.borrow().get_or("justify", "center");
    let mut y_location = 0.0;
    for (row, d) in measured_rows.iter().zip(&dims) {
        let mut x_location = match justify.as_str() {
            "center" => (width - d.width) / 2.0,
            "right" => width - d.width,
            _ => 0.0,
        };
        for (component, w, a, _) in row {
            component.borrow_mut().set("x", &float2str(x_location));
            x_location += w + space;
            let is_text = component.borrow().tag == "text";
            if is_text {
                component
                    .borrow_mut()
                    .set("y", &float2str(y_location + d.above));
            } else {
                component
                    .borrow_mut()
                    .set("y", &float2str(y_location + d.above - a));
            }
        }
        y_location += d.height_with_interline;
    }

    // the transform that places the group:
    //   translate(anchor) · scale(s) · rotate(a) · translate(displacement)
    let anchor = [p[0] + offset[0], p[1] - offset[1]];
    let scale: f64 = element
        .borrow()
        .get_or("scale", "1")
        .parse()
        .unwrap_or(1.0);
    let rotate_attr = element.borrow().get("rotate");
    // The angle used for native placement (0 when absent/unparseable); the SVG
    // transform below preserves the original emission exactly (including an
    // explicit rotate(0)), so `Svg` output is byte-identical to before.
    let angle: f64 = rotate_attr.as_ref().and_then(|r| r.parse::<f64>().ok()).unwrap_or(0.0);
    let disp = [width * displacement[0], -height * displacement[1]];

    let mut tform = ctm::translatestr(anchor[0], anchor[1]);
    if scale != 1.0 {
        tform = format!("{tform} {}", ctm::scalestr(scale, scale));
    }
    if let Some(rotate) = rotate_attr {
        if let Ok(a) = rotate.parse::<f64>() {
            tform = format!("{tform} {}", ctm::rotatestr(a));
        }
    }
    tform = format!("{tform} {}", ctm::translatestr(disp[0], disp[1]));
    group.borrow_mut().set("transform", &tform);

    // Native-label mode: hand each text run's absolute placement back to the
    // host and drop its <text> from the SVG, so the host renders it natively.
    // Math and geometry stay in the SVG. The absolute baseline point of a run at
    // local (cx, cy) is anchor + scale·R·(disp + (cx, cy)), where R is the SAME
    // rotation the group transform applies. That transform uses `rotatestr`,
    // which emits `rotate(-theta)` (PreFigure lays out in y-up math coordinates),
    // so the screen rotation is by -angle, not +angle. Using +angle here would
    // mirror rotated labels relative to the baked geometry/math — see native.typ.
    if diagram.labels.label_mode == LabelMode::Native {
        let rad = (-angle).to_radians();
        let (cos, sin) = (rad.cos(), rad.sin());
        let placements = diagram.labels.placements.clone();
        // A caller may have wrapped this label's `<g>` in an extra transform
        // (e.g. a <line> label inside `translate·rotate`). The runs below are
        // computed in that wrapper's local frame, so compose it into each run's
        // absolute placement — SVG applies it at render, native runs are lifted
        // out. `wrapper_angle` is the wrapper's screen rotation, subtracted from
        // each run's angle (native.typ rotates by `rotate(-angle)`).
        let wrapper = diagram.native_wrapper_for(element);
        let wrapper_angle = wrapper
            .map(|w| w[1][0].atan2(w[0][0]).to_degrees())
            .unwrap_or(0.0);
        for row in &measured_rows {
            for (component, _w, above, _b) in row {
                let el = component.borrow();
                let is_text = el.tag == "text";
                let sentinel = el.get("data-pf-math");
                if !is_text && sentinel.is_none() {
                    continue; // a real math SVG (mk_m_element) — leave it in place
                }
                let cx: f64 = el.get("x").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                let cy: f64 = el.get("y").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                // For text, cy is already the baseline; for a math placeholder cy
                // is its top, so the baseline is cy + above.
                let baseline_local = if is_text { cy } else { cy + above };
                let vx = disp[0] + cx;
                let vy = disp[1] + baseline_local;
                let mut placement = if is_text {
                    TextPlacement {
                        text: el.text.clone().unwrap_or_default(),
                        family: el.get_or("font-family", "sans-serif"),
                        size: el.get("font-size").and_then(|v| v.parse().ok()).unwrap_or(14.0),
                        italic: el.get("font-style").as_deref() == Some("italic"),
                        bold: el.get("font-weight").as_deref() == Some("bold"),
                        color: el.get("fill"),
                        x: anchor[0] + scale * (cos * vx - sin * vy),
                        y: anchor[1] + scale * (sin * vx + cos * vy),
                        angle,
                        scale,
                        math: false,
                    }
                } else {
                    TextPlacement {
                        text: sentinel.unwrap(),
                        family: String::new(),
                        size: MATH_LABEL_SIZE,
                        italic: false,
                        bold: false,
                        color: el.get("data-pf-color"),
                        x: anchor[0] + scale * (cos * vx - sin * vy),
                        y: anchor[1] + scale * (sin * vx + cos * vy),
                        angle,
                        scale,
                        math: true,
                    }
                };
                drop(el);
                // Compose any wrapper transform: the (x, y) above are in the
                // wrapper's local frame; map them to absolute and fold the
                // wrapper's rotation into the run's angle.
                if let Some(w) = wrapper {
                    let (x, y) = (placement.x, placement.y);
                    placement.x = w[0][0] * x + w[0][1] * y + w[0][2];
                    placement.y = w[1][0] * x + w[1][1] * y + w[1][2];
                    placement.angle -= wrapper_angle;
                }
                // Record the run's index and its baseline in the label group's
                // own coordinates. A <legend> re-anchors these after layout
                // (place_legend); for a standalone label they are already final.
                let index = {
                    let mut ps = placements.borrow_mut();
                    ps.push(placement);
                    ps.len() - 1
                };
                diagram.record_native_run(element, index, [cx, baseline_local]);
                xml::remove(&label_group, component);
            }
        }
    }

    // a white rectangle behind the label, if requested
    if element.borrow().get_or("clear-background", "no") == "yes" {
        let bg_margin: i64 = element
            .borrow()
            .get_or("background-margin", "6")
            .parse()
            .unwrap_or(6);
        let rect = xml::sub_element(group, "rect");
        let mut r = rect.borrow_mut();
        r.set("x", &format!("{}", -bg_margin));
        r.set("y", &format!("{}", -bg_margin));
        r.set("width", &py_str(width + 2.0 * bg_margin as f64));
        r.set("height", &py_str(height + 2.0 * bg_margin as f64));
        r.set("stroke", "none");
        r.set("fill", "white");
    }

    xml::append(group, &label_group);
    let expr_id = element.borrow().get("expr");
    diagram.add_id(&label_group, expr_id.as_deref());
}

pub fn snap_to_embossing_grid(x: f64) -> f64 {
    3.6 * (x / 3.6).round_ties_even()
}

/// One item of a braille row: a run of text with a typeform, or a math label.
enum BrailleItem {
    Text(String, u8), // (text, typeform: plain=0, it=1, b=4)
    Math(El),
}

/// Port of label.py position_braille_label: lay a label out in braille cells,
/// translate the text, and snap it to the 20-dpi embossing grid.
fn position_braille_label(
    element: &El,
    diagram: &mut Diagram,
    ctm: &CTM,
    background_group: &El,
    braille_group: &El,
) {
    let group = xml::sub_element(braille_group, "g");
    group
        .borrow_mut()
        .set("id", &element.borrow().get_or("id", "none"));

    // anchor, adjusted by alignment and offset
    let p_attr = element.borrow().get_or("p", "[0,0]");
    let Some(p_user) = diagram
        .ctx
        .valid_eval(&p_attr)
        .ok()
        .and_then(|v| v.as_vec_f64().ok())
        .filter(|v| v.len() >= 2)
    else {
        log::error!("Error in label parsing anchor={p_attr}");
        return;
    };
    let mut p = if element.borrow().get_or("user-coords", "yes") == "yes" {
        ctm.transform([p_user[0], p_user[1]])
    } else {
        [p_user[0], p_user[1]]
    };

    let alignment = util::get_attr(element, "alignment", "center", &mut diagram.ctx);
    let Some(displacement) = braille_displacement(&alignment) else {
        log::error!("Unknown alignment in label: {alignment}");
        return;
    };

    let offset_attr = util::get_attr(element, "abs-offset", "none", &mut diagram.ctx);
    let mut offset = if offset_attr == "none" {
        [
            8.0 * (displacement[0] + 0.5),
            8.0 * (displacement[1] + 0.5),
        ]
    } else {
        match diagram
            .ctx
            .valid_eval(&offset_attr)
            .ok()
            .and_then(|v| v.as_vec_f64().ok())
        {
            Some(v) if v.len() >= 2 => [v[0], v[1]],
            _ => {
                log::error!("Error in label parsing abs-offset");
                return;
            }
        }
    };

    offset = [
        offset[0] + 6.0 * offset[0].signum(),
        offset[1] + 6.0 * offset[1].signum(),
    ];
    if displacement[0] == 0.0 {
        offset[0] += 6.0;
    }
    if displacement[1] == -1.0 {
        offset[1] -= 6.0;
    }
    if alignment == "n" || alignment == "north" {
        offset[1] += 5.0;
    }

    let gap = 3.6;
    match alignment.as_str() {
        "ha" => offset = [-4.0 * gap, -30.0],
        "hat" => offset = [-4.0 * gap, 30.0],
        "va" => offset = [-9.0, 0.0],
        "var" => offset = [30.0, 0.0],
        "xl" => offset = [-10.0, 12.0],
        _ => {}
    }

    if let Some(rel) = element.borrow().get("offset") {
        if let Some(v) = diagram
            .ctx
            .valid_eval(&rel)
            .ok()
            .and_then(|v| v.as_vec_f64().ok())
        {
            offset = [offset[0] + v[0], offset[1] + v[1]];
        }
    }
    p[0] += offset[0];
    p[1] -= offset[1];

    // assemble the rows of items (mirrors the Python row-building)
    let mut rows: Vec<Vec<BrailleItem>> = vec![vec![BrailleItem::Text(
        element.borrow().text.clone().unwrap_or_default(),
        0,
    )]];
    let children: Vec<El> = element.borrow().children.clone();
    for el in &children {
        let tag = el.borrow().tag.clone();
        match tag.as_str() {
            "newline" => rows.push(Vec::new()),
            "m" => rows.last_mut().unwrap().push(BrailleItem::Math(el.clone())),
            "it" | "b" => {
                let tf = if tag == "it" { 1 } else { 4 };
                let text = el.borrow().text.clone().unwrap_or_default();
                rows.last_mut().unwrap().push(BrailleItem::Text(text, tf));
            }
            "plain" => {
                let text = el.borrow().text.clone().unwrap_or_default();
                rows.last_mut().unwrap().push(BrailleItem::Text(text, 0));
            }
            _ => {}
        }
        if let Some(tail) = el.borrow().tail.clone() {
            rows.last_mut().unwrap().push(BrailleItem::Text(tail, 0));
        }
    }

    // translate each row to a braille string
    let space = " ";
    let mut row_texts: Vec<String> = Vec::new();
    for row in &rows {
        let mut row_text = String::new();
        let mut needs_grade1 = false;
        for item in row {
            match item {
                BrailleItem::Text(text, tf) => {
                    let text = text.trim();
                    if text.is_empty() {
                        continue;
                    }
                    let mut t = text.to_string();
                    if !row_text.is_empty() {
                        t.push(' ');
                    }
                    let typeform = vec![*tf; t.chars().count()];
                    if let Some(braille) = diagram.labels.braille.translate(&t, &typeform) {
                        row_text.push_str(&braille);
                    }
                }
                BrailleItem::Math(m) => {
                    let m_id = m.borrow().get_or("id", "none");
                    let m_text = m.borrow().text.clone().unwrap_or_default();
                    let trimmed = m_text.trim();
                    if trimmed.chars().count() == 1 {
                        let ch = trimmed.chars().next().unwrap();
                        if ch.is_ascii_lowercase() {
                            needs_grade1 = true;
                        }
                    }
                    if let Some(MathLabel::Braille(text)) =
                        diagram.labels.math.get_math_label(&m_id)
                    {
                        if !row_text.is_empty() {
                            row_text.push_str(space);
                        }
                        row_text.push_str(&text);
                    }
                }
            }
        }
        if row_text.chars().count() == 1 && needs_grade1 {
            row_text = format!("{GRADE1_INDICATOR}{row_text}");
        }
        row_texts.push(row_text);
    }

    let interline = 28.8;
    let width = 5.0
        * gap
        * row_texts
            .iter()
            .map(|r| r.chars().count())
            .max()
            .unwrap_or(0) as f64;
    let height = 5.0 * gap + interline * (row_texts.len().saturating_sub(1)) as f64;
    diagram.register_label_dims(element, (width, height));

    p[0] += width * displacement[0];
    p[1] -= height * displacement[1];
    // snap onto the 20dpi embossing grid
    p = [snap_to_embossing_grid(p[0]), snap_to_embossing_grid(p[1])];

    group
        .borrow_mut()
        .set("transform", &ctm::translatestr(p[0], p[1]));

    // white background behind the label
    let bg_margin = 9.0;
    let rect = xml::sub_element(background_group, "rect");
    {
        let mut r = rect.borrow_mut();
        r.set("id", &format!("{}-background", element.borrow().get_or("id", "none")));
        r.set("x", &float2str(p[0] - bg_margin));
        r.set("y", &float2str(p[1] - height - bg_margin));
        r.set("width", &float2str(width + 2.0 * bg_margin));
        r.set("height", &float2str(height + 2.0 * bg_margin));
        r.set("stroke", "none");
        r.set("fill", "white");
    }

    // the braille text rows
    let justify = element.borrow().get_or("justify", "center");
    let x = 0.0;
    let mut y = -height + 5.0 * gap;
    for row_text in &row_texts {
        let text_element = xml::sub_element(&group, "text");
        let len = row_text.chars().count() as f64;
        let x_line = match justify.as_str() {
            "right" => x + width - 5.0 * gap * len,
            "center" => {
                let raw = x + (width - 5.0 * gap * len) / 2.0;
                gap * (raw / gap).round_ties_even()
            }
            _ => x,
        };
        let mut t = text_element.borrow_mut();
        t.set("x", &float2str(x_line));
        t.set("y", &float2str(y));
        t.text = Some(row_text.clone());
        t.set("font-family", "Braille29");
        t.set("font-size", "29px");
        y += interline;
    }
}

fn mk_text_element(
    text: &str,
    font: &FontData,
    label_group: &El,
    diagram: &mut Diagram,
) -> Option<Measured> {
    let text_el = xml::sub_element(label_group, "text");
    {
        let mut t = text_el.borrow_mut();
        t.text = Some(text.to_string());
        t.set("font-family", &font.family);
        t.set("font-size", &py_str(font.size));
        if font.italic {
            t.set("font-style", "italic");
        }
        if font.bold {
            t.set("font-weight", "bold");
        }
        if let Some(color) = &font.color {
            t.set("fill", color);
        }
    }

    let measurements = diagram.labels.text.measure_text(text, font)?;
    Some((text_el, measurements[0], measurements[1], measurements[2]))
}

fn mk_m_element(m_tag: &El, diagram: &mut Diagram, label_group: &El) -> Option<Measured> {
    let m_tag_id = m_tag.borrow().get_or("id", "none");
    let Some(MathLabel::Svg(insert)) = diagram.labels.math.get_math_label(&m_tag_id) else {
        return None;
    };

    // prefix glyph ids so multiple diagrams can coexist on a page
    let mut defs_map: Vec<(String, String)> = Vec::new();
    if let Some(defs) = xml::find(&insert, "defs") {
        let glyphs: Vec<El> = defs.borrow().children.clone();
        for glyph in &glyphs {
            let id = glyph.borrow().get("id");
            if let Some(id) = id {
                let new_id = diagram.prepend_id_prefix(&id);
                glyph.borrow_mut().set("id", &new_id);
                defs_map.push((id, new_id));
            }
        }
    }
    for use_el in xml::find_descendants(&insert, "use") {
        let xlink_href = use_el.borrow().get("xlink:href");
        let href = match xlink_href {
            Some(h) => Some(h),
            None => use_el.borrow().get("href"),
        };
        if let Some(href) = href {
            let target = href.strip_prefix('#').unwrap_or(&href);
            if let Some((_, new_id)) = defs_map.iter().find(|(old, _)| old == target) {
                let key = if use_el.borrow().get("xlink:href").is_some() {
                    "xlink:href"
                } else {
                    "href"
                };
                use_el.borrow_mut().set(key, &format!("#{new_id}"));
            }
        }
    }

    // convert dimensions from ex to px for rsvg-convert
    let dim = |attr: &str| -> Option<f64> {
        let value = insert.borrow().get(attr)?;
        let mut fields: Vec<String> = value.split_whitespace().map(str::to_string).collect();
        let last = fields.last()?.clone();
        // Python does dimension[:dimension.find('ex')]; find returning -1
        // slices off the final character — e.g. "0;" becomes "0". Keep that.
        let index = last.find("ex").unwrap_or(last.len().saturating_sub(1));
        let dimension: f64 = last[..index].parse().ok()?;
        let dimension = dimension * 8.0;
        *fields.last_mut()? = format!("{dimension:.3}px");
        insert.borrow_mut().set(attr, &fields.join(" "));
        Some(dimension)
    };
    let style = dim("style")?;
    let width = dim("width")?;
    let height = dim("height")?;

    xml::append(label_group, &insert);

    let above = height + style;
    let below = -style;

    let color = m_tag.borrow().get("color");
    if let Some(color) = color {
        let style_attr = insert.borrow().get_or("style", "");
        let style_attr = style_attr.trim_end();
        let new_style = if style_attr.ends_with(';') {
            format!("{style_attr} color:{color}")
        } else {
            format!("{style_attr}; color:{color}")
        };
        insert.borrow_mut().set("style", &new_style);
    }

    Some((insert, width, above, below))
}

/// The <caption> handler (captions appear on tactile output).
pub fn caption(element: &El, diagram: &mut Diagram, _parent: &El, _outline_group: Option<&El>) {
    if diagram.caption_suppressed() {
        return;
    }
    let text = element.borrow().text.clone().unwrap_or_default();
    if text.is_empty() {
        return;
    }
    diagram.set_caption(text.trim());
}
