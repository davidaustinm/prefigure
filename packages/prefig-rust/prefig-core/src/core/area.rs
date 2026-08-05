//! Port of prefig/core/area.py: regions under and between graphs.

use crate::core::diagram::Diagram;
use crate::core::utilities::{self as util, pt2str};
use crate::evaluator::interp_call;
use crate::value::Value;
use crate::xml::{self, El};

pub fn area_between_curves(
    element: &El,
    diagram: &mut Diagram,
    parent: &El,
    outline_group: Option<&El>,
) {
    let polar = element.borrow().get_or("coordinates", "cartesian") == "polar";
    util::set_attr(element, "stroke", "black", &mut diagram.ctx);
    util::set_attr(element, "fill", "lightgray", &mut diagram.ctx);
    util::set_attr(element, "thickness", "2", &mut diagram.ctx);
    if diagram.output_format() == "tactile" {
        element.borrow_mut().set("stroke", "black");
        util::set_tactile_fill(element);
    }

    let functions_attr = element.borrow().get("functions");
    let (f, g) = if let Some(attr) = functions_attr {
        let pair = diagram.ctx.valid_eval(&attr).ok().and_then(|v| {
            let Value::Array(items) = v else { return None };
            (items.len() == 2).then(|| (items[0].clone(), items[1].clone()))
        });
        match pair {
            Some(pair) => pair,
            None => {
                log::error!("Error in <area> parsing functions={attr}");
                return;
            }
        }
    } else {
        let f_attr = element.borrow().get("function1").unwrap_or_default();
        let Ok(f) = diagram.ctx.valid_eval(&f_attr) else {
            log::error!("Error in <area> defining function1={f_attr}");
            return;
        };
        let g_attr = element.borrow().get("function2").unwrap_or_default();
        let Ok(g) = diagram.ctx.valid_eval(&g_attr) else {
            log::error!("Error in <area> defining function2={g_attr}");
            return;
        };
        (f, g)
    };

    let n: usize = element.borrow().get_or("N", "100").parse().unwrap_or(100);

    let bbox = diagram.bbox();
    let domain_attr = element.borrow().get("domain");
    let mut domain = match domain_attr {
        None => [bbox[0], bbox[2]],
        Some(attr) => {
            let Some(v) = diagram
                .ctx
                .valid_eval(&attr)
                .ok()
                .and_then(|v| v.as_vec_f64().ok())
            else {
                log::error!("Error in <area> parsing domain={attr}");
                return;
            };
            [v[0], v[1]]
        }
    };
    if element.borrow().get_or("domain-degrees", "no") == "yes" {
        domain = [domain[0].to_radians(), domain[1].to_radians()];
    }

    let eval_point = |diagram: &mut Diagram, func: &Value, x: f64| -> Option<[f64; 2]> {
        let y = interp_call(func, &[Value::Num(x)], &mut diagram.ctx)
            .ok()?
            .as_num()
            .ok()?;
        Some(if polar {
            diagram.transform([y * x.cos(), y * x.sin()])
        } else {
            diagram.transform([x, y])
        })
    };

    let dx = (domain[1] - domain[0]) / n as f64;
    let mut x = domain[0];
    let Some(p) = eval_point(diagram, &f, x) else {
        return;
    };
    let mut cmds = vec![format!("M {}", pt2str(p, " "))];
    for _ in 0..=n {
        if let Some(p) = eval_point(diagram, &f, x) {
            cmds.push(format!("L {}", pt2str(p, " ")));
        }
        x += dx;
    }
    for _ in 0..=n {
        x -= dx;
        if let Some(p) = eval_point(diagram, &g, x) {
            cmds.push(format!("L {}", pt2str(p, " ")));
        }
    }
    cmds.push("Z".to_string());

    let path = xml::new_element("path");
    let id = element.borrow().get("id");
    diagram.add_id(&path, id.as_deref());
    diagram.register_svg_element(element, &path);

    path.borrow_mut().set("d", &cmds.join(" "));
    let attrs = util::get_2d_attr(element, diagram);
    util::add_attr(&path, attrs);

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

pub fn area_under_curve(
    element: &El,
    diagram: &mut Diagram,
    parent: &El,
    outline_group: Option<&El>,
) {
    let function = element.borrow().get_or("function", "none");
    element.borrow_mut().set("function1", &function);
    let _ = diagram.ctx.define("__zero(x) = 0");
    element.borrow_mut().set("function2", "__zero");
    area_between_curves(element, diagram, parent, outline_group);
}

fn finish_outline(element: &El, diagram: &mut Diagram, parent: &El) {
    let stroke = element.borrow().get("stroke");
    let thickness = element.borrow().get("thickness");
    let fill = element.borrow().get_or("fill", "none");
    diagram.finish_outline(element, stroke, thickness, &fill, parent);
}
