//! Port of prefig/core/vector.py.

use crate::core::diagram::Diagram;
use crate::core::utilities::{self as util, np2str, pt2long_str, pt2str};
use crate::core::{arrow, label};
use crate::value::Value;
use crate::xml::{self, El};

pub fn vector(element: &El, diagram: &mut Diagram, parent: &El, outline_group: Option<&El>) {
    let v_attr = element.borrow().get("v");
    let Some(v) = v_attr
        .as_deref()
        .and_then(|a| diagram.ctx.valid_eval(a).ok())
        .and_then(|v| v.as_vec_f64().ok())
    else {
        log::error!("Error parsing vector attribute @v={v_attr:?}");
        return;
    };

    let tail_attr = element.borrow().get_or("tail", "[0,0]");
    let tail = diagram
        .ctx
        .valid_eval(&tail_attr)
        .ok()
        .and_then(|v| v.as_vec_f64().ok())
        .unwrap_or(vec![0.0, 0.0]);
    let scale_attr = element.borrow().get_or("scale-length", "1");
    let scale = diagram
        .ctx
        .valid_eval(&scale_attr)
        .ok()
        .and_then(|v| v.as_num().ok())
        .unwrap_or(1.0);
    let v = [scale * v[0], scale * v[1]];
    let tail = [tail[0], tail[1]];
    let w = [v[0] + tail[0], v[1] + tail[1]];

    let vec_value = |p: [f64; 2]| Value::Array(vec![Value::Num(p[0]), Value::Num(p[1])]);
    diagram.register_source_data(element, "v", vec_value(v));
    diagram.register_source_data(element, "head", vec_value(w));
    diagram.register_source_data(element, "tail", vec_value(tail));

    // where along the shaft the head appears (default: the tip)
    let t: Option<f64> = element
        .borrow()
        .get("head-location")
        .and_then(|s| s.parse().ok());
    let head_loc = t.map(|t| {
        [
            (1.0 - t) * tail[0] + t * w[0],
            (1.0 - t) * tail[1] + t * w[1],
        ]
    });

    if diagram.output_format() == "tactile" {
        element.borrow_mut().set("fill", "black");
        element.borrow_mut().set("stroke", "black");
    } else {
        util::set_attr(element, "stroke", "black", &mut diagram.ctx);
        util::set_attr(element, "fill", "none", &mut diagram.ctx);
    }
    util::set_attr(element, "thickness", "3", &mut diagram.ctx);

    let vector = xml::new_element("path");
    let attrs = util::get_2d_attr(element, diagram);
    util::add_attr(&vector, attrs);

    let location = if t.is_some() {
        "marker-mid"
    } else {
        "marker-end"
    };
    let arrow_width = element.borrow().get("arrow-width");
    let arrow_angles = element.borrow().get("arrow-angles");
    let arrow_id = arrow::add_arrowhead_to_path(
        diagram,
        location,
        &vector,
        arrow_width.as_deref(),
        arrow_angles.as_deref(),
    );

    // pull the tip in a bit to accommodate the arrowhead
    let p0 = diagram.transform(tail);
    let mut p1 = diagram.transform(w);
    let diff = [p1[0] - p0[0], p1[1] - p0[1]];
    let mut length = (diff[0] * diff[0] + diff[1] * diff[1]).sqrt();
    let angle = diff[1].atan2(diff[0]);
    diagram.register_source_data(element, "angle", Value::Num(angle));

    let arrow_head_length = arrow_id
        .and_then(|id| diagram.arrow_lengths.get(&id).copied())
        .unwrap_or(0.0);
    let thickness_attr = element.borrow().get_or("thickness", "3");
    let thickness = diagram
        .ctx
        .valid_eval(&thickness_attr)
        .ok()
        .and_then(|v| v.as_num().ok())
        .unwrap_or(3.0);
    if location == "marker-end" {
        length -= thickness * arrow_head_length;
        p1 = [
            length * angle.cos() + p0[0],
            length * angle.sin() + p0[1],
        ];
    }

    let mut cmds = vec![format!("M {}", pt2str(p0, " "))];
    if let Some(head_loc) = head_loc {
        cmds.push(format!("L {}", pt2str(diagram.transform(head_loc), " ")));
    }
    cmds.push(format!("L {}", pt2str(p1, " ")));
    vector.borrow_mut().set("d", &cmds.join(" "));

    let mut parent = parent.clone();
    if label::has_label(element) {
        let group = xml::sub_element(&parent, "g");
        let id = element.borrow().get("id");
        diagram.add_id(&group, id.as_deref());
        diagram.register_svg_element(element, &group);
        parent = group;
        add_label(element, diagram, &parent);
    } else {
        let id = element.borrow().get("id");
        diagram.add_id(&vector, id.as_deref());
        diagram.register_svg_element(element, &vector);
    }

    if let Some(outline_group) = outline_group {
        diagram.add_outline(element, &vector, outline_group, None);
        finish_outline(element, diagram, &parent);
    } else if element.borrow().get_or("outline", "no") == "yes"
        || diagram.output_format() == "tactile"
    {
        diagram.add_outline(element, &vector, &parent, None);
        finish_outline(element, diagram, &parent);
    } else {
        xml::append(&parent, &vector);
    }
}

fn finish_outline(element: &El, diagram: &mut Diagram, parent: &El) {
    let stroke = element.borrow().get("stroke");
    let thickness = element.borrow().get("thickness");
    let fill = element.borrow().get_or("fill", "none");
    diagram.finish_outline(element, stroke, thickness, &fill, parent);
}

/// alignment and offset direction by half-quadrant of the vector's angle
fn alignment_for_half_quadrant(hq: i64) -> (&'static str, f64) {
    match hq {
        0 => ("se", -1.0),
        1 => ("nw", 1.0),
        2 => ("ne", -1.0),
        3 => ("sw", 1.0),
        4 => ("nw", -1.0),
        5 => ("se", 1.0),
        6 => ("sw", -1.0),
        _ => ("ne", 1.0),
    }
}

fn add_label(element: &El, diagram: &mut Diagram, parent: &El) {
    let el = xml::deep_copy(element);
    el.borrow_mut().tag = "label".to_string();

    let alignment = element.borrow().get("alignment");
    let user_offset = element.borrow().get("offset");
    let angle = diagram
        .get_source_data(element, "angle")
        .and_then(|v| v.as_num().ok())
        .unwrap_or(0.0);

    match alignment {
        None => {
            let mut angle_degrees = (-angle).to_degrees();
            while angle_degrees < 0.0 {
                angle_degrees += 360.0;
            }
            let half_quadrant = (angle_degrees / 45.0).floor() as i64;
            let (alignment, offset_dir) = alignment_for_half_quadrant(half_quadrant);
            el.borrow_mut().set("alignment", alignment);

            let normal = [(-angle).cos(), (-angle).sin()];
            let direction = [offset_dir * -normal[1], offset_dir * normal[0]];
            let mut offset = [4.0 * direction[0], 4.0 * direction[1]];
            if let Some(user_offset) = &user_offset {
                if let Some(v) = diagram
                    .ctx
                    .valid_eval(user_offset)
                    .ok()
                    .and_then(|v| v.as_vec_f64().ok())
                {
                    offset = [offset[0] + v[0], offset[1] + v[1]];
                }
            }
            el.borrow_mut().set("abs-offset", &np2str(offset));
        }
        Some(alignment) => {
            let displacement =
                label::alignment_displacement(&alignment).unwrap_or([-0.5, 0.5]);
            let mut def_offset = [
                4.0 * (displacement[0] + 0.5),
                4.0 * (displacement[1] - 0.5),
            ];
            if let Some(user_offset) = &user_offset {
                if let Some(v) = diagram
                    .ctx
                    .valid_eval(user_offset)
                    .ok()
                    .and_then(|v| v.as_vec_f64().ok())
                {
                    def_offset = [def_offset[0] + v[0], def_offset[1] + v[1]];
                }
            }
            el.borrow_mut().set("offset", &np2str(def_offset));
        }
    }

    let label_location = element
        .borrow()
        .get("label-location")
        .and_then(|s| diagram_eval_num(diagram, &s))
        .map(|l| l.clamp(0.0, 1.0))
        .unwrap_or(1.0);
    let head = diagram
        .get_source_data(element, "head")
        .and_then(|v| v.as_vec_f64().ok())
        .unwrap_or(vec![0.0, 0.0]);
    let tail = diagram
        .get_source_data(element, "tail")
        .and_then(|v| v.as_vec_f64().ok())
        .unwrap_or(vec![0.0, 0.0]);
    let anchor = [
        label_location * head[0] + (1.0 - label_location) * tail[0],
        label_location * head[1] + (1.0 - label_location) * tail[1],
    ];
    el.borrow_mut().set("anchor", &pt2long_str(anchor, ","));

    label::label(&el, diagram, parent, None);
}

fn diagram_eval_num(diagram: &mut Diagram, s: &str) -> Option<f64> {
    diagram.ctx.valid_eval(s).ok()?.as_num().ok()
}
