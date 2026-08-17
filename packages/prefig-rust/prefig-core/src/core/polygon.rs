//! Port of prefig/core/polygon.py: polygons, splines, and triangles.

use crate::core::diagram::Diagram;
use crate::core::math_utilities::{linspace, normalize};
use crate::core::spline::{BcType, CubicSpline};
use crate::core::utilities::{self as util, pt2long_str, pt2str};
use crate::core::{arrow, group, label};
use crate::value::{py_str, Function, Value};
use crate::xml::{self, El};
use std::rc::Rc;

type Point = [f64; 2];

/// polygon.parse_points: literal points or generated from a parameter.
pub fn parse_points(element: &El, diagram: &mut Diagram) -> Option<Vec<Point>> {
    let parameter = element.borrow().get("parameter");
    let points_attr = element.borrow().get("points")?;
    match parameter {
        None => {
            let value = diagram.ctx.valid_eval(&points_attr).ok()?;
            points_from_value(&value).or_else(|| {
                log::error!("Error in <polygon> evaluating points={points_attr}");
                None
            })
        }
        Some(parameter) => {
            let (var, expr) = parameter.split_once('=')?;
            let (start, stop) = expr.split_once("..")?;
            let eval_int = |diagram: &mut Diagram, s: &str| -> Option<i64> {
                diagram
                    .ctx
                    .valid_eval(s)
                    .ok()?
                    .as_num()
                    .ok()
                    .map(|n| n as i64)
            };
            let start = eval_int(diagram, start)?;
            let stop = eval_int(diagram, stop)?;
            let mut plot_points = Vec::new();
            for k in start..=stop {
                let _ = diagram
                    .ctx
                    .valid_eval_named(&k.to_string(), Some(var.trim()), true);
                let value = diagram.ctx.valid_eval(&points_attr).ok()?;
                let p = value.as_vec_f64().ok()?;
                plot_points.push([p[0], p[1]]);
            }
            Some(plot_points)
        }
    }
}

fn points_from_value(value: &Value) -> Option<Vec<Point>> {
    let Value::Array(items) = value else {
        return None;
    };
    items
        .iter()
        .map(|i| {
            let v = i.as_vec_f64().ok()?;
            (v.len() >= 2).then(|| [v[0], v[1]])
        })
        .collect()
}

pub fn polygon(
    element: &El,
    diagram: &mut Diagram,
    parent: &El,
    outline_group: Option<&El>,
    points: Option<Vec<Point>>,
    arrow_points: Option<Vec<Point>>,
) {
    if diagram.output_format() == "tactile" {
        if element.borrow().get("stroke").is_some() {
            element.borrow_mut().set("stroke", "black");
        }
        util::set_tactile_fill(element);
    }

    util::set_attr(element, "stroke", "none", &mut diagram.ctx);
    util::set_attr(element, "fill", "none", &mut diagram.ctx);
    util::set_attr(element, "thickness", "2", &mut diagram.ctx);

    let points = match points {
        Some(p) => p,
        None => match parse_points(element, diagram) {
            Some(p) => p,
            None => return,
        },
    };
    let points: Vec<Point> = points.iter().map(|&p| diagram.transform(p)).collect();

    let radius: i64 = element
        .borrow()
        .get_or("corner-radius", "0")
        .parse()
        .unwrap_or(0);
    let closed = element.borrow().get_or("closed", "no");

    let mut d;
    if radius == 0 {
        let mut parts = vec![format!("M {}", pt2str(points[0], " "))];
        for p in &points[1..] {
            parts.push(format!("L {}", pt2str(*p, " ")));
        }
        if closed == "yes" {
            parts.push("Z".to_string());
        }
        d = parts.join(" ");
    } else {
        let radius = radius as f64;
        let mut points = points.clone();
        if closed == "yes" {
            points.push(points[0]);
        }
        let n = points.len() - 1; // number of segments
        let mut cmds = String::new();
        let mut initial_point = [0.0, 0.0];
        for i in 0..n {
            let p = points[i];
            let q = points[i + 1];
            let u = normalize([q[0] - p[0], q[1] - p[1]]);
            let p1 = [p[0] + radius * u[0], p[1] + radius * u[1]];
            let p2 = [q[0] - radius * u[0], q[1] - radius * u[1]];
            if i == 0 {
                if closed == "yes" {
                    cmds = format!("M {}", pt2str(p1, " "));
                    initial_point = p1;
                    cmds += &format!("L {}", pt2str(p2, " "));
                } else {
                    cmds += &format!("M {}", pt2str(p, " "));
                    cmds += &format!("L {}", pt2str(p2, " "));
                }
            }
            if i == n - 1 {
                cmds += &format!("Q {} {}", pt2str(p, " "), pt2str(p1, " "));
                if closed == "yes" {
                    cmds += &format!("L {}", pt2str(p2, " "));
                    cmds += &format!("Q {} {}", pt2str(q, " "), pt2str(initial_point, " "));
                    cmds += "Z";
                } else {
                    cmds += &format!("L{}", pt2str(q, " "));
                }
            }
            if i > 0 && i < n - 1 {
                cmds += &format!("Q{} {}", pt2str(p, " "), pt2str(p1, " "));
                cmds += &format!("L{}", pt2str(p2, " "));
            }
        }
        d = cmds;
    }

    if let Some(arrow_points) = arrow_points {
        let transformed: Vec<Point> = arrow_points.iter().map(|&p| diagram.transform(p)).collect();
        let mut parts = vec![format!(" M {}", pt2str(transformed[0], " "))];
        for p in &transformed[1..] {
            parts.push(format!("L {}", pt2str(*p, " ")));
        }
        d += &parts.join(" ");
    }

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

pub fn polygon_handler(
    element: &El,
    diagram: &mut Diagram,
    parent: &El,
    outline_group: Option<&El>,
) {
    polygon(element, diagram, parent, outline_group, None, None);
}

/// Spline points: 2-D points like polygon's, or a 1-D series of values that
/// Python splines over scalars and later zips with the t values.
fn parse_spline_points(element: &El, diagram: &mut Diagram) -> Option<Vec<Vec<f64>>> {
    if element.borrow().get("parameter").is_some() {
        let points = parse_points(element, diagram)?;
        return Some(points.iter().map(|p| p.to_vec()).collect());
    }
    let attr = element.borrow().get("points")?;
    let value = diagram.ctx.valid_eval(&attr).ok()?;
    if let Value::Array(items) = &value {
        if items.iter().all(|i| matches!(i, Value::Num(_))) {
            // 1-D: a series of scalars, e.g. points="(0,2,1,2,4)"
            let ys = value.as_vec_f64().ok()?;
            return Some(ys.into_iter().map(|y| vec![y]).collect());
        }
    }
    match points_from_value(&value) {
        Some(points) => Some(points.iter().map(|p| p.to_vec()).collect()),
        None => {
            log::error!("Error in <spline> evaluating points={attr}");
            None
        }
    }
}

pub fn spline(element: &El, diagram: &mut Diagram, parent: &El, outline_group: Option<&El>) {
    if element.borrow().get("points").is_none() {
        log::error!("A spline element needs a @points attribute");
        return;
    }
    let Some(point_vecs) = parse_spline_points(element, diagram) else {
        return;
    };

    let t_vals_attr = element.borrow().get("t-values");
    let t_vals: Vec<f64> = match t_vals_attr {
        None => (0..point_vecs.len()).map(|i| i as f64).collect(),
        Some(attr) => match diagram
            .ctx
            .valid_eval(&attr)
            .ok()
            .and_then(|v| v.as_vec_f64().ok())
        {
            Some(v) => v,
            None => return,
        },
    };
    if t_vals.len() != point_vecs.len() {
        log::error!("The number of t values and points must be the same in a spline");
        return;
    }

    let mut bc = BcType::from_name(&element.borrow().get_or("bc-type", "not-a-knot"));
    if element.borrow().get_or("closed", "no") == "yes" {
        bc = BcType::Periodic;
    }

    let cs = Rc::new(CubicSpline::new(&t_vals, &point_vecs, bc));

    if let Some(name) = element.borrow().get("name") {
        let cs_for_ns = cs.clone();
        diagram.ctx.enter_namespace(
            &name,
            Value::Function(Rc::new(Function::Closure(Box::new(move |args, _ctx| {
                let x = args
                    .first()
                    .ok_or_else(|| crate::evaluator::EvalError::new("missing argument"))?
                    .as_num()?;
                let v = cs_for_ns.eval(x);
                Ok(if v.len() == 1 {
                    Value::Num(v[0])
                } else {
                    Value::Array(v.into_iter().map(Value::Num).collect())
                })
            })))),
        );
    }

    let n_attr = element.borrow().get_or("N", "100");
    let n = diagram
        .ctx
        .valid_eval(&n_attr)
        .ok()
        .and_then(|v| v.as_num().ok())
        .unwrap_or(100.0) as usize;
    let domain_attr = element.borrow().get("domain");
    let sample_ts = match domain_attr {
        Some(attr) => {
            let Some(domain) = diagram
                .ctx
                .valid_eval(&attr)
                .ok()
                .and_then(|v| v.as_vec_f64().ok())
            else {
                return;
            };
            element.borrow_mut().set("closed", "no");
            // np.linspace(a, b, N) has N points, N-1 intervals
            linspace(domain[0], domain[1], n - 1)
        }
        None => linspace(t_vals[0], t_vals[t_vals.len() - 1], n - 1),
    };
    let curve: Vec<Point> = sample_ts
        .iter()
        .map(|&t| {
            let v = cs.eval(t);
            if v.len() == 1 {
                [t, v[0]]
            } else {
                [v[0], v[1]]
            }
        })
        .collect();
    element.borrow_mut().tag = "polygon".to_string();

    // optionally move the arrow to arrow-location
    let mut arrow_curve = None;
    if element.borrow().get_or("arrows", "0") == "1" {
        let location_attr = element.borrow().get("arrow-location");
        if let Some(attr) = location_attr {
            if let Some(location) = diagram
                .ctx
                .valid_eval(&attr)
                .ok()
                .and_then(|v| v.as_num().ok())
            {
                let t0 = sample_ts[0].max(location - 0.25);
                let arrow_ts = linspace(t0, location, 9);
                arrow_curve = Some(
                    arrow_ts
                        .iter()
                        .map(|&t| {
                            let v = cs.eval(t);
                            if v.len() == 1 {
                                [t, v[0]]
                            } else {
                                [v[0], v[1]]
                            }
                        })
                        .collect::<Vec<Point>>(),
                );
            }
        }
    }

    polygon(
        element,
        diagram,
        parent,
        outline_group,
        Some(curve),
        arrow_curve,
    );
}

pub fn triangle(element: &El, diagram: &mut Diagram, parent: &El, outline_group: Option<&El>) {
    let vertices_attr = element.borrow().get("vertices").unwrap_or_default();
    let Some(mut vertices) = diagram
        .ctx
        .valid_eval(&vertices_attr)
        .ok()
        .as_ref()
        .and_then(points_from_value)
    else {
        log::error!("Error in <triangle> evaluating vertices={vertices_attr}");
        return;
    };
    if vertices.len() != 3 {
        log::error!("A <triangle> should have exactly 3 vertices");
        return;
    }

    let element_cp = xml::deep_copy(element);
    element.borrow_mut().tag = "group".to_string();
    element.borrow_mut().set("outline", "tactile");

    element_cp.borrow_mut().tag = "polygon".to_string();
    element_cp.borrow_mut().set("closed", "yes");
    let vertices_str = element_cp.borrow().get_or("vertices", "");
    element_cp.borrow_mut().set("points", &vertices_str);
    let stroke = element_cp.borrow().get_or("stroke", "black");
    element_cp.borrow_mut().set("stroke", &stroke);
    xml::append(element, &element_cp);

    if element_cp.borrow().get_or("angle-markers", "no") == "yes" {
        let u = [
            vertices[1][0] - vertices[0][0],
            vertices[1][1] - vertices[0][1],
        ];
        let v = [
            vertices[2][0] - vertices[1][0],
            vertices[2][1] - vertices[1][1],
        ];
        if u[0] * v[1] - u[1] * v[0] > 0.0 {
            vertices.reverse();
        }
        for _ in 0..3 {
            let marker = xml::sub_element(element, "angle-marker");
            let points: Vec<String> = vertices
                .iter()
                .map(|p| format!("({})", pt2long_str(*p, ",")))
                .collect();
            marker
                .borrow_mut()
                .set("points", &format!("({})", points.join(",")));
            vertices.rotate_right(1);
        }
    }

    let labels_attr = element_cp.borrow().get("labels");
    let mut alignments: Vec<Option<String>> = vec![None, None, None];
    let labels: Option<Vec<String>> = labels_attr.map(|l| {
        l.split(',')
            .map(|s| s.trim().to_string())
            .collect::<Vec<_>>()
    });
    if let Some(labels) = &labels {
        if labels.len() < 3 {
            log::error!("A triangle needs three labels");
            return;
        }
        let mut extended = vertices.clone();
        extended.push(vertices[0]);
        extended.push(vertices[1]);
        for i in 1..4 {
            let u = [
                extended[i - 1][0] - extended[i][0],
                extended[i - 1][1] - extended[i][1],
            ];
            let v = [
                extended[i + 1][0] - extended[i][0],
                extended[i + 1][1] - extended[i][1],
            ];
            let direction = [-(u[0] + v[0]), -(u[1] + v[1])];
            alignments[i % 3] = Some(label::get_alignment_from_direction(direction));
        }
    }

    if element_cp.borrow().get_or("show-vertices", "no") == "yes" {
        for i in 0..3 {
            let point_el = xml::sub_element(element, "point");
            point_el
                .borrow_mut()
                .set("p", &pt2long_str(vertices[i], ","));
            let fill = element_cp.borrow().get("point-fill");
            if let Some(fill) = fill {
                point_el.borrow_mut().set("fill", &fill);
            }
            if let (Some(alignment), Some(labels)) = (&alignments[i], &labels) {
                let m_tag = xml::sub_element(&point_el, "m");
                m_tag.borrow_mut().text = Some(labels[i].clone());
                point_el.borrow_mut().set("alignment", alignment);
            }
        }
    } else if let Some(labels) = &labels {
        for i in 0..3 {
            let label_el = xml::sub_element(element, "label");
            label_el
                .borrow_mut()
                .set("anchor", &pt2long_str(vertices[i], ","));
            if let Some(alignment) = &alignments[i] {
                label_el.borrow_mut().set("alignment", alignment);
            }
            let m_tag = xml::sub_element(&label_el, "m");
            m_tag.borrow_mut().text = Some(labels[i].clone());
        }
    }

    group::group(element, diagram, parent, outline_group);
    let _ = py_str(0.0); // keep py_str import for future use
}

fn finish_outline(element: &El, diagram: &mut Diagram, parent: &El) {
    let stroke = element.borrow().get("stroke");
    let thickness = element.borrow().get("thickness");
    let fill = element.borrow().get_or("fill", "none");
    diagram.finish_outline(element, stroke, thickness, &fill, parent, None);
}
