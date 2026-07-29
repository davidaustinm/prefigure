//! The transform-element handlers from prefig/core/CTM.py (the CTM type
//! itself lives in ctm.rs).

use crate::core::diagram::Diagram;
use crate::value::Value;
use crate::xml::El;

pub fn transform_group(element: &El, diagram: &mut Diagram, root: &El, outline_group: Option<&El>) {
    diagram.ctm().push();
    element.borrow_mut().tag = "group".to_string();
    diagram.parse(element, root, outline_group);
    diagram.ctm().pop();
}

pub fn transform_center(
    _element: &El,
    diagram: &mut Diagram,
    _root: &El,
    _outline_group: Option<&El>,
) {
    let bbox = diagram.bbox();
    diagram
        .ctm()
        .translate((bbox[0] + bbox[2]) / 2.0, (bbox[1] + bbox[3]) / 2.0);
}

fn eval_by(diagram: &mut Diagram, element: &El, tag: &str) -> Option<Vec<f64>> {
    let attr = element.borrow().get("by");
    match attr
        .as_deref()
        .and_then(|a| diagram.ctx.valid_eval(a).ok())
        .and_then(|v| v.as_vec_f64().ok())
    {
        Some(v) => Some(v),
        None => {
            log::error!("Error in <{tag}> parsing by={attr:?}");
            None
        }
    }
}

pub fn transform_translate(
    element: &El,
    diagram: &mut Diagram,
    _root: &El,
    _outline_group: Option<&El>,
) {
    if let Some(p) = eval_by(diagram, element, "translate") {
        diagram.ctm().translate(p[0], p[1]);
    }
}

pub fn transform_translate3d(
    element: &El,
    diagram: &mut Diagram,
    _root: &El,
    _outline_group: Option<&El>,
) {
    if let Some(p) = eval_by(diagram, element, "translate3d") {
        diagram.ctm().translate3d(p[0], p[1], p[2]);
    }
}

pub fn transform_basis(
    element: &El,
    diagram: &mut Diagram,
    _root: &El,
    _outline_group: Option<&El>,
) {
    let attr = element.borrow().get("basis");
    let vectors = attr
        .as_deref()
        .and_then(|a| diagram.ctx.valid_eval(a).ok())
        .and_then(|v| {
            let Value::Array(items) = v else { return None };
            let v1 = items.first()?.as_vec_f64().ok()?;
            let v2 = items.get(1)?.as_vec_f64().ok()?;
            Some((v1, v2))
        });
    let Some((v1, v2)) = vectors else {
        log::error!("Error in <change-basis> parsing basis={attr:?}");
        return;
    };
    // the basis vectors are the columns
    let matrix = [[v1[0], v2[0]], [v1[1], v2[1]]];
    diagram.ctm().apply_matrix(matrix);
}

pub fn transform_rotate(
    element: &El,
    diagram: &mut Diagram,
    _root: &El,
    _outline_group: Option<&El>,
) {
    let by_attr = element.borrow().get("by");
    let Some(angle) = by_attr
        .as_deref()
        .and_then(|a| diagram.ctx.valid_eval(a).ok())
        .and_then(|v| v.as_num().ok())
    else {
        log::error!("Error in <rotate> parsing by={by_attr:?}");
        return;
    };
    let about_attr = element.borrow().get_or("about", "(0,0)");
    let Some(p) = diagram
        .ctx
        .valid_eval(&about_attr)
        .ok()
        .and_then(|v| v.as_vec_f64().ok())
    else {
        log::error!("Error in <rotate> parsing about={about_attr}");
        return;
    };

    let degrees = element.borrow().get_or("degrees", "yes") == "yes";
    let ctm = diagram.ctm();
    ctm.translate(p[0], p[1]);
    ctm.rotate(angle, degrees);
    ctm.translate(-p[0], -p[1]);
}

pub fn transform_scale(
    element: &El,
    diagram: &mut Diagram,
    _root: &El,
    _outline_group: Option<&El>,
) {
    let by_attr = element.borrow().get("by");
    let Some(value) = by_attr
        .as_deref()
        .and_then(|a| diagram.ctx.valid_eval(a).ok())
    else {
        log::error!("Error in <scale> parsing by={by_attr:?}");
        return;
    };
    match &value {
        Value::Array(_) => {
            let s = value.as_vec_f64().unwrap_or(vec![1.0, 1.0]);
            diagram.ctm().scale(s[0], s[1]);
        }
        _ => {
            let s = value.as_num().unwrap_or(1.0);
            diagram.ctm().scale(s, s);
        }
    }
}

pub fn transform_scale3d(
    element: &El,
    diagram: &mut Diagram,
    _root: &El,
    _outline_group: Option<&El>,
) {
    if let Some(s) = eval_by(diagram, element, "scale3d") {
        diagram.ctm().scale3d(s[0], s[1], s[2]);
    }
}

pub fn set_eye(element: &El, diagram: &mut Diagram, _root: &El, _outline_group: Option<&El>) {
    let attr = element.borrow().get("eye");
    let Some(mut eye) = attr
        .as_deref()
        .and_then(|a| diagram.ctx.valid_eval(a).ok())
        .and_then(|v| v.as_vec_f64().ok())
    else {
        log::error!("Error in <set-eye> parsing eye={attr:?}");
        return;
    };
    if eye.len() == 3 {
        eye.push(0.0);
    }
    diagram.ctm().set_eye(&eye);
}
