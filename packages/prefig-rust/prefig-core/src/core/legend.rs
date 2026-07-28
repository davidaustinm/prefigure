//! Port of prefig/core/legend.py (non-tactile path; tactile legends TODO).

use crate::core::ctm;
use crate::core::diagram::Diagram;
use crate::core::utilities::{self as util, pt2str};
use crate::core::{label, point};
use crate::value::py_str;
use crate::xml::{self, El};

pub struct LegendData {
    pub element: El,
    pub group: El,
    pub def_anchor: [f64; 2],
    pub displacement: [f64; 2],
    /// (item element, key element, label element)
    pub items: Vec<(El, El, El)>,
    pub key_width: f64,
    pub line_width: f64,
}

pub fn legend(element: &El, diagram: &mut Diagram, parent: &El, _outline_group: Option<&El>) {
    let tactile = diagram.output_format() == "tactile";

    let group = xml::new_element("g");
    let id = element.borrow().get("id");
    diagram.add_id(&group, id.as_deref());
    xml::append(parent, &group);
    diagram.register_svg_element(element, &group);

    let anchor_str = element.borrow().get_or("anchor", "(bbox[2],bbox[3])");
    let Some(user_anchor) = diagram
        .ctx
        .valid_eval(&anchor_str)
        .ok()
        .and_then(|v| v.as_vec_f64().ok())
    else {
        log::error!("Error in <legend> evaluating anchor={anchor_str}");
        return;
    };
    let def_anchor = diagram.transform([user_anchor[0], user_anchor[1]]);

    if element.borrow().get_or("alignment", "c") == "e" {
        element.borrow_mut().set("alignment", "east");
    }
    let alignment = util::get_attr(element, "alignment", "c", &mut diagram.ctx);
    let displacement = label::alignment_displacement(&alignment).unwrap_or([-0.5, 0.5]);

    let mut items: Vec<(El, El, El)> = Vec::new();
    let mut key_width = 0.0f64;
    let point_width = 10.0;
    let line_width = if tactile { 72.0 } else { 24.0 };

    let dummy_group = xml::new_element("g");
    let children: Vec<El> = element.borrow().children.clone();
    for (num, li) in children.iter().enumerate() {
        if li.borrow().tag != "item" {
            log::warn!("{} is not allowed inside a <legend>", li.borrow().tag);
            continue;
        }

        // the label
        let label_id_stub = diagram.prepend_id_prefix("legend-label");
        let label_el = xml::deep_copy(li);
        {
            let mut l = label_el.borrow_mut();
            l.tag = "label".to_string();
            l.set("id", &format!("{label_id_stub}-{num}"));
            l.set("alignment", "se");
            l.set("anchor", &anchor_str);
            l.set("abs-offset", "(0,0)");
            l.set("justify", "left");
        }
        label::label(&label_el, diagram, &dummy_group, None);

        // and the key: find the referenced element by id or at
        let ref_attr = label_el.borrow().get_or("ref", "");
        let mut reference = None;
        for candidate in xml::iter_subtree(&diagram.diagram_element) {
            let c = candidate.borrow();
            if c.get("id").as_deref() == Some(&ref_attr)
                || c.get("at").as_deref() == Some(&ref_attr)
            {
                drop(c);
                reference = Some(candidate);
                break;
            }
        }
        let Some(key_src) = reference else {
            log::warn!("{ref_attr} should refer to an element");
            continue;
        };

        let point_id_stub = diagram.prepend_id_prefix("legend-point");
        let key = if key_src.borrow().tag == "point" {
            let key = xml::deep_copy(&key_src);
            {
                let mut k = key.borrow_mut();
                k.set("p", &anchor_str);
                k.set("size", "4");
                k.set("id", &format!("{point_id_stub}-{num}"));
            }
            key_width = key_width.max(point_width);
            key
        } else {
            let fill = key_src.borrow().get("fill");
            if fill.is_none() || fill.as_deref() == Some("none") {
                let key_el = xml::new_element("line");
                {
                    let mut k = key_el.borrow_mut();
                    k.set("stroke", &key_src.borrow().get_or("stroke", "none"));
                    if let Some(dash) = key_src.borrow().get("dash") {
                        k.set("stroke-dasharray", &dash);
                    }
                }
                key_width = key_width.max(line_width);
                key_el
            } else {
                let key_el = xml::new_element("point");
                {
                    let mut k = key_el.borrow_mut();
                    k.set("stroke", &key_src.borrow().get_or("stroke", "none"));
                    k.set("fill", &fill.unwrap_or_default());
                    k.set("style", &key_src.borrow().get_or("style", "box"));
                    k.set("size", "5");
                }
                key_width = key_width.max(point_width);
                key_el
            }
        };

        items.push((li.clone(), key, label_el));
    }

    diagram.legends.push(LegendData {
        element: element.clone(),
        group,
        def_anchor,
        displacement,
        items,
        key_width,
        line_width,
    });
}

pub fn place_legend(diagram: &mut Diagram, data: &LegendData) {
    if diagram.output_format() == "tactile" {
        place_tactile_legend(diagram, data);
        return;
    }

    let outer_padding = 5.0;
    let center_padding = 10.0;
    let interline_attr = data.element.borrow().get_or("vertical-skip", "7");
    let interline = diagram
        .ctx
        .valid_eval(&interline_attr)
        .ok()
        .and_then(|v| v.as_num().ok())
        .unwrap_or(7.0);
    let mut height = outer_padding;
    let mut label_width = 0.0f64;

    for (_, _, label_el) in &data.items {
        let Some(dims) = diagram.get_label_dims(label_el) else {
            log::warn!("There is a missing label in a <legend>");
            continue;
        };
        height += dims.1 + interline;
        label_width = label_width.max(dims.0);
    }
    height += outer_padding - interline;

    let width = label_width + 2.0 * outer_padding + data.key_width + center_padding;

    let offset = [
        8.0 * (data.displacement[0] + 0.5),
        8.0 * (data.displacement[1] - 0.5),
    ];
    let p = data.def_anchor;
    let mut tform = ctm::translatestr(p[0] + offset[0], p[1] - offset[1]);
    let scale_attr = data.element.borrow().get_or("scale", "1");
    let scale = diagram
        .ctx
        .valid_eval(&scale_attr)
        .ok()
        .and_then(|v| v.as_num().ok())
        .unwrap_or(1.0);
    let dx = scale * width * data.displacement[0];
    let dy = scale * height * data.displacement[1];
    tform = format!("{tform} {}", ctm::translatestr(dx, -dy));
    tform = format!("{tform} {}", ctm::scalestr(scale, scale));
    data.group.borrow_mut().set("transform", &tform);

    // the legend's bounding box
    let rect = xml::new_element("rect");
    xml::append(&data.group, &rect);
    {
        let mut r = rect.borrow_mut();
        r.set("x", "0");
        r.set("y", "0");
        r.set("width", &py_str(width));
        r.set("height", &py_str(height));
        r.set("stroke", &data.element.borrow().get_or("stroke", "black"));
        r.set("fill", "white");
        r.set(
            "fill-opacity",
            &data.element.borrow().get_or("opacity", "1"),
        );
    }

    // place the labels and keys
    let label_x = outer_padding + data.key_width + center_padding;
    let mut y = outer_padding;
    for (_, key, label_el) in &data.items {
        let Some(dims) = diagram.get_label_dims(label_el) else {
            continue;
        };
        let Some(label_group) = diagram.get_label_group(label_el) else {
            continue;
        };
        let tform = ctm::translatestr(label_x, y);
        label_group.borrow_mut().set("transform", &tform);
        xml::append(&data.group, &label_group);

        let key_y = y + dims.1 / 2.0;
        let key_tag = key.borrow().tag.clone();
        if key_tag == "point" {
            let key_x = outer_padding + data.key_width / 2.0;
            let user_point = diagram.inverse_transform([key_x, key_y]);
            key.borrow_mut().set("p", &pt2str(user_point, ","));
            point::point(key, diagram, &data.group, None);
        }
        if key_tag == "line" {
            let key_x0 = outer_padding;
            let key_x1 = outer_padding + data.line_width;
            {
                let mut k = key.borrow_mut();
                k.set("x1", &py_str(key_x1));
                k.set("y1", &py_str(key_y));
                k.set("x2", &py_str(key_x0));
                k.set("y2", &py_str(key_y));
                k.set("stroke-width", "2");
            }
            xml::append(&data.group, key);
        }

        y += dims.1 + interline;
    }
}

/// Port of legend.py place_tactile_legend: braille labels were already emitted
/// into the diagram; move them into the legend group and lay them out on the
/// embossing grid.
fn place_tactile_legend(diagram: &mut Diagram, data: &LegendData) {
    let gap = 3.6;

    // the braille labels were placed into background-group / braille-group;
    // pull the ones belonging to this legend back out
    let legend_label_id = diagram.prepend_id_prefix("legend-label");
    let mut label_groups: Vec<El> = Vec::new();
    let root_children: Vec<El> = diagram.root.borrow().children.clone();
    for g in &root_children {
        let id = g.borrow().get_or("id", "none");
        if id == "background-group" {
            let rects: Vec<El> = g.borrow().children.clone();
            for rect in rects {
                if rect
                    .borrow()
                    .get_or("id", "none")
                    .starts_with(&legend_label_id)
                {
                    xml::remove(g, &rect);
                }
            }
        }
        if id == "braille-group" {
            let labels: Vec<El> = g.borrow().children.clone();
            for label in labels {
                if label
                    .borrow()
                    .get_or("id", "none")
                    .starts_with(&legend_label_id)
                {
                    label_groups.push(label.clone());
                    xml::remove(g, &label);
                }
            }
        }
    }

    let outer_padding = 4.2 * gap;
    let center_padding = 6.0 * gap;
    let interline = 4.0 * gap;
    let mut height = outer_padding;
    let mut label_width = 0.0f64;
    for (_, _, label_el) in &data.items {
        let Some(dims) = diagram.get_label_dims(label_el) else {
            continue;
        };
        height += dims.1 + interline;
        label_width = label_width.max(dims.0);
    }
    height += outer_padding - interline;
    let width = label_width + 2.0 * outer_padding + data.key_width + center_padding;

    let mut offset = [
        8.0 * (data.displacement[0] + 0.5),
        8.0 * (data.displacement[1] - 0.5),
    ];
    offset = [
        offset[0] + 6.0 * offset[0].signum(),
        offset[1] + 6.0 * offset[1].signum(),
    ];
    if data.displacement[0] == 0.0 {
        offset[0] += 6.0;
    }
    if data.displacement[1] == -1.0 {
        offset[1] -= 6.0;
    }

    let p = data.def_anchor;
    let dx = width * data.displacement[0];
    let dy = height * data.displacement[1];
    let translate = [
        gap * ((p[0] + offset[0] + dx) / gap).round_ties_even(),
        gap * ((p[1] - offset[1] - dy) / gap).round_ties_even(),
    ];
    data.group
        .borrow_mut()
        .set("transform", &ctm::translatestr(translate[0], translate[1]));

    let rect = xml::sub_element(&data.group, "rect");
    {
        let mut r = rect.borrow_mut();
        r.set("x", "0");
        r.set("y", "0");
        r.set("width", &py_str(width));
        r.set("height", &py_str(height));
        r.set("stroke", &data.element.borrow().get_or("stroke", "black"));
        r.set("fill", "white");
    }

    let label_x = outer_padding + data.key_width + center_padding;
    let mut y = outer_padding;
    for (num, (_, key, label_el)) in data.items.iter().enumerate() {
        let Some(dims) = diagram.get_label_dims(label_el) else {
            continue;
        };
        let Some(label_group) = diagram.get_label_group(label_el) else {
            continue;
        };
        let label_x = gap * (label_x / gap).round_ties_even();
        y = gap * (y / gap).round_ties_even();
        label_group
            .borrow_mut()
            .set("transform", &ctm::translatestr(label_x, y));
        if let Some(braille) = label_groups.get(num) {
            let label_height = gap * (dims.1 / gap).round_ties_even();
            braille
                .borrow_mut()
                .set("transform", &ctm::translatestr(0.0, label_height));
            xml::append(&label_group, braille);
        }
        xml::append(&data.group, &label_group);

        let key_y = y + dims.1 / 2.0;
        let key_tag = key.borrow().tag.clone();
        if key_tag == "point" {
            let key_x = outer_padding + data.key_width / 2.0;
            let user_point = diagram.inverse_transform([key_x, key_y]);
            key.borrow_mut().set("p", &pt2str(user_point, ","));
            point::point(key, diagram, &data.group, None);
        }
        if key_tag == "line" {
            let key_x0 = outer_padding;
            let key_x1 = outer_padding + data.line_width;
            {
                let mut k = key.borrow_mut();
                k.set("x1", &py_str(key_x1));
                k.set("y1", &py_str(key_y));
                k.set("x2", &py_str(key_x0));
                k.set("y2", &py_str(key_y));
                k.set("stroke-width", "2");
            }
            xml::append(&data.group, key);
        }
        y += dims.1 + interline;
    }
}
