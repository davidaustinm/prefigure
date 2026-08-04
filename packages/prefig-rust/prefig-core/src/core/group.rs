//! Port of prefig/core/group.py: groups for annotation and shared outlines,
//! including the transform attribute.

use crate::core::ctm;
use crate::core::diagram::Diagram;
use crate::value::Value;
use crate::xml::{self, El};

pub fn group(element: &El, diagram: &mut Diagram, parent: &El, outline_group: Option<&El>) {
    let outline = element.borrow().get("outline");
    let tactile = diagram.output_format() == "tactile";
    let transform = element.borrow().get("transform");
    let id = element.borrow().get("id");
    diagram.add_id(element, id.as_deref());
    let element_id = element.borrow().get_or("id", "none");

    if outline_group.is_none()
        && (outline.as_deref() == Some("always")
            || outline.as_deref() == Some(diagram.output_format()))
    {
        // a group for the outlines, then a group for the strokes
        let outline_g = xml::sub_element(parent, "g");
        outline_g.borrow_mut().set("data-outline", "yes");
        diagram.add_id(&outline_g, Some(&format!("{element_id}-outline")));

        let group = xml::sub_element(parent, "g");
        group.borrow_mut().set("id", &element_id);
        diagram.register_svg_element(element, &group);

        if let Some(transform) = &transform {
            process_transform(diagram, transform, &group, tactile);
            let tform = group.borrow().get("transform");
            if let Some(tform) = tform {
                outline_g.borrow_mut().set("transform", &tform);
            }
        }

        diagram.parse(element, &group, Some(&outline_g));
        outline_g.borrow_mut().pop_attr("data-outline");
        if transform.is_some() {
            clean_up_transform(diagram, tactile);
        }
        return;
    }

    let group = xml::sub_element(parent, "g");
    group.borrow_mut().set("id", &element_id);
    diagram.register_svg_element(element, &group);

    if let Some(transform) = &transform {
        process_transform(diagram, transform, &group, tactile);
    }
    let child_outline_group = outline_group.map(|og| {
        let sub = xml::sub_element(og, "g");
        diagram.add_id(&sub, Some(&format!("{element_id}-outline")));
        sub.borrow_mut().set("data-outline", "yes");
        let tform = group.borrow().get("transform");
        if let Some(tform) = tform {
            sub.borrow_mut().set("transform", &tform);
        }
        sub
    });
    diagram.parse(element, &group, child_outline_group.as_ref());
    if let Some(og) = &child_outline_group {
        og.borrow_mut().pop_attr("data-outline");
    }
    if transform.is_some() {
        clean_up_transform(diagram, tactile);
    }
}

fn eval_vec(diagram: &mut Diagram, expr: &str) -> Option<Vec<f64>> {
    diagram.ctx.valid_eval(expr).ok()?.as_vec_f64().ok()
}

fn process_transform(diagram: &mut Diagram, transform: &str, group: &El, tactile: bool) {
    if tactile {
        diagram.ctm().push();
    }
    let transform = transform.trim();
    let Some(index) = transform.find('(') else {
        return;
    };
    let args = &transform[index..];

    if transform.starts_with("translate") {
        let Some(vec) = eval_vec(diagram, args) else {
            return;
        };
        if tactile {
            diagram.ctm().translate(vec[0], vec[1]);
        } else {
            let a = diagram.transform([vec[0], vec[1]]);
            let b = diagram.transform([0.0, 0.0]);
            let t_string = ctm::translatestr(a[0] - b[0], a[1] - b[1]);
            group.borrow_mut().set("transform", &t_string);
        }
    }

    if transform.starts_with("reflect") {
        let Some(data) = eval_vec_or_rows(diagram, args) else {
            return;
        };
        let (q1, q2) = match data {
            VecOrRows::Rows(rows) if rows.len() == 2 => {
                ([rows[0][0], rows[0][1]], [rows[1][0], rows[1][1]])
            }
            VecOrRows::Flat(v) if v.len() == 3 => {
                let (a, b, c) = (v[0], v[1], v[2]);
                if b.abs() < 1e-8 {
                    ([c / a, 0.0], [c / a, 1.0])
                } else {
                    ([0.0, c / b], [1.0, (c - a) / b])
                }
            }
            _ => return,
        };
        let p1 = diagram.transform(q1);
        let p2 = diagram.transform(q2);
        let diff = [p1[0] - p2[0], p1[1] - p2[1]];
        let angle = diff[1].atan2(diff[0]).to_degrees();
        if tactile {
            let ctm = diagram.ctm();
            ctm.translate(q1[0], q1[1]);
            ctm.rotate(-angle, true);
            ctm.scale(1.0, -1.0);
            ctm.rotate(angle, true);
            ctm.translate(-q1[0], -q1[1]);
        } else {
            let t_string = format!(
                "{} {} {} {} {}",
                ctm::translatestr(p1[0], p1[1]),
                ctm::rotatestr(-angle),
                ctm::scalestr(1.0, -1.0),
                ctm::rotatestr(angle),
                ctm::translatestr(-p1[0], -p1[1])
            );
            group.borrow_mut().set("transform", &t_string);
        }
    }

    if transform.starts_with("rotate") {
        let value = diagram.ctx.valid_eval(args).ok();
        let (angle, center) = match value {
            Some(Value::Array(items)) if items.len() == 2 && items[1].rank() == 1 => {
                let angle = items[0].as_num().unwrap_or(0.0);
                let c = items[1].as_vec_f64().unwrap_or(vec![0.0, 0.0]);
                (angle, diagram.transform([c[0], c[1]]))
            }
            Some(v) => (v.as_num().unwrap_or(0.0), diagram.transform([0.0, 0.0])),
            None => return,
        };
        if tactile {
            let center = diagram.inverse_transform(center);
            let ctm = diagram.ctm();
            ctm.translate(center[0], center[1]);
            ctm.rotate(angle, true);
            ctm.translate(-center[0], -center[1]);
        } else {
            let t_string = format!(
                "{} {} {}",
                ctm::translatestr(center[0], center[1]),
                ctm::rotatestr(angle),
                ctm::translatestr(-center[0], -center[1])
            );
            group.borrow_mut().set("transform", &t_string);
        }
    }

    if transform.starts_with("scale") {
        let Some(value) = diagram.ctx.valid_eval(args).ok() else {
            return;
        };
        let Value::Array(items) = &value else {
            return;
        };
        let mut data = items.clone();
        let Some(center_v) = data.pop() else {
            return;
        };
        let Ok(center_user) = center_v.as_vec_f64() else {
            return;
        };
        let center = diagram.transform([center_user[0], center_user[1]]);
        let (sx, sy) = if data.len() == 2 {
            (
                data[0].as_num().unwrap_or(1.0),
                data[1].as_num().unwrap_or(1.0),
            )
        } else {
            let s = data[0].as_num().unwrap_or(1.0);
            (s, s)
        };
        if tactile {
            let center = diagram.inverse_transform(center);
            let ctm = diagram.ctm();
            ctm.translate(center[0], center[1]);
            ctm.scale(sx, sy);
            ctm.translate(-center[0], -center[1]);
        } else {
            let t_string = format!(
                "{}{}{}",
                ctm::translatestr(center[0], center[1]),
                ctm::scalestr(sx, sy),
                ctm::translatestr(-center[0], -center[1])
            );
            group.borrow_mut().set("transform", &t_string);
        }
    }

    if transform.starts_with("matrix") {
        let Some(Value::Array(items)) = diagram.ctx.valid_eval(args).ok() else {
            return;
        };
        if items.len() != 2 {
            return;
        }
        let Value::Array(rows) = &items[0] else {
            return;
        };
        let m: Vec<Vec<f64>> = rows.iter().filter_map(|r| r.as_vec_f64().ok()).collect();
        if m.len() != 2 {
            return;
        }
        let matrix = [[m[0][0], m[0][1]], [m[1][0], m[1][1]]];
        let Ok(user_center) = items[1].as_vec_f64() else {
            return;
        };
        let user_center = [user_center[0], user_center[1]];
        let center = diagram.transform(user_center);
        if tactile {
            let ctm = diagram.ctm();
            ctm.translate(user_center[0], user_center[1]);
            ctm.apply_matrix(matrix);
            ctm.translate(-user_center[0], -user_center[1]);
        } else {
            let t_string = format!(
                "{}{}{}",
                ctm::translatestr(center[0], center[1]),
                ctm::matrixstr(matrix),
                ctm::translatestr(-center[0], -center[1])
            );
            group.borrow_mut().set("transform", &t_string);
        }
    }
}

enum VecOrRows {
    Flat(Vec<f64>),
    Rows(Vec<Vec<f64>>),
}

fn eval_vec_or_rows(diagram: &mut Diagram, expr: &str) -> Option<VecOrRows> {
    let value = diagram.ctx.valid_eval(expr).ok()?;
    match &value {
        Value::Array(items) if items.first().map(|i| i.rank() > 0).unwrap_or(false) => {
            let rows: Option<Vec<Vec<f64>>> = items.iter().map(|i| i.as_vec_f64().ok()).collect();
            Some(VecOrRows::Rows(rows?))
        }
        _ => Some(VecOrRows::Flat(value.as_vec_f64().ok()?)),
    }
}

fn clean_up_transform(diagram: &mut Diagram, tactile: bool) {
    if tactile {
        diagram.ctm().pop();
    }
}
