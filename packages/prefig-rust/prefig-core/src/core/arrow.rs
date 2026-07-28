//! Port of prefig/core/arrow.py: arrowhead markers added to paths.

use crate::core::ctm::CTM;
use crate::core::diagram::Diagram;
use crate::core::repeat::epub_clean;
use crate::core::utilities::{float2str, pt2str};
use crate::value::py_str;
use crate::xml::{self, El};

/// Tactile arrowheads follow BANA-inspired dimensions and get an outline.
fn add_tactile_arrowhead_marker(diagram: &mut Diagram, path: &El) -> Option<String> {
    let stroke_width_str = path.borrow().get_or("stroke-width", "1");
    let stroke_width: f64 = stroke_width_str.parse().ok()?;
    let id = diagram.prepend_id_prefix(&format!("arrow-head-{stroke_width_str}"));

    if diagram.has_reusable(&id) {
        return Some(id);
    }

    let angle = 25f64;
    let a = angle.to_radians();
    let t = 1.0;
    let s = 9.0;
    let l = t / a.tan() + 0.1;
    let y = s * a.tan();

    diagram.arrow_lengths.insert(id.clone(), l);

    let mut ctm = CTM::new();
    ctm.scale(stroke_width, stroke_width);
    ctm.translate(s - l, y);
    let p1 = ctm.transform([l, 0.0]);
    let p2 = ctm.transform([l - s, y]);
    let p3 = ctm.transform([l - s, -y]);
    let d = format!(
        "M {} L {} L {} Z",
        pt2str(p1, " "),
        pt2str(p2, " "),
        pt2str(p3, " ")
    );

    let x2 = l - s;
    let dims = [1.0, 2.0 * y];

    let marker = xml::new_element("marker");
    {
        let mut m = marker.borrow_mut();
        m.set("id", &id);
        m.set("markerWidth", &float2str(stroke_width * (l - x2)));
        m.set("markerHeight", &float2str(stroke_width * dims[1]));
        m.set("markerUnits", "userSpaceOnUse");
        m.set("orient", "auto-start-reverse");
        m.set("refX", &float2str(stroke_width * x2.abs()));
        m.set("refY", &float2str(stroke_width * dims[1] / 2.0));
    }
    let head = xml::sub_element(&marker, "path");
    {
        let mut h = head.borrow_mut();
        h.set("d", &d);
        h.set("fill", "context-stroke");
        h.set("stroke", "context-none");
    }
    diagram.add_reusable(&marker);

    // the outline marker, 1/8" beyond the arrowhead
    let outline_width = 9.0;
    let push_angle = (90.0 - angle).to_radians();
    let w = [
        outline_width * push_angle.cos(),
        outline_width * push_angle.sin(),
    ];
    let q1 = [p1[0] + w[0], p1[1] + w[1]];
    let q2 = [p2[0] + w[0], p2[1] + w[1]];
    let q3 = [p2[0] - outline_width, p2[1]];
    let q4 = [p3[0] - outline_width, p3[1]];
    let q5 = [p3[0] + w[0], p3[1] - w[1]];
    let q6 = [p1[0] + w[0], p1[1] - w[1]];

    let mut ctm = CTM::new();
    ctm.translate(outline_width, outline_width);
    let pts: Vec<String> = [q1, q2, q3, q4, q5, q6]
        .iter()
        .map(|&q| pt2str(ctm.transform(q), " "))
        .collect();
    let ow = py_str(outline_width);
    let d = format!(
        "M {} L {} A {ow} {ow} 0 0 1 {} L {} A {ow} {ow} 0 0 1 {} L {} A {ow} {ow} 0 0 1 {} Z",
        pts[0], pts[1], pts[2], pts[3], pts[4], pts[5], pts[0]
    );

    let marker = xml::new_element("marker");
    {
        let mut m = marker.borrow_mut();
        m.set("id", &format!("{id}-outline"));
        m.set(
            "markerWidth",
            &float2str(stroke_width * (l - x2) + 2.0 * outline_width),
        );
        m.set(
            "markerHeight",
            &float2str(stroke_width * dims[1] + 2.0 * outline_width),
        );
        m.set("markerUnits", "userSpaceOnUse");
        m.set("orient", "auto-start-reverse");
        m.set(
            "refX",
            &float2str((stroke_width * x2).abs() + outline_width),
        );
        m.set(
            "refY",
            &float2str(stroke_width * dims[1] / 2.0 + outline_width),
        );
    }
    let outline = xml::sub_element(&marker, "path");
    {
        let mut o = outline.borrow_mut();
        o.set("d", &d);
        o.set("fill", "context-stroke");
        o.set("stroke", "context-none");
    }
    diagram.add_reusable(&marker);
    Some(id)
}

pub fn add_arrowhead_marker(
    diagram: &mut Diagram,
    path: &El,
    mid: bool,
    arrow_width: Option<&str>,
    arrow_angles: Option<&str>,
) -> Option<String> {
    let arrow_width_value = match arrow_width {
        Some(attr) => match diagram.ctx.valid_eval(attr) {
            Ok(v) => v.as_num().ok(),
            Err(_) => {
                log::error!("Error parsing arrow-width={attr}");
                return None;
            }
        },
        None => None,
    };
    let arrow_angles = match arrow_angles {
        Some(attr) => match diagram
            .ctx
            .valid_eval(attr)
            .ok()
            .and_then(|v| v.as_vec_f64().ok())
        {
            Some(v) if v.len() >= 2 => [v[0], v[1]],
            _ => {
                log::error!("Error parsing arrow-angles={attr}");
                return None;
            }
        },
        None => [24.0, 60.0],
    };

    if diagram.output_format() == "tactile" {
        return add_tactile_arrowhead_marker(diagram, path);
    }

    let stroke_width_str = path.borrow().get_or("stroke-width", "1");
    let stroke_width: f64 = stroke_width_str.parse().ok()?;
    let stroke_color = path.borrow().get_or("stroke", "none");

    // Python f-string embeds None or the evaluated number
    let width_str = match arrow_width_value {
        Some(w) => py_str(w),
        None => "None".to_string(),
    };
    let id_data = format!(
        "_{width_str}_{}_{}",
        py_str(arrow_angles[0]),
        py_str(arrow_angles[1])
    );
    let (id, arrow_width) = if !mid {
        (
            format!("arrow-head-end-{stroke_width_str}{id_data}-{stroke_color}"),
            arrow_width_value.unwrap_or(4.0),
        )
    } else {
        (
            format!("arrow-head-mid-{stroke_width_str}{id_data}-{stroke_color}"),
            arrow_width_value.unwrap_or(13.0 / 3.0),
        )
    };
    let id = epub_clean(&diagram.prepend_id_prefix(&id));

    if diagram.has_reusable(&id) {
        return Some(id);
    }

    let dims = [1.0, arrow_width];
    let (t, s) = (dims[0] / 2.0, dims[1] / 2.0);
    let a = arrow_angles[0].to_radians();
    let b = arrow_angles[1].to_radians();
    let l = t / a.tan() + 0.1;
    let x2 = l - s / a.tan();
    let x1 = x2 + (s - t) / b.tan();

    diagram.arrow_lengths.insert(id.clone(), l);

    let mut ctm = CTM::new();
    ctm.scale(stroke_width, stroke_width);
    ctm.translate(-x2, s);
    let p1 = ctm.transform([l, 0.0]);
    let p2 = ctm.transform([x2, s]);
    let p3 = ctm.transform([x1, t]);
    let p4 = ctm.transform([x1, -t]);
    let p5 = ctm.transform([x2, -s]);

    let d = format!(
        "M {}L {}L {}L {}L {}Z",
        pt2str(p1, " "),
        pt2str(p2, " "),
        pt2str(p3, " "),
        pt2str(p4, " "),
        pt2str(p5, " ")
    );

    let marker = xml::new_element("marker");
    {
        let mut m = marker.borrow_mut();
        m.set("id", &id);
        m.set("markerWidth", &float2str(stroke_width * (l - x2)));
        m.set("markerHeight", &float2str(stroke_width * 2.0 * s));
        m.set("markerUnits", "userSpaceOnUse");
        m.set("orient", "auto-start-reverse");
        m.set("refX", &float2str(stroke_width * x2.abs()));
        m.set("refY", &float2str(stroke_width * s));
    }
    let head = xml::sub_element(&marker, "path");
    {
        let mut h = head.borrow_mut();
        h.set("d", &d);
        h.set("fill", &stroke_color);
        h.set("stroke", "none");
    }
    diagram.add_reusable(&marker);

    // the outline marker
    let outline_width = 2.0;
    let push_angle = std::f64::consts::FRAC_PI_2 - a;
    let w = [
        outline_width * push_angle.cos(),
        outline_width * push_angle.sin(),
    ];
    let q1 = [p1[0] + w[0], p1[1] + w[1]];
    let q2 = [p2[0] + w[0], p2[1] + w[1]];
    let q3 = [p2[0] - outline_width, p2[1]];
    let q4 = [p5[0] - outline_width, p5[1]];
    let q5 = [p5[0] + w[0], p5[1] - w[1]];
    let q6 = [p1[0] + w[0], p1[1] - w[1]];

    let mut ctm = CTM::new();
    ctm.translate(outline_width, outline_width);
    let pts: Vec<String> = [q1, q2, q3, q4, q5, q6]
        .iter()
        .map(|&q| pt2str(ctm.transform(q), " "))
        .collect();
    let ow = py_str(outline_width);
    let d = format!(
        "M {} L {} A {ow} {ow} 0 0 1 {} L {} A {ow} {ow} 0 0 1 {} L {} A {ow} {ow} 0 0 1 {} Z",
        pts[0], pts[1], pts[2], pts[3], pts[4], pts[5], pts[0]
    );

    let marker = xml::new_element("marker");
    {
        let mut m = marker.borrow_mut();
        m.set("id", &format!("{id}-outline"));
        m.set(
            "markerWidth",
            &float2str(stroke_width * (l - x2) + 2.0 * outline_width),
        );
        m.set(
            "markerHeight",
            &float2str(stroke_width * 2.0 * s + 2.0 * outline_width),
        );
        m.set("markerUnits", "userSpaceOnUse");
        m.set("orient", "auto-start-reverse");
        m.set(
            "refX",
            &float2str((stroke_width * x2).abs() + outline_width),
        );
        m.set("refY", &float2str(stroke_width * s + outline_width));
    }
    let outline = xml::sub_element(&marker, "path");
    {
        let mut o = outline.borrow_mut();
        o.set("d", &d);
        o.set("fill", "white");
        o.set("stroke", "none");
    }
    diagram.add_reusable(&marker);

    Some(id)
}

pub fn add_arrowhead_to_path(
    diagram: &mut Diagram,
    location: &str,
    path: &El,
    arrow_width: Option<&str>,
    arrow_angles: Option<&str>,
) -> Option<String> {
    let id = add_arrowhead_marker(
        diagram,
        path,
        location.ends_with("mid"),
        arrow_width,
        arrow_angles,
    )?;
    path.borrow_mut().set(location, &format!("url(#{id})"));
    Some(id)
}
