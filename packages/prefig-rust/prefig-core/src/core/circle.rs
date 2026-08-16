//! Port of prefig/core/circle.py: circles, ellipses, arcs (angle markers TODO).

use crate::core::arrow;
use crate::core::ctm::CTM;
use crate::core::diagram::{Diagram, Point};
use crate::core::utilities::{self as util, pt2str};
use crate::xml::{self, El};

pub fn circle(element: &El, diagram: &mut Diagram, parent: &El, outline_group: Option<&El>) {
    let center_attr = element.borrow().get("center");
    let Some(center) = eval_point(diagram, center_attr.as_deref()) else {
        log::error!("Error in <circle> parsing center={center_attr:?}");
        return;
    };
    let radius_attr = element.borrow().get_or("radius", "1");
    let Ok(radius) = diagram
        .ctx
        .valid_eval(&radius_attr)
        .and_then(|v| v.as_num())
    else {
        log::error!("Error in <circle> parsing radius={radius_attr}");
        return;
    };

    // a path rather than an SVG ellipse, for unions and intersections
    let circle = xml::new_element("path");
    let id = element.borrow().get("id");
    diagram.add_id(&circle, id.as_deref());
    diagram.register_svg_element(element, &circle);

    let n_attr = element.borrow().get_or("N", "100");
    let n = diagram
        .ctx
        .valid_eval(&n_attr)
        .ok()
        .and_then(|v| v.as_num().ok())
        .unwrap_or(100.0) as usize;
    let mut cmds = make_path(diagram, center, (radius, radius), (0.0, 360.0), 0.0, n);
    cmds.push("Z".to_string());
    circle.borrow_mut().set("d", &cmds.join(" "));

    if diagram.output_format() == "tactile" {
        if element.borrow().get("stroke").is_some() {
            element.borrow_mut().set("stroke", "black");
        }
        util::set_tactile_fill(element);
    } else {
        let stroke = element.borrow().get_or("stroke", "black");
        element.borrow_mut().set("stroke", &stroke);
        let fill = element.borrow().get_or("fill", "none");
        element.borrow_mut().set("fill", &fill);
    }
    let thickness = element.borrow().get_or("thickness", "2");
    element.borrow_mut().set("thickness", &thickness);
    let attrs = util::get_2d_attr(element, diagram);
    util::add_attr(&circle, attrs);
    util::cliptobbox(&circle, element, diagram);

    finish_element(element, diagram, parent, outline_group, &circle, true);
}

pub fn ellipse(element: &El, diagram: &mut Diagram, parent: &El, outline_group: Option<&El>) {
    let center_attr = element.borrow().get("center");
    let Some(center) = eval_point(diagram, center_attr.as_deref()) else {
        log::error!("Error in <ellipse> parsing center={center_attr:?}");
        return;
    };
    let axes_attr = element.borrow().get_or("axes", "(1,1)");
    let Some(axes_length) = diagram
        .ctx
        .valid_eval(&axes_attr)
        .ok()
        .and_then(|v| v.as_vec_f64().ok())
    else {
        log::error!("Error in <ellipse> parsing axes={axes_attr}");
        return;
    };

    let rotate_attr = element.borrow().get_or("rotate", "0");
    let mut rotate = diagram
        .ctx
        .valid_eval(&rotate_attr)
        .ok()
        .and_then(|v| v.as_num().ok())
        .unwrap_or(0.0);
    if element.borrow().get_or("degrees", "yes") == "no" {
        rotate = rotate.to_degrees();
    }

    let n_attr = element.borrow().get_or("N", "100");
    let n = diagram
        .ctx
        .valid_eval(&n_attr)
        .ok()
        .and_then(|v| v.as_num().ok())
        .unwrap_or(100.0) as usize;

    let circle = xml::new_element("path");
    let id = element.borrow().get("id");
    diagram.add_id(&circle, id.as_deref());
    diagram.register_svg_element(element, &circle);

    let mut cmds = make_path(
        diagram,
        center,
        (axes_length[0], axes_length[1]),
        (0.0, 360.0),
        rotate,
        n,
    );
    cmds.push("Z".to_string());
    circle.borrow_mut().set("d", &cmds.join(" "));

    if diagram.output_format() == "tactile" {
        if element.borrow().get("stroke").is_some() {
            element.borrow_mut().set("stroke", "black");
        }
        util::set_tactile_fill(element);
    } else {
        let stroke = element.borrow().get_or("stroke", "none");
        element.borrow_mut().set("stroke", &stroke);
        let fill = element.borrow().get_or("fill", "none");
        element.borrow_mut().set("fill", &fill);
    }
    let thickness = element.borrow().get_or("thickness", "2");
    element.borrow_mut().set("thickness", &thickness);
    let attrs = util::get_2d_attr(element, diagram);
    util::add_attr(&circle, attrs);
    util::cliptobbox(&circle, element, diagram);

    finish_element(element, diagram, parent, outline_group, &circle, false);
}

pub fn arc(element: &El, diagram: &mut Diagram, parent: &El, outline_group: Option<&El>) {
    if diagram.output_format() == "tactile" {
        if element.borrow().get("stroke").is_some() {
            element.borrow_mut().set("stroke", "black");
        }
        util::set_tactile_fill(element);
    } else {
        let stroke = element.borrow().get_or("stroke", "none");
        element.borrow_mut().set("stroke", &stroke);
        let fill = element.borrow().get_or("fill", "none");
        element.borrow_mut().set("fill", &fill);
    }
    let thickness = element.borrow().get_or("thickness", "2");
    element.borrow_mut().set("thickness", &thickness);

    let points_attr = element.borrow().get("points");
    let (center, angular_range) = if let Some(attr) = points_attr {
        let Some(points) = eval_points(diagram, &attr) else {
            log::error!("Error in <arc> parsing points={attr}");
            return;
        };
        let center = points[1];
        let v = [points[0][0] - points[1][0], points[0][1] - points[1][1]];
        let u = [points[2][0] - points[1][0], points[2][1] - points[1][1]];
        let start = v[1].atan2(v[0]).to_degrees();
        let mut stop = u[1].atan2(u[0]).to_degrees();
        if stop < start {
            stop += 360.0;
        }
        element.borrow_mut().set("degrees", "yes");
        (center, (start, stop))
    } else {
        let center_attr = element.borrow().get("center");
        let Some(center) = eval_point(diagram, center_attr.as_deref()) else {
            log::error!("Error in <arc> parsing center={center_attr:?}");
            return;
        };
        let range_attr = element.borrow().get("range");
        let Some(range) = range_attr
            .as_deref()
            .and_then(|attr| diagram.ctx.valid_eval(attr).ok())
            .and_then(|v| v.as_vec_f64().ok())
        else {
            log::error!("Error in <arc> parsing range={range_attr:?}");
            return;
        };
        (center, (range[0], range[1]))
    };

    let radius_attr = element.borrow().get("radius");
    let Some(radius) = radius_attr
        .as_deref()
        .and_then(|attr| diagram.ctx.valid_eval(attr).ok())
        .and_then(|v| v.as_num().ok())
    else {
        log::error!("Error in <arc> parsing radius={radius_attr:?}");
        return;
    };
    let sector = element.borrow().get_or("sector", "no") == "yes";

    let angular_range = if element.borrow().get_or("degrees", "yes") == "no" {
        (angular_range.0.to_degrees(), angular_range.1.to_degrees())
    } else {
        angular_range
    };

    let n_attr = element.borrow().get_or("N", "100");
    let n = diagram
        .ctx
        .valid_eval(&n_attr)
        .ok()
        .and_then(|v| v.as_num().ok())
        .unwrap_or(100.0) as usize;

    let arc = xml::new_element("path");
    let id = element.borrow().get("id");
    diagram.add_id(&arc, id.as_deref());
    diagram.register_svg_element(element, &arc);

    let mut cmds = make_path(diagram, center, (radius, radius), angular_range, 0.0, n);
    if sector {
        cmds.push("L".to_string());
        cmds.push(pt2str(diagram.transform(center), " "));
        cmds.push("Z".to_string());
    }
    arc.borrow_mut().set("d", &cmds.join(" "));
    let attrs = util::get_2d_attr(element, diagram);
    util::add_attr(&arc, attrs);
    util::cliptobbox(&arc, element, diagram);

    let arrows: i64 = element.borrow().get_or("arrows", "0").parse().unwrap_or(0);
    let (mut forward, mut backward) = ("marker-end", "marker-start");
    if element.borrow().get_or("reverse", "no") == "yes" {
        std::mem::swap(&mut forward, &mut backward);
    }
    let arrow_width = element.borrow().get("arrow-width");
    let arrow_angles = element.borrow().get("arrow-angles");
    if arrows > 0 {
        arrow::add_arrowhead_to_path(
            diagram,
            forward,
            &arc,
            arrow_width.as_deref(),
            arrow_angles.as_deref(),
        );
    }
    if arrows > 1 {
        arrow::add_arrowhead_to_path(
            diagram,
            backward,
            &arc,
            arrow_width.as_deref(),
            arrow_angles.as_deref(),
        );
    }

    finish_element(element, diagram, parent, outline_group, &arc, false);
}

/// The angle-marker handler (circle.angle in Python).
pub fn angle(element: &El, diagram: &mut Diagram, parent: &El, outline_group: Option<&El>) {
    use crate::core::math_utilities::{dot, normalize};
    use crate::core::{arrow, label};

    let has_label = label::has_label(element);

    let stroke = element.borrow().get_or("stroke", "black");
    element.borrow_mut().set("stroke", &stroke);
    if diagram.output_format() == "tactile" {
        util::set_tactile_fill(element);
    } else {
        let fill = element.borrow().get_or("fill", "none");
        element.borrow_mut().set("fill", &fill);
    }
    let thickness_attr = element.borrow().get_or("thickness", "2");
    element.borrow_mut().set("thickness", &thickness_attr);

    let points_attr = element.borrow().get("points");
    let (p, p1, p2) = match points_attr {
        None => {
            let get = |diagram: &mut Diagram, attr: &str| -> Option<Point> {
                let a = element.borrow().get(attr)?;
                let v = diagram.ctx.valid_eval(&a).ok()?.as_vec_f64().ok()?;
                (v.len() >= 2).then(|| [v[0], v[1]])
            };
            let (Some(p), Some(p1), Some(p2)) =
                (get(diagram, "p"), get(diagram, "p1"), get(diagram, "p2"))
            else {
                log::error!("Error in <angle-marker> parsing attributes p, p1, or p2");
                return;
            };
            (p, p1, p2)
        }
        Some(attr) => {
            let Some(points) = eval_points(diagram, &attr) else {
                log::error!("Error in <angle-marker> parsing points={attr}");
                return;
            };
            (points[1], points[0], points[2])
        }
    };

    // is this a right angle?
    let u = normalize([p1[0] - p[0], p1[1] - p[1]]);
    let v = normalize([p2[0] - p[0], p2[1] - p[1]]);
    let right = dot(u, v).abs() < 0.001;

    // convert to svg coordinates
    let p = diagram.transform(p);
    let p1 = diagram.transform(p1);
    let p2 = diagram.transform(p2);

    let v1 = normalize([p1[0] - p[0], p1[1] - p[1]]);
    let v2 = normalize([p2[0] - p[0], p2[1] - p[1]]);

    // orientation from the cross product (svg y-axis points down)
    let large_arc = v1[0] * v2[1] - v1[1] * v2[0] > 0.0;

    let angle_measure = if large_arc {
        2.0 * std::f64::consts::PI - dot(v1, v2).acos()
    } else {
        dot(v1, v2).acos()
    };

    // heuristically determined radius
    let mut default_radius = (27.0 / angle_measure) as i64;
    default_radius = default_radius.clamp(15, 30);
    let mut default_radius = default_radius as f64;
    if diagram.output_format() == "tactile" {
        default_radius *= 1.5;
    }
    let radius_attr = element
        .borrow()
        .get_or("radius", &py_str_int(default_radius));
    let radius = diagram
        .ctx
        .valid_eval(&radius_attr)
        .ok()
        .and_then(|v| v.as_num().ok())
        .unwrap_or(default_radius);

    let mut angle2 = v1[1].atan2(v1[0]);
    let angle1 = v2[1].atan2(v2[0]);

    let sum = [v1[0] + v2[0], v1[1] + v2[1]];
    let direction = if sum[0].abs() < 1e-8 && sum[1].abs() < 1e-8 {
        [v1[1], -v1[0]]
    } else {
        let n = normalize(sum);
        let sign = if large_arc { -1.0 } else { 1.0 };
        [sign * n[0], sign * n[1]]
    };
    let label_location = [p[0] + direction[0] * radius, p[1] + direction[1] * radius];
    element
        .borrow_mut()
        .set("label-location", &pt2str_comma(label_location));
    let alignment = element.borrow().get("alignment");
    match alignment {
        None => {
            let a = label::get_alignment_from_direction([direction[0], -direction[1]]);
            element.borrow_mut().set("alignment", &a);
        }
        Some(a) if a.trim() == "e" => {
            element.borrow_mut().set("alignment", "east");
        }
        _ => {}
    }

    let arc = xml::new_element("path");
    let mut parent = parent.clone();
    if has_label {
        let group = xml::sub_element(&parent, "g");
        let id = element.borrow().get("id");
        diagram.add_id(&group, id.as_deref());
        parent = group;
        angle_add_label(element, diagram, &parent);
    } else {
        let id = element.borrow().get("id");
        diagram.add_id(&arc, id.as_deref());
    }
    diagram.register_svg_element(element, &arc);

    let attrs = util::get_2d_attr(element, diagram);
    util::add_attr(&arc, attrs);
    util::cliptobbox(&arc, element, diagram);

    let thickness = diagram
        .ctx
        .valid_eval(&thickness_attr)
        .ok()
        .and_then(|v| v.as_num().ok())
        .unwrap_or(2.0);
    let mut angle1 = angle1;
    if element.borrow().get_or("arrow", "no") == "yes" {
        let arrow_width = element.borrow().get("arrow-width");
        let arrow_angles = element.borrow().get("arrow-angles");
        if element.borrow().get_or("reverse", "no") == "yes" {
            let arrow_id = arrow::add_arrowhead_to_path(
                diagram,
                "marker-end",
                &arc,
                arrow_width.as_deref(),
                arrow_angles.as_deref(),
            );
            let arrow_length = arrow_id
                .and_then(|id| diagram.arrow_lengths.get(&id).copied())
                .unwrap_or(0.0);
            angle2 -= thickness * arrow_length / radius;
        } else {
            let arrow_id = arrow::add_arrowhead_to_path(
                diagram,
                "marker-start",
                &arc,
                arrow_width.as_deref(),
                arrow_angles.as_deref(),
            );
            let arrow_length = arrow_id
                .and_then(|id| diagram.arrow_lengths.get(&id).copied())
                .unwrap_or(0.0);
            angle1 += thickness * arrow_length / radius;
        }
    }

    let d = if right && angle_measure.to_degrees() < 180.0 {
        element.borrow_mut().set("arrow", "no");
        format!(
            "M {} L {} L {}",
            pt2str([radius * v1[0] + p[0], radius * v1[1] + p[1]], " "),
            pt2str(
                [
                    radius * (v1[0] + v2[0]) + p[0],
                    radius * (v1[1] + v2[1]) + p[1]
                ],
                " "
            ),
            pt2str([radius * v2[0] + p[0], radius * v2[1] + p[1]], " ")
        )
    } else {
        while angle2 < angle1 {
            angle2 += 2.0 * std::f64::consts::PI;
        }
        let n = 100;
        let dangle = (angle2 - angle1) / n as f64;
        let mut a = angle1;
        let arc_pt = [p[0] + radius * a.cos(), p[1] + radius * a.sin()];
        let mut cmds = vec!["M".to_string(), pt2str(arc_pt, " ")];
        for _ in 0..n {
            a += dangle;
            let arc_pt = [p[0] + radius * a.cos(), p[1] + radius * a.sin()];
            cmds.push(format!("L {}", pt2str(arc_pt, " ")));
        }
        cmds.join(" ")
    };
    arc.borrow_mut().set("d", &d);

    if let Some(outline_group) = outline_group {
        diagram.add_outline(element, &arc, outline_group, Some(4));
        finish_outline(element, diagram, &parent, false);
    } else if element.borrow().get_or("outline", "no") == "yes"
        || diagram.output_format() == "tactile"
    {
        diagram.add_outline(element, &arc, &parent, Some(4));
        finish_outline(element, diagram, &parent, false);
    } else {
        xml::append(&parent, &arc);
    }
}

fn py_str_int(x: f64) -> String {
    crate::value::py_str(x)
}

fn pt2str_comma(p: Point) -> String {
    pt2str(p, ",")
}

fn angle_add_label(element: &El, diagram: &mut Diagram, parent: &El) {
    use crate::core::label;
    let el = xml::deep_copy(element);
    el.borrow_mut().tag = "label".to_string();
    let alignment = element.borrow().get_or("alignment", "");
    el.borrow_mut().set("alignment", &alignment);
    let location = element.borrow().get_or("label-location", "(0,0)");
    el.borrow_mut().set("p", &location);
    el.borrow_mut().set("user-coords", "no");
    let offset = element.borrow().get("offset");
    if let Some(offset) = offset {
        el.borrow_mut().set("offset", &offset);
    }
    label::label(&el, diagram, parent, None);
}

pub fn make_path(
    diagram: &Diagram,
    center: Point,
    axes_length: (f64, f64),
    angular_range: (f64, f64),
    rotate: f64,
    n: usize,
) -> Vec<String> {
    let mut ctm = CTM::new();
    ctm.translate(center[0], center[1]);
    ctm.rotate(rotate, true);
    ctm.scale(axes_length.0, axes_length.1);
    let start = angular_range.0.to_radians();
    let stop = angular_range.1.to_radians();
    let mut t = start;
    let dt = (stop - start) / n as f64;
    let mut cmds = Vec::new();
    for _ in 0..=n {
        let point = ctm.transform([t.cos(), t.sin()]);
        let point = diagram.transform(point);
        cmds.push(if cmds.is_empty() { "M" } else { "L" }.to_string());
        cmds.push(pt2str(point, " "));
        t += dt;
    }
    cmds
}

fn eval_point(diagram: &mut Diagram, attr: Option<&str>) -> Option<Point> {
    let attr = attr?;
    let v = diagram.ctx.valid_eval(attr).ok()?.as_vec_f64().ok()?;
    (v.len() >= 2).then(|| [v[0], v[1]])
}

fn eval_points(diagram: &mut Diagram, attr: &str) -> Option<Vec<Point>> {
    let value = diagram.ctx.valid_eval(attr).ok()?;
    match value {
        crate::value::Value::Array(items) => items
            .iter()
            .map(|i| {
                let v = i.as_vec_f64().ok()?;
                (v.len() >= 2).then(|| [v[0], v[1]])
            })
            .collect(),
        _ => None,
    }
}

fn finish_element(
    element: &El,
    diagram: &mut Diagram,
    parent: &El,
    outline_group: Option<&El>,
    path: &El,
    fill_from_attr: bool,
) {
    if let Some(outline_group) = outline_group {
        diagram.add_outline(element, path, outline_group, None);
        finish_outline(element, diagram, parent, fill_from_attr);
    } else if element.borrow().get_or("outline", "no") == "yes"
        || diagram.output_format() == "tactile"
    {
        diagram.add_outline(element, path, parent, None);
        finish_outline(element, diagram, parent, fill_from_attr);
    } else {
        xml::append(parent, path);
    }
}

fn finish_outline(element: &El, diagram: &mut Diagram, parent: &El, fill_from_attr: bool) {
    let stroke = element.borrow().get("stroke");
    let thickness = element.borrow().get("thickness");
    let fill = if fill_from_attr {
        element.borrow().get_or("fill", "None")
    } else {
        element.borrow().get_or("fill", "none")
    };
    diagram.finish_outline(element, stroke, thickness, &fill, parent);
}
