//! Port of prefig/core/line.py.

use crate::core::ctm::CTM;
use crate::core::diagram::{Diagram, Point};
use crate::core::utilities::{self as util, float2str, pt2str};
use crate::core::{arrow, ctm, label};
use crate::value::Value;
use crate::xml::{self, El};

pub enum EndpointOffsets {
    /// A 1-D pair: scalar offsets along the line's direction.
    Along([f64; 2]),
    /// A 2-D pair: absolute (x, y) displacements per endpoint.
    Absolute([[f64; 2]; 2]),
}

impl EndpointOffsets {
    pub fn from_value(v: &Value) -> Option<EndpointOffsets> {
        match v {
            Value::Array(items) if !items.is_empty() => match &items[0] {
                Value::Array(_) => {
                    let rows: Vec<Vec<f64>> = items
                        .iter()
                        .map(|i| i.as_vec_f64().ok())
                        .collect::<Option<_>>()?;
                    Some(EndpointOffsets::Absolute([
                        [rows[0][0], rows[0][1]],
                        [rows[1][0], rows[1][1]],
                    ]))
                }
                _ => {
                    let v = v.as_vec_f64().ok()?;
                    Some(EndpointOffsets::Along([v[0], v[1]]))
                }
            },
            _ => None,
        }
    }
}

/// line.mk_line: build an SVG <line> from two points.
pub fn mk_line(
    p0: Point,
    p1: Point,
    diagram: &mut Diagram,
    id: Option<&str>,
    endpoint_offsets: Option<&EndpointOffsets>,
    user_coords: bool,
) -> El {
    let line = xml::new_element("line");
    diagram.add_id(&line, id);
    let (mut p0, mut p1) = if user_coords {
        (diagram.transform(p0), diagram.transform(p1))
    } else {
        (p0, p1)
    };
    match endpoint_offsets {
        Some(EndpointOffsets::Along(offsets)) => {
            let diff = [p1[0] - p0[0], p1[1] - p0[1]];
            let len = (diff[0] * diff[0] + diff[1] * diff[1]).sqrt();
            let u = [diff[0] / len, diff[1] / len];
            p0 = [p0[0] + offsets[0] * u[0], p0[1] + offsets[0] * u[1]];
            p1 = [p1[0] + offsets[1] * u[0], p1[1] + offsets[1] * u[1]];
        }
        Some(EndpointOffsets::Absolute(offsets)) => {
            p0[0] += offsets[0][0];
            p0[1] -= offsets[0][1];
            p1[0] += offsets[1][0];
            p1[1] -= offsets[1][1];
        }
        None => {}
    }
    {
        let mut l = line.borrow_mut();
        l.set("x1", &float2str(p0[0]));
        l.set("y1", &float2str(p0[1]));
        l.set("x2", &float2str(p1[0]));
        l.set("y2", &float2str(p1[1]));
    }
    line
}

/// Where an "infinite" line through p0, p1 meets the bounding box.
pub fn infinite_line(
    p0: Point,
    p1: Point,
    diagram: &Diagram,
    slope: Option<f64>,
) -> Option<(Point, Point)> {
    let bbox = diagram.bbox();
    let p = p0;
    let v = match slope {
        Some(m) => [1.0, m],
        None => [p1[0] - p0[0], p1[1] - p0[1]],
    };
    let mut t_max = f64::INFINITY;
    let mut t_min = f64::NEG_INFINITY;
    if v[0] != 0.0 {
        let (mut t0, mut t1) = ((bbox[0] - p[0]) / v[0], (bbox[2] - p[0]) / v[0]);
        if t0 > t1 {
            std::mem::swap(&mut t0, &mut t1);
        }
        t_max = t_max.min(t1);
        t_min = t_min.max(t0);
    }
    if v[1] != 0.0 {
        let (mut t0, mut t1) = ((bbox[1] - p[1]) / v[1], (bbox[3] - p[1]) / v[1]);
        if t0 > t1 {
            std::mem::swap(&mut t0, &mut t1);
        }
        t_max = t_max.min(t1);
        t_min = t_min.max(t0);
    }
    if t_min > t_max {
        return None;
    }
    Some((
        [p[0] + t_min * v[0], p[1] + t_min * v[1]],
        [p[0] + t_max * v[0], p[1] + t_max * v[1]],
    ))
}

/// The <line> handler.
pub fn line(element: &El, diagram: &mut Diagram, parent: &El, outline_group: Option<&El>) {
    // A caller (e.g. <poset>) may have precomputed the endpoints and stashed
    // them as source data; those win over any @endpoints/@p1/@p2 attribute.
    let endpoints_data = diagram.get_source_data(element, "endpoints");
    let (p1, p2) = if let Some(value) = endpoints_data {
        match endpoints_pair(&value) {
            Some(pair) => pair,
            None => {
                log::error!("Error in <line> reading endpoints from source data");
                return;
            }
        }
    } else {
        let endpoints = element.borrow().get("endpoints");
        match endpoints {
            None => {
                let p1_attr = element.borrow().get("p1");
                let p2_attr = element.borrow().get("p2");
                let p1 = eval_point(diagram, p1_attr.as_deref());
                let p2 = eval_point(diagram, p2_attr.as_deref());
                match (p1, p2) {
                    (Some(p1), Some(p2)) => (p1, p2),
                    _ => {
                        log::error!("Error in <line> parsing p1/p2");
                        return;
                    }
                }
            }
            Some(attr) => {
                let pair = diagram
                    .ctx
                    .valid_eval(&attr)
                    .ok()
                    .as_ref()
                    .and_then(endpoints_pair);
                match pair {
                    Some(pair) => pair,
                    None => {
                        log::error!("Error in <line> parsing endpoints={attr}");
                        return;
                    }
                }
            }
        }
    };

    let mut endpoint_offsets = None;
    let (p1, p2) = if element.borrow().get_or("infinite", "no") == "yes" {
        match infinite_line(p1, p2, diagram, None) {
            Some(pair) => pair,
            None => return, // the line doesn't hit the bounding box
        }
    } else {
        let offsets_attr = element.borrow().get("endpoint-offsets");
        if let Some(attr) = offsets_attr {
            match diagram
                .ctx
                .valid_eval(&attr)
                .ok()
                .and_then(|v| EndpointOffsets::from_value(&v))
            {
                Some(offsets) => endpoint_offsets = Some(offsets),
                None => {
                    log::error!("Error in <line> parsing endpoint-offsets={attr}");
                    return;
                }
            }
        }
        (p1, p2)
    };

    let id = element.borrow().get("id");
    let line = mk_line(
        p1,
        p2,
        diagram,
        id.as_deref(),
        endpoint_offsets.as_ref(),
        true,
    );
    diagram.register_svg_element(element, &line);

    // hold on to the SVG endpoints in case the line is labeled
    let get_f = |attr: &str| -> f64 { line.borrow().get_or(attr, "0").parse().unwrap_or(0.0) };
    let q1 = [get_f("x1"), get_f("y1")];
    let q2 = [get_f("x2"), get_f("y2")];
    diagram.save_line_endpoints(element, q1, q2);

    util::set_attr(element, "thickness", "2", &mut diagram.ctx);
    let thickness_attr = element.borrow().get_or("thickness", "2");
    let thickness = diagram
        .ctx
        .valid_eval(&thickness_attr)
        .ok()
        .and_then(|v| v.as_num().ok())
        .unwrap_or(2.0);

    let decorations_attr = element.borrow().get("decorations");
    let has_decorations = decorations_attr.is_some();
    let (decoration_path_el, decoration_path_id_str) = if let Some(ref dec_str) = decorations_attr {
        let dec_path = xml::new_element("path");
        let line_id = line.borrow().get_or("id", "none");
        let dec_id = format!("{}-decorations", line_id);
        dec_path.borrow_mut().set("id", &dec_id);

        let diff = [q2[0] - q1[0], q2[1] - q1[1]];
        let length = (diff[0] * diff[0] + diff[1] * diff[1]).sqrt();
        let angle = diff[1].atan2(diff[0]);

        let mut ctm_local = CTM::new();
        ctm_local.translate(q1[0], q1[1]);
        ctm_local.rotate(angle.to_degrees(), true);

        let mut d = String::new();
        for c in dec_str.chars() {
            let h = 3.0 * thickness;
            match c {
                '[' => {
                    let w = 2.0 * thickness;
                    let p1 = ctm_local.transform([w, h]);
                    let p2 = ctm_local.transform([0.0, h]);
                    let p3 = ctm_local.transform([0.0, -h]);
                    let p4 = ctm_local.transform([w, -h]);
                    d += &format!("M {} {} L {} {} L {} {} L {} {} ",
                        float2str(p1[0]), float2str(p1[1]),
                        float2str(p2[0]), float2str(p2[1]),
                        float2str(p3[0]), float2str(p3[1]),
                        float2str(p4[0]), float2str(p4[1]));
                }
                ']' => {
                    let w = 2.0 * thickness;
                    let p1 = ctm_local.transform([length - w, h]);
                    let p2 = ctm_local.transform([length, h]);
                    let p3 = ctm_local.transform([length, -h]);
                    let p4 = ctm_local.transform([length - w, -h]);
                    d += &format!("M {} {} L {} {} L {} {} L {} {} ",
                        float2str(p1[0]), float2str(p1[1]),
                        float2str(p2[0]), float2str(p2[1]),
                        float2str(p3[0]), float2str(p3[1]),
                        float2str(p4[0]), float2str(p4[1]));
                }
                ')' => {
                    let w = thickness;
                    let b = 4.25 * thickness;
                    let p1 = ctm_local.transform([length - w, -h]);
                    let p3 = ctm_local.transform([length - w, h]);
                    d += &format!("M {} {} A {} {} 0 0,1 {} {} ",
                        float2str(p1[0]), float2str(p1[1]),
                        float2str(b), float2str(b),
                        float2str(p3[0]), float2str(p3[1]));
                }
                '(' => {
                    let w = thickness;
                    let b = 4.25 * thickness;
                    let p1 = ctm_local.transform([w, h]);
                    let p3 = ctm_local.transform([w, -h]);
                    d += &format!("M {} {} A {} {} 0 0,1 {} {} ",
                        float2str(p1[0]), float2str(p1[1]),
                        float2str(b), float2str(b),
                        float2str(p3[0]), float2str(p3[1]));
                }
                _ => {
                    log::error!("{c} is not a valid decoration on a line");
                }
            }
        }
        dec_path.borrow_mut().set("d", d.trim_end());
        (Some(dec_path), Some(dec_id))
    } else {
        (None, None)
    };

    util::set_attr(element, "stroke", "black", &mut diagram.ctx);
    if diagram.output_format() == "tactile" {
        element.borrow_mut().set("stroke", "black");
    }
    util::add_attr(&line, util::get_1d_attr(element, &mut diagram.ctx));
    if has_decorations {
        if let Some(ref dec_path) = decoration_path_el {
            util::add_attr(dec_path, util::get_1d_attr(element, &mut diagram.ctx));
        }
    }

    let arrows: i64 = element.borrow().get_or("arrows", "0").parse().unwrap_or(0);
    let (mut forward, mut backward) = ("marker-end", "marker-start");
    if element.borrow().get_or("reverse", "no") == "yes" {
        std::mem::swap(&mut forward, &mut backward);
    }

    let arrow_width = element.borrow().get("arrow-width");
    let arrow_angles = element.borrow().get("arrow-angles");
    let mut p0 = q1;
    let mut p1_svg = q2;
    let mut angle = 0.0;
    let mut arrow_length = 0.0;
    if arrows > 0 {
        let arrow_id = arrow::add_arrowhead_to_path(
            diagram,
            forward,
            &line,
            arrow_width.as_deref(),
            arrow_angles.as_deref(),
        );
        let diff = [p1_svg[0] - p0[0], p1_svg[1] - p0[1]];
        let length = (diff[0] * diff[0] + diff[1] * diff[1]).sqrt();
        angle = diff[1].atan2(diff[0]);
        arrow_length = thickness
            * arrow_id
                .and_then(|id| diagram.arrow_lengths.get(&id).copied())
                .unwrap_or(0.0);
        let shortened = length - arrow_length;
        p1_svg = [
            shortened * angle.cos() + p0[0],
            shortened * angle.sin() + p0[1],
        ];
        line.borrow_mut().set("x2", &float2str(p1_svg[0]));
        line.borrow_mut().set("y2", &float2str(p1_svg[1]));
    }
    if arrows > 1 {
        arrow::add_arrowhead_to_path(
            diagram,
            backward,
            &line,
            arrow_width.as_deref(),
            arrow_angles.as_deref(),
        );
        p0 = [
            p0[0] + arrow_length * angle.cos(),
            p0[1] + arrow_length * angle.sin(),
        ];
        line.borrow_mut().set("x1", &float2str(p0[0]));
        line.borrow_mut().set("y1", &float2str(p0[1]));
    }

    let additional_attr = element.borrow().get("additional-arrows");
    if let Some(attr) = additional_attr {
        if let Ok(value) = diagram.ctx.valid_eval(&attr) {
            let mut additional = match &value {
                Value::Array(_) => value.as_vec_f64().unwrap_or_default(),
                _ => value.as_num().map(|n| vec![n]).unwrap_or_default(),
            };
            additional.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            line.borrow_mut().tag = "path".to_string();
            let x1 = line.borrow().get_or("x1", "0");
            let y1 = line.borrow().get_or("y1", "0");
            let x2 = line.borrow().get_or("x2", "0");
            let y2 = line.borrow().get_or("y2", "0");
            let p1n: Point = [x1.parse().unwrap_or(0.0), y1.parse().unwrap_or(0.0)];
            let p2n: Point = [x2.parse().unwrap_or(0.0), y2.parse().unwrap_or(0.0)];
            let mut cmds = vec!["M".to_string(), x1.clone(), y1.clone()];
            for a in additional {
                let p = [
                    (1.0 - a) * p1n[0] + a * p2n[0],
                    (1.0 - a) * p1n[1] + a * p2n[1],
                ];
                cmds.push("L".to_string());
                cmds.push(pt2str(p, " "));
            }
            cmds.push("L".to_string());
            cmds.push(x2);
            cmds.push(y2);
            line.borrow_mut().set("d", &cmds.join(" "));
            arrow::add_arrowhead_to_path(
                diagram,
                "marker-mid",
                &line,
                arrow_width.as_deref(),
                arrow_angles.as_deref(),
            );
        }
    }

    util::cliptobbox(&line, element, diagram);
    let has_label = label::has_label(element);
    let mut parent = parent.clone();
    if has_label || has_decorations {
        let group = xml::sub_element(&parent, "g");
        group
            .borrow_mut()
            .set("id", &line.borrow().get_or("id", "none"));
        line.borrow_mut().pop_attr("id");
        parent = group;
        if has_label {
            add_label(element, diagram, &parent);
        }
    }

    if let Some(outline_group) = outline_group {
        diagram.add_outline(element, &line, outline_group, None, None);
        if let (Some(ref dec_path), Some(ref dec_id)) = (&decoration_path_el, &decoration_path_id_str) {
            diagram.add_outline(element, dec_path, outline_group, None, Some(dec_id.as_str()));
        }
        finish_outline(element, diagram, &parent, None);
        if let Some(ref dec_id) = decoration_path_id_str {
            finish_outline(element, diagram, &parent, Some(dec_id.clone()));
        }
    } else if element.borrow().get_or("outline", "no") == "yes"
        || diagram.output_format() == "tactile"
    {
        diagram.add_outline(element, &line, &parent, None, None);
        if let (Some(ref dec_path), Some(ref dec_id)) = (&decoration_path_el, &decoration_path_id_str) {
            diagram.add_outline(element, dec_path, &parent, None, Some(dec_id.as_str()));
        }
        finish_outline(element, diagram, &parent, None);
        if let Some(ref dec_id) = decoration_path_id_str {
            finish_outline(element, diagram, &parent, Some(dec_id.clone()));
        }
    } else {
        xml::append(&parent, &line);
        if let Some(ref dec_path) = decoration_path_el {
            xml::append(&parent, dec_path);
        }
    }
}

/// Interpret a Value as a pair of 2-D endpoints: `[[x1, y1], [x2, y2]]`.
fn endpoints_pair(value: &Value) -> Option<(Point, Point)> {
    match value {
        Value::Array(items) if items.len() == 2 => {
            let p1 = items[0].as_vec_f64().ok()?;
            let p2 = items[1].as_vec_f64().ok()?;
            (p1.len() >= 2 && p2.len() >= 2).then(|| ([p1[0], p1[1]], [p2[0], p2[1]]))
        }
        _ => None,
    }
}

fn eval_point(diagram: &mut Diagram, attr: Option<&str>) -> Option<Point> {
    let attr = attr?;
    let v = diagram.ctx.valid_eval(attr).ok()?.as_vec_f64().ok()?;
    (v.len() >= 2).then(|| [v[0], v[1]])
}

fn finish_outline(element: &El, diagram: &mut Diagram, parent: &El, id: Option<String>) {
    let stroke = element.borrow().get("stroke");
    let thickness = element.borrow().get("thickness");
    let fill = element.borrow().get_or("fill", "none");
    diagram.finish_outline(element, stroke, thickness, &fill, parent, id);
}

fn add_label(element: &El, diagram: &mut Diagram, parent: &El) {
    let el = xml::deep_copy(element);
    el.borrow_mut().tag = "label".to_string();

    let Some((mut q1, mut q2)) = diagram.retrieve_line_endpoints(element) else {
        return;
    };

    let location_attr = element.borrow().get_or("label-location", "0.5");
    let mut label_location = diagram
        .ctx
        .valid_eval(&location_attr)
        .ok()
        .and_then(|v| v.as_num().ok())
        .unwrap_or(0.5);
    if label_location < 0.0 {
        label_location = -label_location;
        std::mem::swap(&mut q1, &mut q2);
    }

    el.borrow_mut().set("user-coords", "no");
    let diff = [q2[0] - q1[0], q2[1] - q1[1]];
    let d = (diff[0] * diff[0] + diff[1] * diff[1]).sqrt();
    let angle = diff[1].atan2(diff[0]).to_degrees();
    if diagram.output_format() == "tactile" {
        let anchor = [
            q1[0] + label_location * diff[0],
            q1[1] + label_location * diff[1],
        ];
        el.borrow_mut().set(
            "anchor",
            &format!(
                "({}, {})",
                crate::value::py_str(anchor[0]),
                crate::value::py_str(anchor[1])
            ),
        );
        let alignment = label::get_alignment_from_direction([diff[1], diff[0]]);
        el.borrow_mut().set("alignment", &alignment);
        label::label(&el, diagram, parent, None);
    } else {
        let tform = format!(
            "{} {}",
            ctm::translatestr(q1[0], q1[1]),
            ctm::rotatestr(-angle)
        );
        let distance = d * label_location;
        let g = xml::sub_element(parent, "g");
        g.borrow_mut().set("transform", &tform);
        el.borrow_mut()
            .set("anchor", &format!("({},0)", crate::value::py_str(distance)));
        let alignment = element.borrow().get_or("alignment", "north");
        el.borrow_mut().set("alignment", &alignment);
        // Native-label mode: the label's `<g>` (and thus its lifted-out native
        // runs) live in this wrapper's local frame. Record the wrapper as a
        // matrix matching the SVG string above — `translate(q1)` then
        // `rotatestr(-angle)`, which emits `rotate(angle)` == `rotation(angle)` —
        // so `position_svg_label` composes it into each run's absolute placement.
        diagram.set_native_wrapper(
            &el,
            ctm::concat(ctm::translation(q1[0], q1[1]), ctm::rotation(angle, true)),
        );
        label::label(&el, diagram, &g, None);
    }
}
