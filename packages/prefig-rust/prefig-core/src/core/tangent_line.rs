//! Port of prefig/core/tangent_line.py.

use crate::core::calculus;
use crate::core::ctm::AxisScale;
use crate::core::diagram::Diagram;
use crate::core::line::{infinite_line, mk_line};
use crate::core::math_utilities::{linspace, logspace};
use crate::core::utilities::{self as util, pt2str};
use crate::evaluator::interp_call;
use crate::value::{Function, Value};
use crate::xml::{self, El};
use std::rc::Rc;

pub fn tangent(element: &El, diagram: &mut Diagram, parent: &El, outline_group: Option<&El>) {
    let function_attr = element.borrow().get("function").unwrap_or_default();
    let f = match diagram.ctx.valid_eval(&function_attr) {
        Ok(f @ Value::Function(_)) => f,
        _ => {
            log::error!("Error retrieving tangent-line attribute @function={function_attr}");
            return;
        }
    };

    let point_attr = element.borrow().get("point").unwrap_or_default();
    let Ok(a) = diagram.ctx.valid_eval(&point_attr).and_then(|v| v.as_num()) else {
        log::error!("Error parsing tangent-line attribute @point={point_attr}");
        return;
    };

    let Ok(y0) = interp_call(&f, &[Value::Num(a)], &mut diagram.ctx).and_then(|v| v.as_num())
    else {
        log::error!("Error evaluating the function in <tangent-line>");
        return;
    };
    let Ok(m) = calculus::derivative(
        |x| interp_call(&f, &[Value::Num(x)], &mut diagram.ctx)?.as_num(),
        a,
        true,
    ) else {
        log::error!("Error differentiating the function in <tangent-line>");
        return;
    };

    let tangent = move |x: f64| y0 + m * (x - a);

    if let Some(name) = element.borrow().get("name") {
        diagram.ctx.enter_namespace(
            &name,
            Value::Function(Rc::new(Function::Closure(Box::new(move |args, _ctx| {
                let x = args
                    .first()
                    .ok_or_else(|| crate::evaluator::EvalError::new("missing argument"))?
                    .as_num()?;
                Ok(Value::Num(tangent(x)))
            })))),
        );
    }

    let bbox = diagram.bbox();
    let domain_attr = element.borrow().get("domain");
    let domain = match &domain_attr {
        None => [bbox[0], bbox[2]],
        Some(attr) => {
            let Some(v) = diagram
                .ctx
                .valid_eval(attr)
                .ok()
                .and_then(|v| v.as_vec_f64().ok())
            else {
                log::error!("Error parsing tangent-line domain={attr}");
                return;
            };
            [v[0], v[1]]
        }
    };

    let scales = diagram.get_scales();
    let (x1, x2) = (domain[0], domain[1]);
    let line_el = if scales == [AxisScale::Linear, AxisScale::Linear] {
        let p1 = [x1, tangent(x1)];
        let p2 = [x2, tangent(x2)];
        let (p1, p2) = if element.borrow().get_or("infinite", "") == "yes" || domain_attr.is_none()
        {
            match infinite_line(p1, p2, diagram, None) {
                Some(pair) => pair,
                None => return,
            }
        } else {
            (p1, p2)
        };
        let id = element.borrow().get("id");
        mk_line(p1, p2, diagram, id.as_deref(), None, true)
    } else {
        let line_el = xml::new_element("path");
        let x_positions = if scales[0] == AxisScale::Log {
            // Python has np.log(x2) here (natural log) — a quirk kept for parity
            logspace(x1.log10(), x2.ln(), 100)
        } else {
            linspace(x1, x2, 100)
        };
        let mut cmds: Vec<String> = Vec::new();
        let mut next_cmd = "M";
        for x in x_positions {
            let y = tangent(x);
            if y < 0.0 && scales[1] == AxisScale::Log {
                next_cmd = "M";
                continue;
            }
            cmds.push(next_cmd.to_string());
            cmds.push(pt2str(diagram.transform([x, y]), " "));
            next_cmd = "L";
        }
        line_el.borrow_mut().set("d", &cmds.join(" "));
        line_el
    };

    diagram.register_svg_element(element, &line_el);
    if diagram.output_format() == "tactile" {
        element.borrow_mut().set("stroke", "black");
    } else {
        util::set_attr(element, "stroke", "red", &mut diagram.ctx);
    }
    util::set_attr(element, "thickness", "2", &mut diagram.ctx);

    util::add_attr(&line_el, util::get_1d_attr(element, &mut diagram.ctx));
    element.borrow_mut().set("cliptobbox", "yes");
    util::cliptobbox(&line_el, element, diagram);

    if let Some(outline_group) = outline_group {
        diagram.add_outline(element, &line_el, outline_group, None);
        finish_outline(element, diagram, parent);
    } else if element.borrow().get_or("outline", "no") == "yes"
        || diagram.output_format() == "tactile"
    {
        diagram.add_outline(element, &line_el, parent, None);
        finish_outline(element, diagram, parent);
    } else {
        xml::append(parent, &line_el);
    }
}

fn finish_outline(element: &El, diagram: &mut Diagram, parent: &El) {
    let stroke = element.borrow().get("stroke");
    let thickness = element.borrow().get("thickness");
    let fill = element.borrow().get_or("fill", "none");
    diagram.finish_outline(element, stroke, thickness, &fill, parent);
}
