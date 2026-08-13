//! Port of prefig/core/point.py.

use crate::core::diagram::Diagram;
use crate::core::label;
use crate::core::utilities::{self as util, float2str, pt2long_str, pt2str};
use crate::value::py_str;
use crate::xml::{self, El};

pub fn point(element: &El, diagram: &mut Diagram, parent: &El, outline_group: Option<&El>) {
    let p_attr = element.borrow().get("p");
    // A caller (e.g. <poset>) may have precomputed the point's location and
    // stashed it as source data; that wins over the @p attribute.
    let p_source = diagram.get_source_data(element, "p");
    let p_vec = match p_source {
        Some(v) => v.as_vec_f64().ok(),
        None => p_attr
            .as_deref()
            .and_then(|attr| diagram.ctx.valid_eval(attr).ok())
            .and_then(|v| v.as_vec_f64().ok()),
    };
    let p = match p_vec {
        Some(v) if v.len() >= 2 => {
            let mut p = [v[0], v[1]];
            if element.borrow().get_or("coordinates", "cartesian") == "polar" {
                let radial = p[0];
                let mut angle = p[1];
                if element.borrow().get_or("degrees", "no") == "yes" {
                    angle = angle.to_radians();
                }
                p = [radial * angle.cos(), radial * angle.sin()];
                element.borrow_mut().set("p", &pt2long_str(p, ","));
            }
            diagram.transform(p)
        }
        _ => {
            log::error!("Error in <point> defining p={p_attr:?}");
            return;
        }
    };

    if diagram.output_format() == "tactile" {
        let size_attr = element.borrow().get("size");
        let size = match size_attr {
            Some(attr) => diagram
                .ctx
                .valid_eval(&attr)
                .ok()
                .and_then(|v| v.as_num().ok())
                .map(|s| s.max(9.0))
                .unwrap_or(9.0),
            None => 9.0,
        };
        element.borrow_mut().set("size", &py_str(size));
    } else {
        let size = element.borrow().get_or("size", "4");
        element.borrow_mut().set("size", &size);
    }
    let size_str = util::get_attr(element, "size", "1", &mut diagram.ctx);
    let raw_size_str = element.borrow().get_or("size", "1");

    let shape = xml::new_element("circle");
    let has_label = label::has_label(element);
    let mut parent = parent.clone();
    if has_label {
        let group = xml::sub_element(&parent, "g");
        let id = element.borrow().get("id");
        diagram.add_id(&group, id.as_deref());
        diagram.register_svg_element(element, &group);
        parent = group;
        // The label itself is added later — before the shape for the usual
        // alignments (so a `clear-background` box sits behind the point, see
        // issue #70), but after the shape for centered ("c…") alignments so the
        // label sits on top of the point.
    } else {
        let id = element.borrow().get("id");
        diagram.add_id(&shape, id.as_deref());
        diagram.register_svg_element(element, &shape);
    }

    let style = util::get_attr(element, "style", "circle", &mut diagram.ctx);
    if style == "circle" {
        let mut s = shape.borrow_mut();
        s.set("cx", &float2str(p[0]));
        s.set("cy", &float2str(p[1]));
        s.set("r", &size_str);
    }
    let mut size: f64 = size_str.parse().unwrap_or(1.0);
    match style.as_str() {
        "box" => {
            let mut s = shape.borrow_mut();
            s.tag = "rect".to_string();
            s.set("x", &float2str(p[0] - size));
            s.set("y", &float2str(p[1] - size));
            s.set("width", &float2str(2.0 * size));
            s.set("height", &float2str(2.0 * size));
        }
        "diamond" => {
            size *= 1.4;
            let mut s = shape.borrow_mut();
            s.tag = "polygon".to_string();
            let points = format!(
                "{} {} {} {}",
                pt2str([p[0], p[1] - size], ","),
                pt2str([p[0] + size, p[1]], ","),
                pt2str([p[0], p[1] + size], ","),
                pt2str([p[0] - size, p[1]], ",")
            );
            s.set("points", &points);
        }
        "cross" => {
            size *= 1.4;
            let mut s = shape.borrow_mut();
            s.tag = "path".to_string();
            let d = format!(
                "M {}L {}M {}L {}",
                pt2str([p[0] - size, p[1] + size], " "),
                pt2str([p[0] + size, p[1] - size], " "),
                pt2str([p[0] + size, p[1] + size], " "),
                pt2str([p[0] - size, p[1] - size], " ")
            );
            s.set("d", &d);
        }
        "plus" => {
            size *= 1.4;
            let mut s = shape.borrow_mut();
            s.tag = "path".to_string();
            let d = format!(
                "M {}L {}M {}L {}",
                pt2str([p[0] - size, p[1]], " "),
                pt2str([p[0] + size, p[1]], " "),
                pt2str([p[0], p[1] + size], " "),
                pt2str([p[0], p[1] - size], " ")
            );
            s.set("d", &d);
        }
        "double-circle" => {
            let r1 = size;
            let indent = (size / 4.0).min(9.0);
            let r2 = if diagram.output_format() == "tactile" {
                size - 9.0
            } else {
                size - indent
            };
            let size_str_2 = py_str(r2);
            let mut s = shape.borrow_mut();
            s.tag = "path".to_string();
            let mut d = format!("M {}", pt2str([p[0] - r1, p[1]], " "));
            d += &format!(
                "A {raw_size_str} {raw_size_str} 0 0 0 {} ",
                pt2str([p[0] + r1, p[1]], " ")
            );
            d += &format!(
                "A {raw_size_str} {raw_size_str} 0 0 0 {} Z ",
                pt2str([p[0] - r1, p[1]], " ")
            );
            d += &format!("M {}", pt2str([p[0] - r2, p[1]], " "));
            d += &format!(
                "A {size_str_2} {size_str_2} 0 0 0 {} ",
                pt2str([p[0] + r2, p[1]], " ")
            );
            d += &format!(
                "A {size_str_2} {size_str_2} 0 0 0 {} Z ",
                pt2str([p[0] - r2, p[1]], " ")
            );
            s.set("d", &d);
        }
        _ => {}
    }

    if diagram.output_format() == "tactile" {
        element.borrow_mut().set("stroke", "black");
        util::set_tactile_fill(element);
    } else {
        let fill = util::get_attr(element, "fill", "red", &mut diagram.ctx);
        element.borrow_mut().set("fill", &fill);
        let stroke = util::get_attr(element, "stroke", "black", &mut diagram.ctx);
        element.borrow_mut().set("stroke", &stroke);
    }
    let thickness = util::get_attr(element, "thickness", "2", &mut diagram.ctx);
    element.borrow_mut().set("thickness", &thickness);
    let attrs = util::get_2d_attr(element, diagram);
    util::add_attr(&shape, attrs);
    util::cliptobbox(&shape, element, diagram);

    let centered = element.borrow().get_or("alignment", "ne").starts_with('c');
    if has_label && !centered {
        add_label(element, diagram, &parent);
    }

    if let Some(outline_group) = outline_group {
        diagram.add_outline(element, &shape, outline_group, None);
        finish_outline(element, diagram, &parent);
    } else if element.borrow().get_or("outline", "no") == "yes"
        || diagram.output_format() == "tactile"
    {
        // Python passes outline_group (None) here; mirror by using the parent
        diagram.add_outline(element, &shape, &parent, None);
        finish_outline(element, diagram, &parent);
    } else {
        xml::append(&parent, &shape);
    }

    if has_label && centered {
        add_label(element, diagram, &parent);
    }
}

fn finish_outline(element: &El, diagram: &mut Diagram, parent: &El) {
    let stroke = element.borrow().get("stroke");
    let thickness = element.borrow().get("thickness");
    let fill = element.borrow().get_or("fill", "none");
    diagram.finish_outline(element, stroke, thickness, &fill, parent);
}

/// point.inside: is p within the drawn point marker at center?
pub fn inside(
    p: [f64; 2],
    center: [f64; 2],
    size: f64,
    style: &str,
    ctm: &crate::core::ctm::CTM,
    buffer: f64,
) -> bool {
    let p = ctm.transform(p);
    let center = ctm.transform(center);
    let p = [p[0] - center[0], p[1] - center[1]];
    match style {
        "circle" | "double-circle" => (p[0] * p[0] + p[1] * p[1]).sqrt() < size + buffer,
        "box" | "cross" | "plus" => {
            let size = if style == "box" { size } else { size * 1.4 };
            p[0].abs() < size + buffer && p[1].abs() < size + buffer
        }
        "diamond" => {
            let size = size * 1.4;
            (p[0] + p[1]).abs() < size + buffer && (p[0] - p[1]).abs() < size + buffer
        }
        _ => false,
    }
}

fn add_label(element: &El, diagram: &mut Diagram, parent: &El) {
    let el = xml::deep_copy(element);
    el.borrow_mut().tag = "label".to_string();

    if element.borrow().get_or("alignment", "").trim() == "e" {
        element.borrow_mut().set("alignment", "east");
    }
    let alignment = util::get_attr(element, "alignment", "ne", &mut diagram.ctx);
    el.borrow_mut().set("alignment", &alignment);
    let size = element.borrow().get_or("size", "4");
    let Some(displacement) = label::alignment_displacement(&alignment) else {
        log::error!("Unknown alignment in label: {alignment}");
        return;
    };
    let anchor = util::get_attr(element, "p", "(0,0)", &mut diagram.ctx);
    el.borrow_mut().set("anchor", &anchor);

    let o: f64 = size.parse::<f64>().unwrap_or(4.0) + 1.0;
    let mut offset = [
        2.0 * o * (displacement[0] + 0.5),
        2.0 * o * (displacement[1] - 0.5),
    ];
    if diagram.output_format() == "tactile" {
        if offset[0] < 0.0 {
            offset[0] -= 6.0;
        }
    } else {
        // push regular labels a bit more in cardinal directions
        let cardinal_push = 3.0;
        if offset[0].abs() < 1e-14 {
            if offset[1] > 0.0 {
                offset[1] += cardinal_push;
            }
            if offset[1] < 0.0 {
                offset[1] -= cardinal_push;
            }
        }
        if offset[1].abs() < 1e-14 {
            if offset[0] > 0.0 {
                offset[0] += cardinal_push;
            }
            if offset[0] < 0.0 {
                offset[0] -= cardinal_push;
            }
        }
    }

    let relative = element.borrow().get("offset");
    if let Some(relative) = relative {
        if let Some(v) = diagram
            .ctx
            .valid_eval(&relative)
            .ok()
            .and_then(|v| v.as_vec_f64().ok())
        {
            offset[0] += v[0];
            offset[1] += v[1];
        }
    }
    el.borrow_mut().set("abs-offset", &util::np2str(offset));

    label::label(&el, diagram, parent, None);
}
