//! Port of prefig/core/riemann_sum.py.

use crate::core::diagram::Diagram;
use crate::core::math_utilities::{fmt_g, linspace};
use crate::core::{group, label};
use crate::evaluator::interp_call;
use crate::value::{py_str, Function, Value};
use crate::xml::{self, El};
use std::rc::Rc;

pub fn riemann_sum(element: &El, diagram: &mut Diagram, parent: &El, outline_group: Option<&El>) {
    let id = element.borrow().get("id");
    diagram.add_id(element, id.as_deref());
    let element_id = element.borrow().get_or("id", "none");

    // the author may give a partition and samples
    let mut partition: Option<Vec<f64>> = None;
    let mut samples: Option<Vec<f64>> = None;
    let partition_attr = element.borrow().get("partition");
    if let Some(attr) = partition_attr {
        partition = diagram
            .ctx
            .valid_eval(&attr)
            .ok()
            .and_then(|v| v.as_vec_f64().ok());
    }
    let samples_attr = element.borrow().get("samples");
    if let Some(attr) = samples_attr {
        samples = diagram
            .ctx
            .valid_eval(&attr)
            .ok()
            .and_then(|v| v.as_vec_f64().ok());
    }

    let partition = match partition {
        Some(p) => p,
        None => {
            let bbox = diagram.bbox();
            let domain_attr = element.borrow().get("domain");
            let domain = match domain_attr {
                None => [bbox[0], bbox[2]],
                Some(attr) => {
                    let Some(v) = diagram
                        .ctx
                        .valid_eval(&attr)
                        .ok()
                        .and_then(|v| v.as_vec_f64().ok())
                    else {
                        log::error!("Error in <riemann-sum> parsing domain={attr}");
                        return;
                    };
                    [v[0], v[1]]
                }
            };
            let n_attr = element.borrow().get("N");
            let Some(n) = n_attr.as_deref().and_then(|a| a.parse::<usize>().ok()) else {
                log::error!("Error in <riemann-sum> setting N={n_attr:?}");
                return;
            };
            linspace(domain[0], domain[1], n)
        }
    };
    let n = partition.len() - 1;

    let rule = element.borrow().get("rule").unwrap_or_else(|| {
        if samples.is_none() {
            "left".to_string()
        } else {
            "user-defined".to_string()
        }
    });

    let samples: Vec<f64> = match rule.as_str() {
        "left" => partition[..n].to_vec(),
        "right" => partition[1..].to_vec(),
        "midpoint" => partition
            .windows(2)
            .map(|w| (w[0] + w[1]) / 2.0)
            .collect(),
        _ => samples.unwrap_or_default(),
    };

    let function_attr = element.borrow().get("function").unwrap_or_default();
    let Ok(f) = diagram.ctx.valid_eval(&function_attr) else {
        log::error!("Error in <riemann-sum> retrieving function={function_attr}");
        return;
    };

    let mut annotation = None;
    let mut interval_text = None;
    if element.borrow().get_or("annotate", "no") == "yes" {
        let a = xml::new_element("annotation");
        for attrib in ["id", "text", "circular", "sonify", "speech"] {
            if let Some(value) = element.borrow().get(attrib) {
                a.borrow_mut().set(attrib, &value);
            }
        }
        let a_id = a.borrow().get("id");
        if let Some(a_id) = a_id {
            a.borrow_mut().set("ref", &a_id);
        }
        for attrib in ["text", "speech"] {
            let value = a.borrow().get(attrib);
            if let Some(value) = value {
                let evaluated = label::evaluate_text(&value, &mut diagram.ctx);
                a.borrow_mut().set(attrib, &evaluated);
            }
        }
        diagram.push_to_annotation_branch(a.clone());
        interval_text = element.borrow().get("subinterval-text");
        annotation = Some(a);
    }

    // change this element to a group and add area elements below it
    element.borrow_mut().tag = "group".to_string();
    let outline = element.borrow().get("outline");
    match outline {
        None => element.borrow_mut().set("outline", "tactile"),
        Some(o) if o == "yes" => element.borrow_mut().set("outline", "always"),
        _ => {}
    }
    let stroke = element.borrow().get_or("stroke", "black");
    let mut fill = element.borrow().get_or("fill", "none");
    let thickness = element.borrow().get_or("thickness", "2");
    let miterlimit = element.borrow().get("miterlimit");
    if diagram.output_format() == "tactile" && fill != "none" {
        fill = "lightgray".to_string();
    }

    for interval_num in 0..n {
        let left = partition[interval_num];
        let right = partition[interval_num + 1];
        diagram
            .ctx
            .enter_namespace("_interval", Value::Num(interval_num as f64));
        diagram
            .ctx
            .enter_namespace("_left", Value::Str(fmt_g(left)));
        diagram
            .ctx
            .enter_namespace("_right", Value::Str(fmt_g(right)));
        let area = xml::sub_element(element, "area-under-curve");
        {
            let mut a = area.borrow_mut();
            a.set("id", &format!("{element_id}_{interval_num}"));
            a.set(
                "domain",
                &format!("({},{})", py_str(left), py_str(right)),
            );
            a.set("stroke", &stroke);
            a.set("fill", &fill);
            a.set("thickness", &thickness);
            if let Some(miterlimit) = &miterlimit {
                a.set("miterlimit", miterlimit);
            }
        }

        match rule.as_str() {
            "left" | "right" | "midpoint" | "user-defined" | "upper" | "lower" => {
                let y_value = match rule.as_str() {
                    "upper" | "lower" => {
                        let xs = linspace(left, right, 100);
                        let ys: Vec<f64> = xs
                            .iter()
                            .filter_map(|&x| {
                                interp_call(&f, &[Value::Num(x)], &mut diagram.ctx)
                                    .ok()?
                                    .as_num()
                                    .ok()
                            })
                            .collect();
                        if rule == "upper" {
                            ys.into_iter().fold(f64::NEG_INFINITY, f64::max)
                        } else {
                            ys.into_iter().fold(f64::INFINITY, f64::min)
                        }
                    }
                    _ => interp_call(
                        &f,
                        &[Value::Num(samples[interval_num])],
                        &mut diagram.ctx,
                    )
                    .ok()
                    .and_then(|v| v.as_num().ok())
                    .unwrap_or(0.0),
                };
                diagram
                    .ctx
                    .enter_namespace("_height", Value::Str(fmt_g(y_value)));
                let function_name = format!("__constant_{interval_num}");
                diagram.ctx.enter_namespace(
                    &function_name,
                    Value::Function(Rc::new(Function::Closure(Box::new(
                        move |_args, _ctx| Ok(Value::Num(y_value)),
                    )))),
                );
                area.borrow_mut().set("function", &function_name);
                area.borrow_mut().set("N", "1");
            }
            "trapezoidal" => {
                area.borrow_mut().set("function", &function_attr);
                area.borrow_mut().set("N", "1");
            }
            "simpsons" => {
                let h = (right - left) / 2.0;
                let mid = left + h;
                let mut eval_at = |x: f64| {
                    interp_call(&f, &[Value::Num(x)], &mut diagram.ctx)
                        .ok()
                        .and_then(|v| v.as_num().ok())
                        .unwrap_or(0.0)
                };
                let y0 = eval_at(left);
                let y1 = eval_at(mid);
                let y2 = eval_at(right);
                let c = y1;
                let a_coef = (y0 + y2 - 2.0 * y1) / (2.0 * h * h);
                let b_coef = (y2 - y0) / (2.0 * h);
                let function_name = format!("__parabola_{interval_num}");
                diagram.ctx.enter_namespace(
                    &function_name,
                    Value::Function(Rc::new(Function::Closure(Box::new(
                        move |args, _ctx| {
                            let x = args
                                .first()
                                .ok_or_else(|| {
                                    crate::evaluator::EvalError::new("missing argument")
                                })?
                                .as_num()?;
                            Ok(Value::Num(
                                a_coef * (x - mid).powi(2) + b_coef * (x - mid) + c,
                            ))
                        },
                    )))),
                );
                area.borrow_mut().set("function", &function_name);
                area.borrow_mut().set("N", "100");
            }
            _ => {}
        }

        if let (Some(interval_text), Some(annotation)) = (&interval_text, &annotation) {
            let interval_annotation = xml::sub_element(annotation, "annotation");
            let area_id = area.borrow().get_or("id", "none");
            interval_annotation.borrow_mut().set("ref", &area_id);
            let evaluated = label::evaluate_text(interval_text, &mut diagram.ctx);
            interval_annotation.borrow_mut().set("text", &evaluated);
        }
    }

    group::group(element, diagram, parent, outline_group);
    if annotation.is_some() {
        diagram.pop_from_annotation_branch();
    }
}
