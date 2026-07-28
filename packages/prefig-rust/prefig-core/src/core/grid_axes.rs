//! Port of prefig/core/grid_axes.py: grids and the grid-axes convenience.

use crate::core::ctm::AxisScale;
use crate::core::diagram::Diagram;
use crate::core::line::{infinite_line, mk_line};
use crate::core::math_utilities::{length, linspace};
use crate::core::utilities::{self as util, float2str, pt2str};
use crate::core::axes;
use crate::xml::{self, El};

fn grid_delta(key: i64) -> f64 {
    match key {
        2 => 0.1,
        3 | 4 => 0.25,
        5..=11 => 0.5,
        12..=20 => 1.0,
        _ => 1.0,
    }
}

pub fn find_gridspacing(range: [f64; 2], pi_format: bool) -> (f64, f64, f64) {
    let range = if pi_format {
        [
            range[0] / std::f64::consts::PI,
            range[1] / std::f64::consts::PI,
        ]
    } else {
        range
    };
    let mut dx = 1.0f64;
    let mut distance = (range[1] - range[0]).abs();
    while distance > 10.0 {
        distance /= 10.0;
        dx *= 10.0;
    }
    while distance <= 1.0 {
        distance *= 10.0;
        dx /= 10.0;
    }
    dx *= grid_delta((2.0 * distance).round_ties_even() as i64);
    let (x0, x1) = if range[1] < range[0] {
        dx *= -1.0;
        (
            dx * (range[0] / dx + 1e-10).floor(),
            dx * (range[1] / dx - 1e-10).ceil(),
        )
    } else {
        (
            dx * (range[0] / dx - 1e-10).ceil(),
            dx * (range[1] / dx + 1e-10).floor(),
        )
    };
    if pi_format {
        (
            x0 * std::f64::consts::PI,
            dx * std::f64::consts::PI,
            x1 * std::f64::consts::PI,
        )
    } else {
        (x0, dx, x1)
    }
}

/// The grid module's log-position table (differs slightly from axes.py's).
pub fn find_log_positions(r: &[f64]) -> Vec<f64> {
    let x0 = r[0].log10();
    let x1 = r[r.len() - 1].log10();
    let spacing = if r.len() == 3 {
        let step = r[1];
        if step < 1.0 {
            step
        } else if step < 2.0 {
            1.0
        } else if step < 4.0 {
            2.0
        } else if step < 7.0 {
            5.0
        } else {
            10.0
        }
    } else {
        let width = (x1 - x0).abs();
        if width < 1.5 {
            10.0
        } else if width < 3.0 {
            5.0
        } else if width < 5.0 {
            2.0
        } else if width <= 10.0 {
            1.0
        } else {
            10.0 / width
        }
    };

    let x0 = x0.floor() as i64;
    let x1 = x1.ceil() as i64;
    let mut positions = Vec::new();
    if spacing <= 1.0 {
        let gap = (1.0 / spacing).round_ties_even() as i64;
        let mut x = x0;
        while x <= x1 {
            positions.push(10f64.powi(x as i32));
            x += gap;
        }
    } else {
        let intermediate: &[f64] = if spacing == 2.0 {
            &[1.0, 5.0]
        } else if spacing == 5.0 {
            &[1.0, 2.0, 4.0, 6.0, 8.0]
        } else if spacing == 10.0 {
            &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]
        } else {
            &[1.0]
        };
        let mut x = x0;
        while x <= x1 {
            positions.extend(intermediate.iter().map(|c| 10f64.powi(x as i32) * c));
            x += 1;
        }
    }
    positions
}

fn find_linear_positions(r: (f64, f64, f64)) -> Vec<f64> {
    let n = ((r.2 - r.0) / r.1).round_ties_even() as usize;
    linspace(r.0, r.2, n)
}

pub fn grid(element: &El, diagram: &mut Diagram, parent: &El, outline_group: Option<&El>) {
    let basis = element.borrow().get("basis");
    if let Some(basis) = basis {
        grid_with_basis(element, diagram, parent, &basis, outline_group);
        return;
    }

    let thickness = element.borrow().get_or("thickness", "1");
    let stroke = element.borrow().get_or("stroke", "#ccc");
    let id = element.borrow().get_or("id", "grid");
    let id = diagram.prepend_id_prefix(&id);
    let grid = xml::sub_element(parent, "g");
    {
        let mut g = grid.borrow_mut();
        g.set("id", &id);
        g.set("stroke", &stroke);
        g.set("stroke-width", &thickness);
    }
    diagram.register_svg_element(element, &grid);
    util::cliptobbox(&grid, element, diagram);

    let bbox = diagram.bbox();
    let spacings = element.borrow().get("spacings");
    let h_pi_format = element.borrow().get_or("h-pi-format", "no") == "yes";
    let v_pi_format = element.borrow().get_or("v-pi-format", "no") == "yes";

    let coordinates = element.borrow().get_or("coordinates", "cartesian");
    let scales = diagram.get_scales();

    let eval_triple = |diagram: &mut Diagram, s: &str| -> Option<(f64, f64, f64)> {
        let v = diagram.ctx.valid_eval(s).ok()?.as_vec_f64().ok()?;
        (v.len() >= 3).then(|| (v[0], v[1], v[2]))
    };

    let mut hspacings_set = false;
    let mut rx: Option<(f64, f64, f64)> = None;
    let mut ry: Option<(f64, f64, f64)> = None;
    let mut x_positions: Vec<f64> = Vec::new();
    let mut y_positions: Vec<f64> = Vec::new();

    if let Some(spacings_attr) = spacings {
        let pair = diagram.ctx.valid_eval(&spacings_attr).ok().and_then(|v| {
            let crate::value::Value::Array(items) = v else {
                return None;
            };
            let rx = items.first()?.as_vec_f64().ok()?;
            let ry = items.get(1)?.as_vec_f64().ok()?;
            Some(((rx[0], rx[1], rx[2]), (ry[0], ry[1], ry[2])))
        });
        let Some((rx_v, ry_v)) = pair else {
            log::error!("Error in <grid> parsing spacings={spacings_attr}");
            return;
        };
        x_positions = if scales[0] == AxisScale::Log {
            find_log_positions(&[rx_v.0, rx_v.1, rx_v.2])
        } else {
            find_linear_positions(rx_v)
        };
        y_positions = if scales[1] == AxisScale::Log {
            find_log_positions(&[ry_v.0, ry_v.1, ry_v.2])
        } else {
            find_linear_positions(ry_v)
        };
        rx = Some(rx_v);
        ry = Some(ry_v);
        hspacings_set = true;
    } else {
        let hspacing = element.borrow().get("hspacing");
        match hspacing {
            None => {
                if scales[0] == AxisScale::Log {
                    x_positions = find_log_positions(&[bbox[0], bbox[2]]);
                } else {
                    let r = find_gridspacing([bbox[0], bbox[2]], h_pi_format);
                    x_positions = find_linear_positions(r);
                    rx = Some(r);
                }
            }
            Some(attr) => {
                let Some(r) = eval_triple(diagram, &attr) else {
                    log::error!("Error in <grid> parsing hspacing={attr}");
                    return;
                };
                x_positions = if scales[0] == AxisScale::Log {
                    find_log_positions(&[r.0, r.1, r.2])
                } else {
                    find_linear_positions(r)
                };
                rx = Some(r);
                hspacings_set = true;
            }
        }

        if coordinates == "polar" {
            ry = Some((0.0, std::f64::consts::PI / 6.0, 2.0 * std::f64::consts::PI));
        } else {
            let vspacing = element.borrow().get("vspacing");
            match vspacing {
                None => {
                    if scales[1] == AxisScale::Log {
                        y_positions = find_log_positions(&[bbox[1], bbox[3]]);
                    } else {
                        let r = find_gridspacing([bbox[1], bbox[3]], v_pi_format);
                        y_positions = find_linear_positions(r);
                        ry = Some(r);
                    }
                }
                Some(attr) => {
                    let Some(r) = eval_triple(diagram, &attr) else {
                        log::error!("Error in <grid> parsing vspacing={attr}");
                        return;
                    };
                    y_positions = if scales[1] == AxisScale::Log {
                        find_log_positions(&[r.0, r.1, r.2])
                    } else {
                        find_linear_positions(r)
                    };
                    ry = Some(r);
                }
            }
        }
    }

    if coordinates == "polar" {
        let clip_id = diagram.get_clippath();
        grid.borrow_mut()
            .set("clip-path", &format!("url(#{clip_id})"));

        let bbox_list = diagram.bbox();
        // the four corners, by rotating the bbox list
        let endpoints = [
            [bbox_list[0], bbox_list[1]],
            [bbox_list[1], bbox_list[2]],
            [bbox_list[2], bbox_list[3]],
            [bbox_list[3], bbox_list[0]],
        ];
        let mut r_max = endpoints
            .iter()
            .map(|&p| length(p))
            .fold(0.0f64, f64::max);
        let rx = rx.expect("polar grid has hspacing");
        if hspacings_set {
            r_max = rx.2;
        }
        let mut r = rx.1;
        let n = 100;
        let dt = 2.0 * std::f64::consts::PI / n as f64;
        while r <= r_max {
            let circle = xml::sub_element(&grid, "path");
            let mut t = 0.0;
            let mut cmds = vec!["M".to_string()];
            cmds.push(pt2str(
                diagram.transform([r * f64::cos(t), r * f64::sin(t)]),
                " ",
            ));
            for _ in 0..n {
                t += dt;
                cmds.push("L".to_string());
                cmds.push(pt2str(
                    diagram.transform([r * f64::cos(t), r * f64::sin(t)]),
                    " ",
                ));
            }
            cmds.push("Z".to_string());
            circle.borrow_mut().set("d", &cmds.join(" "));
            circle.borrow_mut().set("fill", "none");
            r += rx.1;
        }

        let mut ry = ry.expect("polar grid has vspacing");
        if element.borrow().get_or("spacing-degrees", "no") == "yes" {
            ry = (ry.0.to_radians(), ry.1.to_radians(), ry.2.to_radians());
        }
        let mut t = ry.0;
        while t <= ry.2 {
            let direction = [t.cos(), t.sin()];
            let mut intersection_times: Vec<f64> = Vec::new();
            let vert = direction[0].abs() < 1e-8;
            let horiz = direction[1].abs() < 1e-8;
            if !vert {
                intersection_times.push(bbox_list[0] / direction[0]);
                intersection_times.push(bbox_list[2] / direction[0]);
            }
            if !horiz {
                intersection_times.push(bbox_list[1] / direction[1]);
                intersection_times.push(bbox_list[3] / direction[1]);
            }
            let mut intersection_time = intersection_times
                .iter()
                .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            if hspacings_set {
                intersection_time = r_max;
            }
            if intersection_time > 0.0 {
                let line_el = xml::sub_element(&grid, "line");
                let start = diagram.transform([0.0, 0.0]);
                let end = diagram.transform([
                    intersection_time * direction[0],
                    intersection_time * direction[1],
                ]);
                let mut l = line_el.borrow_mut();
                l.set("x1", &float2str(start[0]));
                l.set("y1", &float2str(start[1]));
                l.set("x2", &float2str(end[0]));
                l.set("y2", &float2str(end[1]));
            }
            t += ry.1;
        }
        return;
    }

    // a plain rectangular grid
    for x in x_positions {
        if x < bbox[0] || x > bbox[2] {
            continue;
        }
        let line_el = mk_line([x, bbox[1]], [x, bbox[3]], diagram, None, None, true);
        line_el.borrow_mut().pop_attr("id");
        xml::append(&grid, &line_el);
    }
    for y in y_positions {
        if y < bbox[1] || y > bbox[3] {
            continue;
        }
        let line_el = mk_line([bbox[0], y], [bbox[2], y], diagram, None, None, true);
        line_el.borrow_mut().pop_attr("id");
        xml::append(&grid, &line_el);
    }
}

/// <grid-axes>: a grid and axes with automatic spacing, wrapped in a group.
pub fn grid_axes(element: &El, diagram: &mut Diagram, parent: &El, outline_group: Option<&El>) {
    let id = element.borrow().get_or("id", "grid-axes");
    let id = diagram.prepend_id_prefix(&id);
    let group = xml::sub_element(parent, "g");
    group.borrow_mut().set("id", &id);
    diagram.register_svg_element(element, &group);

    let annotation_id = diagram.prepend_id_prefix("grid-axes");
    let grid_id = diagram.prepend_id_prefix("grid");
    let axes_id = diagram.prepend_id_prefix("axes");

    let group_annotation = xml::new_element("annotation");
    group_annotation.borrow_mut().set("ref", &annotation_id);
    group_annotation
        .borrow_mut()
        .set("text", "The coordinate grid and axes");
    if element.borrow().get_or("annotate", "yes") == "yes" {
        diagram.add_default_annotation(group_annotation.clone());
    }
    element.borrow_mut().set("id", &grid_id);
    grid(element, diagram, &group, outline_group);

    let annotation = xml::new_element("annotation");
    annotation.borrow_mut().set("ref", &grid_id);
    annotation.borrow_mut().set("text", "The coordinate grid");
    xml::append(&group_annotation, &annotation);

    element.borrow_mut().set("id", &axes_id);
    axes::axes(element, diagram, &group, outline_group);

    let annotation = xml::new_element("annotation");
    annotation.borrow_mut().set("ref", &axes_id);
    annotation.borrow_mut().set("text", "The coordinate axes");
    xml::append(&group_annotation, &annotation);
}

fn grid_with_basis(
    element: &El,
    diagram: &mut Diagram,
    parent: &El,
    basis: &str,
    outline_group: Option<&El>,
) {
    let vectors = diagram.ctx.valid_eval(basis).ok().and_then(|v| {
        let crate::value::Value::Array(items) = v else {
            return None;
        };
        let v1 = items.first()?.as_vec_f64().ok()?;
        let v2 = items.get(1)?.as_vec_f64().ok()?;
        Some(([v1[0], v1[1]], [v2[0], v2[1]]))
    });
    let Some((v1, v2)) = vectors else {
        log::error!("Error in <grid> parsing basis={basis}");
        return;
    };

    if diagram.output_format() == "tactile" {
        element.borrow_mut().set("stroke", "black");
    } else {
        let stroke = element.borrow().get_or("stroke", "black");
        element.borrow_mut().set("stroke", &stroke);
    }
    let thickness = element.borrow().get_or("thickness", "2");
    element.borrow_mut().set("thickness", &thickness);

    let mut cmds: Vec<String> = Vec::new();
    let mut add_family = |diagram: &mut Diagram, base: [f64; 2], dir: [f64; 2], range: Vec<i64>| {
        for i in range {
            let sv = [i as f64 * base[0], i as f64 * base[1]];
            let sv_end = [sv[0] + dir[0], sv[1] + dir[1]];
            let Some((p1, p2)) = infinite_line(sv, sv_end, diagram, None) else {
                break;
            };
            let p1 = diagram.transform(p1);
            let p2 = diagram.transform(p2);
            cmds.push(format!("M {}", pt2str(p1, " ")));
            cmds.push(format!("L {}", pt2str(p2, " ")));
        }
    };
    add_family(diagram, v1, v2, (0..100).collect());
    add_family(diagram, v1, v2, (-99..0).rev().collect());
    add_family(diagram, v2, v1, (0..100).collect());
    add_family(diagram, v2, v1, (-99..0).rev().collect());

    let coords = xml::new_element("path");
    let id = element.borrow().get("id");
    diagram.add_id(&coords, id.as_deref());
    diagram.register_svg_element(element, &coords);

    util::add_attr(&coords, util::get_1d_attr(element, &mut diagram.ctx));
    coords.borrow_mut().set("d", &cmds.join(" "));

    if let Some(outline_group) = outline_group {
        diagram.add_outline(element, &coords, outline_group, None);
        finish_outline(element, diagram, parent);
    } else if element.borrow().get_or("outline", "no") == "yes"
        || diagram.output_format() == "tactile"
    {
        diagram.add_outline(element, &coords, parent, None);
        finish_outline(element, diagram, parent);
    } else {
        xml::append(parent, &coords);
    }
}

fn finish_outline(element: &El, diagram: &mut Diagram, parent: &El) {
    let stroke = element.borrow().get("stroke");
    let thickness = element.borrow().get("thickness");
    diagram.finish_outline(element, stroke, thickness, "none", parent);
}
