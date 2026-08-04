//! Port of prefig/core/slope_field.py: slope fields and vector fields.

use crate::core::diagram::Diagram;
use crate::core::grid_axes::find_gridspacing;
use crate::core::group;
use crate::core::math_utilities::{distance, length};
use crate::core::utilities::pt2long_str;
use crate::evaluator::interp_call;
use crate::value::Value;
use crate::xml::{self, El};

fn get_function(diagram: &mut Diagram, element: &El) -> Option<Value> {
    let attr = element.borrow().get("function")?;
    match diagram.ctx.valid_eval(&attr) {
        Ok(f @ Value::Function(_)) => Some(f),
        _ => {
            log::error!("Error retrieving slope-field function={attr}");
            None
        }
    }
}

/// An (x, y) pair of grid-spacing triples, each `(min, step, max)`.
type SpacingTriples = ((f64, f64, f64), (f64, f64, f64));

fn get_spacings(diagram: &mut Diagram, element: &El) -> Option<SpacingTriples> {
    let bbox = diagram.bbox();
    let spacings = element.borrow().get("spacings");
    match spacings {
        Some(attr) => {
            let pair = diagram.ctx.valid_eval(&attr).ok().and_then(|v| {
                let Value::Array(items) = v else { return None };
                let rx = items.first()?.as_vec_f64().ok()?;
                let ry = items.get(1)?.as_vec_f64().ok()?;
                Some(((rx[0], rx[1], rx[2]), (ry[0], ry[1], ry[2])))
            });
            if pair.is_none() {
                log::error!("Error parsing slope-field attribute @spacings={attr}");
            }
            pair
        }
        None => Some((
            find_gridspacing([bbox[0], bbox[2]], false),
            find_gridspacing([bbox[1], bbox[3]], false),
        )),
    }
}

fn line_template(element: &El, diagram: &Diagram, arrows_default: bool) -> El {
    let template = xml::new_element("line");
    {
        let mut t = template.borrow_mut();
        if diagram.output_format() == "tactile" {
            t.set("stroke", "black");
        } else {
            t.set("stroke", &element.borrow().get_or("stroke", "blue"));
        }
        t.set("thickness", &element.borrow().get_or("thickness", "2"));
        if arrows_default || element.borrow().get_or("arrows", "no") == "yes" {
            t.set("arrows", "1");
        }
        for attr in ["arrow-width", "arrow-angles"] {
            if let Some(value) = element.borrow().get(attr) {
                t.set(attr, &value);
            }
        }
    }
    template
}

pub fn slope_field(element: &El, diagram: &mut Diagram, parent: &El, outline_group: Option<&El>) {
    let Some(f) = get_function(diagram, element) else {
        return;
    };

    if element.borrow().get("id").is_none() {
        diagram.add_id(element, None);
    }
    element.borrow_mut().tag = "group".to_string();
    if element.borrow().get_or("outline", "no") == "yes" {
        element.borrow_mut().set("outline", "always");
    }

    let template = line_template(element, diagram, false);
    let system = element.borrow().get("system").as_deref() == Some("yes");
    let Some((rx, ry)) = get_spacings(diagram, element) else {
        return;
    };

    let mut x = rx.0;
    while x <= rx.2 {
        let mut y = ry.0;
        while y <= ry.2 {
            let line = xml::deep_copy(&template);
            let (dx, dy);
            if system {
                let change = interp_call(
                    &f,
                    &[
                        Value::Num(0.0),
                        Value::Array(vec![Value::Num(x), Value::Num(y)]),
                    ],
                    &mut diagram.ctx,
                )
                .ok()
                .and_then(|v| v.as_vec_f64().ok())
                .unwrap_or(vec![0.0, 0.0]);
                if length([change[0], change[1]]) > 1e-5 {
                    xml::append(element, &line);
                }
                if change[0].abs() < 1e-8 {
                    dx = 0.0;
                    dy = if change[1] < 0.0 {
                        -ry.1 / 4.0
                    } else {
                        ry.1 / 4.0
                    };
                } else {
                    let slope = change[1] / change[0];
                    let mut ddx = rx.1 / 4.0;
                    let mut ddy = slope * ddx;
                    if ddy.abs() > ry.1 / 4.0 {
                        ddy = ry.1 / 4.0;
                        ddx = ddy / slope;
                    }
                    if change[0] * ddx < 0.0 {
                        ddx *= -1.0;
                        ddy *= -1.0;
                    }
                    dx = ddx;
                    dy = ddy;
                }
            } else {
                let slope = interp_call(&f, &[Value::Num(x), Value::Num(y)], &mut diagram.ctx)
                    .ok()
                    .and_then(|v| v.as_num().ok());
                match slope {
                    None => {
                        // Python: ZeroDivisionError -> a vertical dash
                        dx = 0.0;
                        dy = ry.1 / 4.0;
                    }
                    Some(slope) if slope.is_infinite() || slope.is_nan() => {
                        dx = 0.0;
                        dy = ry.1 / 4.0;
                    }
                    Some(slope) => {
                        let mut ddx = rx.1 / 4.0;
                        let mut ddy = slope * ddx;
                        if ddy.abs() > ry.1 / 4.0 {
                            ddy = ry.1 / 4.0;
                            ddx = ddy / slope;
                        }
                        if ddx < 0.0 {
                            ddx *= -1.0;
                            ddy *= -1.0;
                        }
                        dx = ddx;
                        dy = ddy;
                    }
                }
                xml::append(element, &line);
            }
            line.borrow_mut()
                .set("p1", &pt2long_str([x - dx, y - dy], ","));
            line.borrow_mut()
                .set("p2", &pt2long_str([x + dx, y + dy], ","));
            y += ry.1;
        }
        x += rx.1;
    }

    group::group(element, diagram, parent, outline_group);
}

pub fn vector_field(element: &El, diagram: &mut Diagram, parent: &El, outline_group: Option<&El>) {
    let Some(f) = get_function(diagram, element) else {
        return;
    };

    if element.borrow().get("id").is_none() {
        diagram.add_id(element, None);
    }
    element.borrow_mut().tag = "group".to_string();
    if element.borrow().get_or("outline", "no") == "yes" {
        element.borrow_mut().set("outline", "always");
    }

    let template = line_template(element, diagram, true);

    let mut field_data: Vec<([f64; 2], Vec<f64>)> = Vec::new();
    let mut scale_factor;

    let curve_attr = element.borrow().get("curve");
    if let Some(curve_attr) = curve_attr {
        let Ok(curve) = diagram.ctx.valid_eval(&curve_attr) else {
            return;
        };
        let domain_attr = element.borrow().get("domain");
        let Some(domain) = domain_attr
            .as_deref()
            .and_then(|a| diagram.ctx.valid_eval(a).ok())
            .and_then(|v| v.as_vec_f64().ok())
        else {
            log::error!("A @domain is needed if adding a vector field to a curve");
            return;
        };
        let n_attr = element.borrow().get("N");
        let Some(n) = n_attr
            .as_deref()
            .and_then(|a| diagram.ctx.valid_eval(a).ok())
            .and_then(|v| v.as_num().ok())
        else {
            log::error!("A @N is needed if adding a vector field to a curve");
            return;
        };
        let n = n as usize;

        let mut t = domain[0];
        // is f a function of t or of (x, y)?
        let one_variable = interp_call(&f, &[Value::Num(t)], &mut diagram.ctx).is_ok();
        let dt = (domain[1] - domain[0]) / (n - 1) as f64;
        for _ in 0..n {
            let Ok(position) = interp_call(&curve, &[Value::Num(t)], &mut diagram.ctx)
                .and_then(|v| v.as_vec_f64())
            else {
                t += dt;
                continue;
            };
            let value = if one_variable {
                interp_call(&f, &[Value::Num(t)], &mut diagram.ctx)
            } else {
                interp_call(
                    &f,
                    &[Value::Num(position[0]), Value::Num(position[1])],
                    &mut diagram.ctx,
                )
            };
            if let Ok(v) = value.and_then(|v| v.as_vec_f64()) {
                field_data.push(([position[0], position[1]], v));
            }
            t += dt;
        }
        let scale_attr = element.borrow().get_or("scale", "1");
        scale_factor = diagram
            .ctx
            .valid_eval(&scale_attr)
            .ok()
            .and_then(|v| v.as_num().ok())
            .unwrap_or(1.0);
    } else {
        let Some((rx, ry)) = get_spacings(diagram, element) else {
            return;
        };

        let mut max_scale = 0.0f64;
        let exponent_attr = element.borrow().get_or("exponent", "1");
        let exponent = diagram
            .ctx
            .valid_eval(&exponent_attr)
            .ok()
            .and_then(|v| v.as_num().ok())
            .unwrap_or(1.0);
        let mut x = rx.0;
        while x <= rx.2 {
            let mut y = ry.0;
            while y <= ry.2 {
                let Ok(f_value) =
                    interp_call(&f, &[Value::Num(x), Value::Num(y)], &mut diagram.ctx)
                        .and_then(|v| v.as_vec_f64())
                else {
                    y += ry.1;
                    continue;
                };
                if f_value.iter().any(|v| v.is_nan()) {
                    y += ry.1;
                    continue;
                }
                if f_value.len() != 2 {
                    log::error!("Only two-dimensional vector fields are supported");
                    return;
                }
                let norm = length([f_value[0], f_value[1]]);
                let f_value = if norm < 1e-10 {
                    vec![0.0, 0.0]
                } else {
                    // scale the length by length**exponent to promote
                    // shorter vectors
                    let factor = norm.powf(exponent) / norm;
                    vec![factor * f_value[0], factor * f_value[1]]
                };
                max_scale = max_scale
                    .max((f_value[0] / rx.1).abs())
                    .max((f_value[1] / ry.1).abs());
                field_data.push(([x, y], f_value));
                y += ry.1;
            }
            x += rx.1;
        }

        scale_factor = 1f64.min(0.75 / max_scale);
        let scale_attr = element.borrow().get("scale");
        if let Some(scale_attr) = scale_attr {
            if let Some(scale) = diagram
                .ctx
                .valid_eval(&scale_attr)
                .ok()
                .and_then(|v| v.as_num().ok())
            {
                scale_factor = scale;
            }
        }
    }

    for (p, v) in &field_data {
        let v = [scale_factor * v[0], scale_factor * v[1]];
        let tail = *p;
        let tip = [p[0] + v[0], p[1] + v[1]];
        let p0 = diagram.transform(tail);
        let p1 = diagram.transform(tip);
        if distance(p0, p1) < 2.0 {
            continue;
        }
        let line_el = xml::deep_copy(&template);
        line_el.borrow_mut().set("p1", &pt2long_str(tail, ","));
        line_el.borrow_mut().set("p2", &pt2long_str(tip, ","));
        xml::append(element, &line_el);
    }

    group::group(element, diagram, parent, outline_group);
}
