//! Port of prefig/core/graph.py: the graph of a one-variable function.

use crate::core::arrow;
use crate::core::calculus;
use crate::core::ctm::AxisScale;
use crate::core::diagram::Diagram;
use crate::core::math_utilities::{linspace, logspace};
use crate::core::utilities::{self as util, pt2str};
use crate::evaluator::{interp_call, EvalError};
use crate::value::Value;
use crate::xml::{self, El};

pub fn graph(element: &El, diagram: &mut Diagram, parent: &El, outline_group: Option<&El>) {
    let polar = element.borrow().get_or("coordinates", "cartesian") == "polar";
    let bbox = diagram.bbox();
    let domain_attr = element.borrow().get("domain");
    let mut domain = match domain_attr {
        None => {
            if polar {
                [0.0, 2.0 * std::f64::consts::PI]
            } else {
                [bbox[0], bbox[2]]
            }
        }
        Some(attr) => {
            let Some(v) = diagram
                .ctx
                .valid_eval(&attr)
                .ok()
                .and_then(|v| v.as_vec_f64().ok())
            else {
                log::error!("Error in <graph> parsing domain={attr}");
                return;
            };
            let mut d = [v[0], v[1]];
            if d[0] == f64::NEG_INFINITY {
                d[0] = bbox[0];
            }
            if d[1] == f64::INFINITY {
                d[1] = bbox[2];
            }
            d
        }
    };

    util::set_attr(element, "thickness", "2", &mut diagram.ctx);
    let thickness_attr = element.borrow().get_or("thickness", "2");
    let thickness = diagram
        .ctx
        .valid_eval(&thickness_attr)
        .ok()
        .and_then(|v| v.as_num().ok())
        .unwrap_or(2.0);
    util::set_attr(element, "stroke", "blue", &mut diagram.ctx);
    if diagram.output_format() == "tactile" {
        element.borrow_mut().set("stroke", "black");
    }

    let path = xml::new_element("path");
    let id = element.borrow().get("id");
    diagram.add_id(&path, id.as_deref());
    diagram.register_svg_element(element, &path);

    util::add_attr(&path, util::get_1d_attr(element, &mut diagram.ctx));
    if polar {
        let fill = element.borrow().get("fill");
        if let Some(fill) = fill {
            path.borrow_mut().set("fill", &fill);
        }
    }

    let arrows: i64 = element.borrow().get_or("arrows", "0").parse().unwrap_or(0);
    let reverse = element.borrow().get_or("reverse", "no") == "yes";
    let (mut forward, mut backward) = ("marker-end", "marker-start");
    if reverse {
        std::mem::swap(&mut forward, &mut backward);
    }
    let arrow_width = element.borrow().get("arrow-width");
    let arrow_angles = element.borrow().get("arrow-angles");

    let mut arrow_length = 0.0;
    if arrows > 0 {
        let arrow_id = arrow::add_arrowhead_to_path(
            diagram,
            forward,
            &path,
            arrow_width.as_deref(),
            arrow_angles.as_deref(),
        );
        arrow_length = thickness
            * arrow_id
                .and_then(|id| diagram.arrow_lengths.get(&id).copied())
                .unwrap_or(0.0);
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

    let function_attr = element.borrow().get("function").unwrap_or_default();
    let f = match diagram.ctx.valid_eval(&function_attr) {
        Ok(f @ Value::Function(_)) => f,
        _ => {
            log::error!("Error retrieving function in graph: {function_attr}");
            return;
        }
    };

    if !polar {
        // shorten the domain to make room for arrowheads
        if arrows > 1 || (arrows == 1 && reverse) {
            if let Some(new_start) = shortened_endpoint(diagram, &f, domain, arrow_length, true) {
                domain[0] = new_start;
            }
        }
        if arrows > 1 || (arrows == 1 && !reverse) {
            if let Some(new_end) = shortened_endpoint(diagram, &f, domain, arrow_length, false) {
                domain[1] = new_end;
            }
        }
    }

    let n: usize = element.borrow().get_or("N", "100").parse().unwrap_or(100);
    let cmds = if polar {
        polar_path(element, diagram, &f, domain, n)
    } else {
        cartesian_path(diagram, &f, domain, n)
    };

    path.borrow_mut().set("d", &cmds.join(" "));
    if element.borrow().get("cliptobbox").is_none() {
        element.borrow_mut().set("cliptobbox", "yes");
    }
    util::cliptobbox(&path, element, diagram);

    if let Some(outline_group) = outline_group {
        diagram.add_outline(element, &path, outline_group, None, None);
        finish_outline(element, diagram, parent);
    } else if element.borrow().get_or("outline", "no") == "yes"
        || diagram.output_format() == "tactile"
    {
        diagram.add_outline(element, &path, parent, None, None);
        finish_outline(element, diagram, parent);
    } else {
        xml::append(parent, &path);
    }
}

fn finish_outline(element: &El, diagram: &mut Diagram, parent: &El) {
    let stroke = element.borrow().get("stroke");
    let thickness = element.borrow().get("thickness");
    let fill = element.borrow().get_or("fill", "none");
    diagram.finish_outline(element, stroke, thickness, &fill, parent, None);
}

fn eval_f(diagram: &mut Diagram, f: &Value, x: f64) -> Result<f64, EvalError> {
    interp_call(f, &[Value::Num(x)], &mut diagram.ctx)?.as_num()
}

fn shortened_endpoint(
    diagram: &mut Diagram,
    f: &Value,
    domain: [f64; 2],
    arrow_length: f64,
    left: bool,
) -> Option<f64> {
    let (a, b) = if left {
        (domain[0], domain[1])
    } else {
        (domain[1], domain[0])
    };
    let y0 = eval_f(diagram, f, a).ok()?;
    let fp = calculus::derivative(|x| eval_f(diagram, f, x), a, left).ok()?;
    let y1 = fp * (domain[1] - domain[0]) + y0;
    let p0 = diagram.transform([a, y0]);
    let p1 = diagram.transform([b, y1]);
    let diff = [p1[0] - p0[0], p1[1] - p0[1]];
    let length = (diff[0] * diff[0] + diff[1] * diff[1]).sqrt();
    let begin = [
        p0[0] + arrow_length / length * diff[0],
        p0[1] + arrow_length / length * diff[1],
    ];
    Some(diagram.inverse_transform(begin)[0])
}

pub fn cartesian_path(diagram: &mut Diagram, f: &Value, domain: [f64; 2], n: usize) -> Vec<String> {
    // Walk across the horizontal axis connecting points with lines, easing up
    // to singularities and vertical asymptotes by subdivision. Points within a
    // buffer of 3x the height centered on the view are plotted.
    let scales = diagram.get_scales();
    let x_positions = if scales[0] == AxisScale::Log {
        logspace(domain[0].log10(), domain[1].log10(), n)
    } else {
        linspace(domain[0], domain[1], n)
    };

    let bbox = diagram.bbox();
    let mut cmds: Vec<String> = Vec::new();
    let mut next_cmd = "M";
    let (lower, upper) = if scales[1] == AxisScale::Log {
        let bottom = bbox[1].log10();
        let top = bbox[3].log10();
        (10f64.powf(bottom - 3.0), 10f64.powf(top + 3.0))
    } else {
        let height = bbox[3] - bbox[1];
        (bbox[1] - height, bbox[3] + height)
    };
    let mut last_visible = false;

    // Python's numpy floats make 1/0 infinity rather than an error; our
    // evaluator matches, so f can fail only on true domain errors.
    for i in 0..x_positions.len() {
        let x = x_positions[i];
        let dx = if i > 0 { x - x_positions[i - 1] } else { 0.0 };
        let y = match eval_f(diagram, f, x) {
            Ok(y) => y,
            Err(_) => {
                if last_visible {
                    // find the singularity in (x-dx, x) by subdividing 8 times
                    let mut ddx = dx / 2.0;
                    let mut xx = x - ddx;
                    let mut last_good_x = x - dx;
                    for _ in 0..8 {
                        ddx /= 2.0;
                        match eval_f(diagram, f, xx) {
                            Err(_) => {
                                xx -= ddx;
                                continue;
                            }
                            Ok(_) => {
                                last_good_x = xx;
                                xx += ddx;
                            }
                        }
                    }
                    if let Ok(y) = eval_f(diagram, f, last_good_x) {
                        let p = diagram.transform([last_good_x, y]);
                        cmds.push("L".to_string());
                        cmds.push(pt2str(p, " "));
                    }
                }
                last_visible = false;
                next_cmd = "M";
                continue;
            }
        };
        if y > upper || y < lower {
            if last_visible {
                // possibly a vertical asymptote; subdivide into plotting range
                let mut ddx = dx / 2.0;
                let mut xx = x - ddx;
                let mut last_good_x = x - dx;
                for _ in 0..8 {
                    ddx /= 2.0;
                    match eval_f(diagram, f, xx) {
                        Ok(yy) if yy <= upper && yy >= lower => {
                            last_good_x = xx;
                            xx += ddx;
                        }
                        _ => {
                            xx -= ddx;
                        }
                    }
                }
                if let Ok(y) = eval_f(diagram, f, last_good_x) {
                    let p = diagram.transform([last_good_x, y]);
                    cmds.push("L".to_string());
                    cmds.push(pt2str(p, " "));
                }
            }
            last_visible = false;
            next_cmd = "M";
            continue;
        }
        if next_cmd == "M" && x > domain[0] {
            // back up to find the asymptote or the edge of the domain
            let mut ddx = dx / 2.0;
            let mut xx = x - ddx;
            let mut last_good_x = x;
            for _ in 0..8 {
                ddx /= 2.0;
                match eval_f(diagram, f, xx) {
                    Err(_) => {
                        xx += ddx;
                        continue;
                    }
                    Ok(yy) if yy > upper || yy < lower => {
                        xx += ddx;
                        continue;
                    }
                    Ok(_) => {
                        last_good_x = xx;
                        xx -= ddx;
                    }
                }
            }
            if last_good_x < x {
                if let Ok(y) = eval_f(diagram, f, last_good_x) {
                    let p = diagram.transform([last_good_x, y]);
                    cmds.push("M".to_string());
                    cmds.push(pt2str(p, " "));
                    next_cmd = "L";
                }
            }
        }

        let p = diagram.transform([x, y]);
        cmds.push(next_cmd.to_string());
        cmds.push(pt2str(p, " "));
        next_cmd = "L";
        last_visible = y < bbox[3] && y > bbox[1];
    }

    cmds
}

fn polar_path(
    element: &El,
    diagram: &mut Diagram,
    f: &Value,
    domain: [f64; 2],
    n: usize,
) -> Vec<String> {
    let bbox = diagram.bbox();
    let center = [(bbox[0] + bbox[2]) / 2.0, (bbox[1] + bbox[3]) / 2.0];
    let corner = [bbox[2], bbox[3]];
    let r_max = ((corner[0] - center[0]).powi(2) + (corner[1] - center[1]).powi(2)).sqrt();

    let domain = if element.borrow().get_or("domain-degrees", "no") == "yes" {
        [domain[0].to_radians(), domain[1].to_radians()]
    } else {
        domain
    };
    let mut t = domain[0];
    let dt = (domain[1] - domain[0]) / n as f64;
    let mut cmds: Vec<String> = Vec::new();
    let mut next_cmd = "M";
    for _ in 0..=n {
        let r = match eval_f(diagram, f, t) {
            Ok(r) => r,
            Err(_) => {
                next_cmd = "M";
                t += dt;
                continue;
            }
        };
        let p = [r * t.cos(), r * t.sin()];
        let dist = ((p[0] - center[0]).powi(2) + (p[1] - center[1]).powi(2)).sqrt();
        if dist > 2.0 * r_max {
            next_cmd = "M";
            t += dt;
            continue;
        }
        cmds.push(next_cmd.to_string());
        cmds.push(pt2str(diagram.transform(p), " "));
        next_cmd = "L";
        t += dt;
    }
    if element.borrow().get_or("closed", "no") == "yes" {
        cmds.push("Z".to_string());
    }
    cmds
}
