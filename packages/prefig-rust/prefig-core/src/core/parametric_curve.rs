//! Port of prefig/core/parametric_curve.py.

use crate::core::diagram::Diagram;
use crate::core::math_utilities::length;
use crate::core::utilities::{self as util, pt2str};
use crate::core::arrow;
use crate::evaluator::interp_call;
use crate::value::Value;
use crate::xml::{self, El};

const SEPARATION_TOLERANCE: f64 = 5.0;

fn eval_curve(diagram: &mut Diagram, f: &Value, t: f64) -> Option<[f64; 2]> {
    let v = interp_call(f, &[Value::Num(t)], &mut diagram.ctx)
        .ok()?
        .as_vec_f64()
        .ok()?;
    (v.len() >= 2).then(|| diagram.transform([v[0], v[1]]))
}

pub fn parametric_curve(
    element: &El,
    diagram: &mut Diagram,
    parent: &El,
    outline_group: Option<&El>,
) {
    let function_attr = element.borrow().get("function").unwrap_or_default();
    let Ok(f) = diagram.ctx.valid_eval(&function_attr) else {
        log::error!("Error in <parametric-curve> defining function={function_attr}");
        return;
    };
    let domain_attr = element.borrow().get("domain").unwrap_or_default();
    let Some(domain) = diagram
        .ctx
        .valid_eval(&domain_attr)
        .ok()
        .and_then(|v| v.as_vec_f64().ok())
    else {
        log::error!("Error in <parametric-curve> defining domain={domain_attr}");
        return;
    };

    let arrows: i64 = element
        .borrow()
        .get_or("arrows", "0")
        .parse()
        .unwrap_or(0);
    let n: usize = element
        .borrow()
        .get_or("N", "100")
        .parse()
        .unwrap_or(100);

    let mut t = domain[0];
    let dt = (domain[1] - domain[0]) / n as f64;
    let Some(p) = eval_curve(diagram, &f, t) else {
        return;
    };
    let mut points = vec![format!("M {}", pt2str(p, " "))];
    for _ in 0..n {
        take_step(diagram, &f, t, dt, &mut points);
        t += dt;
    }

    if element.borrow().get_or("closed", "no") == "yes" {
        points.push("Z".to_string());
    }

    let arrow_location = element.borrow().get("arrow-location");
    if arrows > 0 {
        if let Some(attr) = arrow_location {
            if let Some(arrow_location) = diagram
                .ctx
                .valid_eval(&attr)
                .ok()
                .and_then(|v| v.as_num().ok())
            {
                let num_pts = 5;
                let mut t = arrow_location - num_pts as f64 * dt;
                if let Some(p) = eval_curve(diagram, &f, t) {
                    points.push(format!("M {}", pt2str(p, " ")));
                }
                for _ in 0..num_pts {
                    t += dt;
                    if let Some(p) = eval_curve(diagram, &f, t) {
                        points.push(format!("L {}", pt2str(p, " ")));
                    }
                }
            }
        }
    }

    let d = points.join(" ");

    if diagram.output_format() == "tactile" {
        element.borrow_mut().set("stroke", "black");
        util::set_tactile_fill(element);
    } else {
        util::set_attr(element, "stroke", "blue", &mut diagram.ctx);
        util::set_attr(element, "fill", "none", &mut diagram.ctx);
    }
    util::set_attr(element, "thickness", "2", &mut diagram.ctx);

    let path = xml::new_element("path");
    let id = element.borrow().get("id");
    diagram.add_id(&path, id.as_deref());
    diagram.register_svg_element(element, &path);

    path.borrow_mut().set("d", &d);
    let attrs = util::get_2d_attr(element, diagram);
    util::add_attr(&path, attrs);

    let clip = element.borrow().get_or("cliptobbox", "yes");
    element.borrow_mut().set("cliptobbox", &clip);
    util::cliptobbox(&path, element, diagram);

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
            &path,
            arrow_width.as_deref(),
            arrow_angles.as_deref(),
        );
    }
    if arrows > 1 {
        arrow::add_arrowhead_to_path(
            diagram,
            backward,
            &path,
            arrow_width.as_deref(),
            arrow_angles.as_deref(),
        );
    }

    if let Some(outline_group) = outline_group {
        diagram.add_outline(element, &path, outline_group, None);
        finish_outline(element, diagram, parent);
    } else if element.borrow().get_or("outline", "no") == "yes"
        || diagram.output_format() == "tactile"
    {
        diagram.add_outline(element, &path, parent, None);
        finish_outline(element, diagram, parent);
    } else {
        xml::append(parent, &path);
    }
}

/// Recursively subdivide until consecutive points are close on screen.
fn take_step(diagram: &mut Diagram, f: &Value, t0: f64, dt: f64, out: &mut Vec<String>) {
    let Some(last_p) = eval_curve(diagram, f, t0) else {
        return;
    };
    let t1 = t0 + dt;
    let Some(p) = eval_curve(diagram, f, t1) else {
        return;
    };
    if length([p[0] - last_p[0], p[1] - last_p[1]]) < SEPARATION_TOLERANCE {
        out.push(format!("L {}", pt2str(p, " ")));
        return;
    }
    let dt = dt / 2.0;
    take_step(diagram, f, t0, dt, out);
    take_step(diagram, f, t0 + dt, dt, out);
}

fn finish_outline(element: &El, diagram: &mut Diagram, parent: &El) {
    let stroke = element.borrow().get("stroke");
    let thickness = element.borrow().get("thickness");
    let fill = element.borrow().get_or("fill", "none");
    diagram.finish_outline(element, stroke, thickness, &fill, parent);
}
