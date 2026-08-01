//! Port of prefig/core/network.py for explicitly positioned graphs.
//! Automatic layout (networkx's spring/spectral/... algorithms) is not ported
//! yet; every node needs a @p attribute.

use crate::core::ctm::CTM;
use crate::core::diagram::Diagram;
use crate::core::math_utilities::length;
use crate::core::utilities::{self as util, pt2long_str};
use crate::core::{group, label, point};
use crate::evaluator::interp_call;
use crate::value::{py_str, Value};
use crate::xml::{self, El};
use indexmap::IndexMap;

type Point = [f64; 2];

fn fmt_point(p: Point) -> String {
    format!("({})", pt2long_str(p, ","))
}

fn eval_bezier2(p0: Point, p1: Point, p2: Point, t: f64) -> Point {
    let mt = 1.0 - t;
    [
        mt * mt * p0[0] + 2.0 * mt * t * p1[0] + t * t * p2[0],
        mt * mt * p0[1] + 2.0 * mt * t * p1[1] + t * t * p2[1],
    ]
}

fn eval_bezier3(c: &[Point; 4], t: f64) -> Point {
    let mt = 1.0 - t;
    let w = [mt * mt * mt, 3.0 * mt * mt * t, 3.0 * mt * t * t, t * t * t];
    [
        (0..4).map(|i| w[i] * c[i][0]).sum(),
        (0..4).map(|i| w[i] * c[i][1]).sum(),
    ]
}

fn rotate_vec(v: Point, theta: f64) -> Point {
    let (s, c) = theta.sin_cos();
    [c * v[0] - s * v[1], s * v[0] + c * v[1]]
}

#[allow(clippy::too_many_lines)]
pub fn network(element: &El, diagram: &mut Diagram, parent: &El, outline_group: Option<&El>) {
    let directed = element.borrow().get_or("directed", "no") == "yes";
    let global_loop_scale = element.borrow().get("loop-scale").and_then(|a| {
        diagram
            .ctx
            .valid_eval(&a)
            .ok()
            .and_then(|v| v.as_vec_f64().ok())
    });

    // label dictionary
    let label_dict_attr = element.borrow().get_or("label-dictionary", "{}");
    let mut label_dictionary: IndexMap<String, String> = IndexMap::new();
    if let Ok(Value::Dict(map)) = diagram.ctx.valid_eval(&label_dict_attr) {
        for (k, v) in map {
            label_dictionary.insert(k, v.to_py_str());
        }
    }

    // the graph may be given as a dictionary
    let mut graph_dict: IndexMap<String, Vec<String>> = IndexMap::new();
    let graph_attr = element.borrow().get("graph");
    if let Some(attr) = graph_attr {
        let Ok(Value::Dict(map)) = diagram.ctx.valid_eval(&attr) else {
            log::error!("@graph attribute of a <network> element should be a dictionary");
            return;
        };
        for (key, value) in map {
            let destinations: Vec<String> = match value {
                Value::Array(items) => items.iter().map(|v| v.to_py_str()).collect(),
                other => vec![other.to_py_str()],
            };
            graph_dict.insert(key, destinations);
        }
    }

    // multigraph bookkeeping
    let mut loops: IndexMap<String, Vec<Option<El>>> = IndexMap::new();
    let mut directed_edges: IndexMap<(String, String), Vec<Option<El>>> = IndexMap::new();
    let mut all_edges: IndexMap<(String, String), i64> = IndexMap::new();

    let mut nodes: IndexMap<String, Option<El>> = IndexMap::new();
    let mut positions: IndexMap<String, Point> = IndexMap::new();

    for node in xml::find_all(element, "node") {
        let Some(handle) = node.borrow().get("at") else {
            continue;
        };
        nodes.insert(handle.clone(), Some(node.clone()));

        let position = node.borrow().get("p");
        if let Some(p_attr) = position {
            if let Some(v) = diagram
                .ctx
                .valid_eval(&p_attr)
                .ok()
                .and_then(|v| v.as_vec_f64().ok())
            {
                positions.insert(handle.clone(), [v[0], v[1]]);
            }
        }

        let edges_attr = node.borrow().get("edges");
        if let Some(attr) = edges_attr {
            let Ok(value) = diagram.ctx.valid_eval(&attr) else {
                continue;
            };
            let destinations: Vec<String> = match value {
                Value::Array(items) => items.iter().map(|v| v.to_py_str()).collect(),
                other => vec![other.to_py_str()],
            };
            for destination in destinations {
                if destination == handle {
                    loops.entry(handle.clone()).or_default().push(None);
                    continue;
                }
                directed_edges
                    .entry((handle.clone(), destination.clone()))
                    .or_default()
                    .push(None);
                let mut key = [handle.clone(), destination.clone()];
                key.sort();
                *all_edges
                    .entry((key[0].clone(), key[1].clone()))
                    .or_insert(0) += 1;
            }
        }
    }

    for (node, edges) in &graph_dict {
        nodes.entry(node.clone()).or_insert(None);
        for destination in edges {
            if destination == node {
                loops.entry(node.clone()).or_default().push(None);
                continue;
            }
            directed_edges
                .entry((node.clone(), destination.clone()))
                .or_default()
                .push(None);
            let mut key = [node.clone(), destination.clone()];
            key.sort();
            *all_edges
                .entry((key[0].clone(), key[1].clone()))
                .or_insert(0) += 1;
        }
    }

    // <edge> subelements carry decorations
    for edge in xml::find_all(element, "edge") {
        let vertices_attr = edge.borrow().get("vertices").unwrap_or_default();
        let endpoints = diagram.ctx.valid_eval(&vertices_attr).ok().and_then(|v| {
            let Value::Array(items) = v else { return None };
            (items.len() >= 2).then(|| (items[0].to_py_str(), items[1].to_py_str()))
        });
        let Some((p, q)) = endpoints else {
            log::error!("Error in <edge> evaluating vertices={vertices_attr}");
            return;
        };
        if p == q {
            let record = loops.entry(p.clone()).or_default();
            if let Some(slot) = record.iter_mut().find(|e| e.is_none()) {
                *slot = Some(edge.clone());
            } else {
                record.push(Some(edge.clone()));
            }
            continue;
        }

        let mut placed = false;
        if let Some(record) = directed_edges.get_mut(&(p.clone(), q.clone())) {
            if let Some(slot) = record.iter_mut().find(|e| e.is_none()) {
                *slot = Some(edge.clone());
                placed = true;
            }
        }
        if !placed && !directed {
            if let Some(record) = directed_edges.get_mut(&(q.clone(), p.clone())) {
                if let Some(slot) = record.iter_mut().find(|e| e.is_none()) {
                    *slot = Some(edge.clone());
                    placed = true;
                }
            }
        }
        if !placed {
            directed_edges
                .entry((p.clone(), q.clone()))
                .or_default()
                .push(Some(edge.clone()));
            let mut key = [p.clone(), q.clone()];
            key.sort();
            *all_edges
                .entry((key[0].clone(), key[1].clone()))
                .or_insert(0) += 1;
        }
    }

    // Nodes without an explicit @p get an automatic layout.
    let auto_layout = positions.len() != nodes.len();
    let mut bbox_str = String::new();
    if auto_layout {
        let Some((computed, bbox)) = compute_layout(element, diagram, &nodes, &directed_edges)
        else {
            return;
        };
        positions = computed;
        bbox_str = bbox;
    }

    let edge_stroke = element.borrow().get_or("edge-stroke", "black");
    let edge_thickness = element.borrow().get_or("edge-thickness", "2");
    let edge_dash = element.borrow().get_or("edge-dash", "none");
    let mut node_fill = element.borrow().get_or("node-fill", "darkorange");
    let mut node_stroke = element.borrow().get_or("node-stroke", "black");
    let node_thickness = element.borrow().get_or("node-thickness", "1");
    let node_style = element.borrow().get_or("node-style", "circle");
    let labels = element.borrow().get_or("labels", "no") == "yes";
    let default_node_size = if labels { "12" } else { "10" };
    let node_size = element.borrow().get_or("node-size", default_node_size);
    let mid_arrows = element.borrow().get_or("arrows", "end") == "middle";

    if diagram.output_format() == "tactile" {
        node_fill = "white".to_string();
        node_stroke = "black".to_string();
    }

    let mut arrow_buffer = 3.0;
    let mut spread = 15.0;
    let spread_attr = element.borrow().get("edge-spread");
    if let Some(attr) = spread_attr {
        if let Some(s) = diagram
            .ctx
            .valid_eval(&attr)
            .ok()
            .and_then(|v| v.as_num().ok())
        {
            spread = s;
        }
    }
    if diagram.output_format() == "tactile" {
        arrow_buffer = 12.0;
        spread = 20.0;
    }

    // The future CTM is the coordinate system the nodes will be drawn in:
    // for auto-layout that's the <coordinates> system defined by bbox_str; for
    // explicit positions it's the current CTM.
    let future_ctm: CTM = if auto_layout {
        future_ctm_for_bbox(diagram, &bbox_str)
    } else {
        diagram.ctm().clone()
    };
    {
        let mut el = element.borrow_mut();
        el.attrs.clear();
        el.text = None;
        el.children.clear();
        if auto_layout {
            el.tag = "coordinates".to_string();
            el.set("bbox", &bbox_str);
        } else {
            el.tag = "group".to_string();
        }
    }

    let edge_group = xml::sub_element(element, "group");
    edge_group.borrow_mut().set("outline", "tactile");

    // directions entering/leaving each node, for placing loops
    let mut edge_directions: IndexMap<String, Vec<Point>> = IndexMap::new();

    for ((handle_0, handle_1), edges) in &directed_edges {
        let mut endpoints = [handle_0.clone(), handle_1.clone()];
        endpoints.sort();
        let mut y = (all_edges
            .get(&(endpoints[0].clone(), endpoints[1].clone()))
            .copied()
            .unwrap_or(1) as f64
            - 1.0)
            / 2.0
            * spread;
        for (num, edge) in edges.iter().enumerate() {
            let mut ctm = CTM::new();
            let user_p0 = positions[handle_0];
            let user_p1 = positions[handle_1];
            let p0 = future_ctm.transform(user_p0);
            let p1 = future_ctm.transform(user_p1);
            let u = [p1[0] - p0[0], p1[1] - p0[1]];
            let angle = u[1].atan2(u[0]);
            let edge_len = length(u);
            ctm.translate(p0[0], p0[1]);
            ctm.rotate(angle, false);
            let center = future_ctm.inverse_transform(ctm.transform([edge_len / 2.0, y]));
            let c1 = future_ctm.inverse_transform(ctm.transform([edge_len / 4.0, y]));
            let c2 = future_ctm.inverse_transform(ctm.transform([3.0 * edge_len / 4.0, y]));

            edge_directions
                .entry(handle_0.clone())
                .or_default()
                .push(c1);
            edge_directions
                .entry(handle_1.clone())
                .or_default()
                .push(c2);

            let mut handle = format!("edge-{handle_0}-{handle_1}");
            if edges.len() > 1 {
                handle += &format!("-{num}");
            }
            let path = xml::sub_element(&edge_group, "path");
            path.borrow_mut().set("at", &handle);
            if directed {
                if mid_arrows {
                    path.borrow_mut().set("mid-arrow", "yes");
                } else {
                    path.borrow_mut().set("arrows", "1");
                }
            }

            match edge {
                None => {
                    let mut p = path.borrow_mut();
                    p.set("stroke", &edge_stroke);
                    p.set("thickness", &edge_thickness);
                    p.set("dash", &edge_dash);
                }
                Some(edge) => {
                    {
                        let mut p = path.borrow_mut();
                        p.set("stroke", &edge.borrow().get_or("stroke", &edge_stroke));
                        p.set(
                            "thickness",
                            &edge.borrow().get_or("thickness", &edge_thickness),
                        );
                        p.set("dash", &edge.borrow().get_or("dash", &edge_dash));
                    }

                    // does this edge have a label?
                    let has_content = !edge.borrow().children.is_empty()
                        || edge
                            .borrow()
                            .text
                            .as_ref()
                            .is_some_and(|t| !t.trim().is_empty());
                    if has_content {
                        let location_attr = edge.borrow().get_or("label-location", "0.5");
                        let anchor = if location_attr == "0.5" {
                            center
                        } else {
                            let location = diagram
                                .ctx
                                .valid_eval(&location_attr)
                                .ok()
                                .and_then(|v| v.as_num().ok())
                                .unwrap_or(0.5);
                            if location < 0.5 {
                                eval_bezier2(user_p0, c1, center, 2.0 * location)
                            } else {
                                eval_bezier2(center, c2, user_p1, 2.0 * (location - 0.5))
                            }
                        };
                        let direction = [user_p1[0] - user_p0[0], user_p1[1] - user_p0[1]];
                        let label_direction = if y >= 0.0 {
                            rotate_vec(direction, -std::f64::consts::FRAC_PI_2)
                        } else {
                            rotate_vec(direction, std::f64::consts::FRAC_PI_2)
                        };
                        let alignment = label::get_alignment_from_direction(label_direction);
                        let label_element = xml::deep_copy(edge);
                        label_element.borrow_mut().tag = "label".to_string();
                        xml::append(&edge_group, &label_element);
                        if label_element.borrow().get("alignment").is_none() {
                            label_element.borrow_mut().set("alignment", &alignment);
                        }
                        label_element
                            .borrow_mut()
                            .set("anchor", &fmt_point(anchor));
                    }
                }
            }

            if y.abs() < 1e-10 && directed {
                // it's a straight line
                path.borrow_mut().tag = "line".to_string();
                if mid_arrows {
                    path.borrow_mut().set(
                        "endpoints",
                        &format!("{},{}", fmt_point(user_p0), fmt_point(user_p1)),
                    );
                    path.borrow_mut().set("arrows", "0");
                    path.borrow_mut().set("additional-arrows", "(0.5)");
                    y -= spread;
                    continue;
                }
                let mut segment = [center, user_p1];
                let end_style = match nodes.get(handle_1).and_then(|n| n.clone()) {
                    None => node_style.clone(),
                    Some(node) => node.borrow().get_or("style", &node_style),
                };
                let node_size_f: f64 = node_size.parse().unwrap_or(10.0);
                for _ in 0..10 {
                    let (q0, q1) = (segment[0], segment[1]);
                    let c = [0.5 * (q0[0] + q1[0]), 0.5 * (q0[1] + q1[1])];
                    if point::inside(
                        c,
                        user_p1,
                        node_size_f,
                        &end_style,
                        &future_ctm,
                        arrow_buffer,
                    ) {
                        segment = [q0, c];
                    } else {
                        segment = [c, q1];
                    }
                }
                path.borrow_mut().set(
                    "endpoints",
                    &format!("{},{}", fmt_point(user_p0), fmt_point(segment[0])),
                );
                y -= spread;
                continue;
            }

            path.borrow_mut()
                .set("start", &pt2long_str(user_p0, ","));
            let curveto = xml::sub_element(&path, "quadratic-bezier");
            curveto.borrow_mut().set(
                "controls",
                &format!("{},{}", fmt_point(c1), fmt_point(center)),
            );

            if !directed || mid_arrows {
                let curveto = xml::sub_element(&path, "quadratic-bezier");
                curveto.borrow_mut().set(
                    "controls",
                    &format!("{},{}", fmt_point(c2), fmt_point(user_p1)),
                );
            } else {
                let mut current = [center, c2, user_p1];
                let end_style = match nodes.get(handle_1).and_then(|n| n.clone()) {
                    None => node_style.clone(),
                    Some(node) => node.borrow().get_or("style", &node_style),
                };
                let node_size_f: f64 = node_size.parse().unwrap_or(10.0);
                for _ in 0..6 {
                    let (p0c, p1c, p2c) = (current[0], current[1], current[2]);
                    let c0 = [0.5 * (p0c[0] + p1c[0]), 0.5 * (p0c[1] + p1c[1])];
                    let c1m = [0.5 * (p1c[0] + p2c[0]), 0.5 * (p1c[1] + p2c[1])];
                    let mid = [0.5 * (c0[0] + c1m[0]), 0.5 * (c0[1] + c1m[1])];
                    if point::inside(
                        mid,
                        user_p1,
                        node_size_f,
                        &end_style,
                        &future_ctm,
                        arrow_buffer,
                    ) {
                        current = [p0c, c0, mid];
                    } else {
                        current = [mid, c1m, p2c];
                        let curveto = xml::sub_element(&path, "quadratic-bezier");
                        curveto.borrow_mut().set(
                            "controls",
                            &format!("{},{}", fmt_point(c0), fmt_point(mid)),
                        );
                    }
                }
            }
            y -= spread;
        }
    }

    // now the loops
    for (node, loop_record) in &loops {
        let node_element = nodes.get(node).and_then(|n| n.clone());
        let loop_orientation = node_element
            .as_ref()
            .and_then(|n| n.borrow().get("loop-orientation"));
        let directions = edge_directions.get(node).cloned();

        let (loop_angle, loop_gap) = match directions {
            // With incoming/outgoing edges and no forced orientation, aim the loop
            // into the widest angular gap between them.
            Some(dirs) if loop_orientation.is_none() => {
                let node_position = future_ctm.transform(positions[node]);
                let mut angles: Vec<f64> = dirs
                    .iter()
                    .map(|d| {
                        let target = future_ctm.transform(*d);
                        (target[1] - node_position[1]).atan2(target[0] - node_position[0])
                    })
                    .collect();
                angles.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                angles.push(angles[0] + 2.0 * std::f64::consts::PI);
                let mut max_gap = 0;
                let mut max_gap_size = f64::NEG_INFINITY;
                for i in 0..angles.len() - 1 {
                    let gap = angles[i + 1] - angles[i];
                    if gap > max_gap_size {
                        max_gap_size = gap;
                        max_gap = i;
                    }
                }
                (
                    (angles[max_gap + 1] + angles[max_gap]) / 2.0,
                    (0.5 * max_gap_size).min(std::f64::consts::PI / 1.75),
                )
            }
            // No edges, or an explicit orientation: use that orientation (or 0).
            _ => {
                let loop_angle = match &loop_orientation {
                    Some(attr) => -diagram
                        .ctx
                        .valid_eval(attr)
                        .ok()
                        .and_then(|v| v.as_num().ok())
                        .unwrap_or(0.0)
                        .to_radians(),
                    None => 0.0,
                };
                (loop_angle, std::f64::consts::PI / 1.75)
            }
        };
        let _ = loop_gap; // matches Python, which computes but never uses it

        let node_position = positions[node];
        let p_svg = future_ctm.transform(node_position);
        let node_size_f: f64 = node_size.parse().unwrap_or(10.0);
        for (j, loop_edge) in loop_record.iter().enumerate() {
            let mut ctm = CTM::new();
            ctm.translate(p_svg[0], p_svg[1]);
            ctm.rotate(loop_angle, false);
            let scale = (2.0 - 0.75 * j as f64) * node_size_f;
            ctm.scale(scale, scale);

            let mut loop_scale = vec![1.0, 1.0];
            if let Some(gs) = &global_loop_scale {
                loop_scale = gs.clone();
            }
            if let Some(loop_edge) = loop_edge {
                let local = loop_edge.borrow().get("loop-scale");
                if let Some(attr) = local {
                    if let Some(v) = diagram
                        .ctx
                        .valid_eval(&attr)
                        .ok()
                        .and_then(|v| v.as_vec_f64().ok())
                    {
                        loop_scale = v;
                    }
                }
            }
            ctm.scale(loop_scale[0], loop_scale[1]);

            let alpha = 4.0 / 3.0;
            let pts: Vec<Point> = [
                [0.0, -alpha],
                [2.0, -alpha],
                [2.0, 0.0],
                [2.0, alpha],
                [0.0, alpha],
            ]
            .iter()
            .map(|&p| future_ctm.inverse_transform(ctm.transform(p)))
            .collect();
            let (p1, p2, p3, p4, p5) = (pts[0], pts[1], pts[2], pts[3], pts[4]);

            let loop_curves = [
                [node_position, p1, p2, p3],
                [p3, p4, p5, node_position],
            ];

            let path = xml::sub_element(&edge_group, "path");
            let mut handle = format!("loop-{node}");
            if loop_record.len() > 1 {
                handle += &format!("-{j}");
            }
            path.borrow_mut().set("at", &handle);
            path.borrow_mut()
                .set("start", &fmt_point(node_position));
            if directed {
                if mid_arrows {
                    path.borrow_mut().set("mid-arrow", "yes");
                } else {
                    path.borrow_mut().set("arrows", "1");
                }
            }

            let set_style = |path: &El| {
                let mut p = path.borrow_mut();
                match loop_edge {
                    None => {
                        p.set("stroke", &edge_stroke);
                        p.set("thickness", &edge_thickness);
                        p.set("dash", &edge_dash);
                    }
                    Some(edge) => {
                        p.set("stroke", &edge.borrow().get_or("stroke", &edge_stroke));
                        p.set(
                            "thickness",
                            &edge.borrow().get_or("thickness", &edge_thickness),
                        );
                        p.set("dash", &edge.borrow().get_or("dash", &edge_dash));
                    }
                }
            };
            set_style(&path);

            let curveto = xml::sub_element(&path, "cubic-bezier");
            curveto.borrow_mut().set(
                "controls",
                &format!(
                    "({},{},{})",
                    fmt_point(p1),
                    fmt_point(p2),
                    fmt_point(p3)
                ),
            );

            if !directed || mid_arrows {
                let curveto = xml::sub_element(&path, "cubic-bezier");
                curveto.borrow_mut().set(
                    "controls",
                    &format!(
                        "({},{},{})",
                        fmt_point(p4),
                        fmt_point(p5),
                        fmt_point(node_position)
                    ),
                );
            } else {
                let mut current = loop_curves[1];
                let end_style = match &node_element {
                    None => node_style.clone(),
                    Some(node) => node.borrow().get_or("style", &node_style),
                };
                for _ in 0..6 {
                    let (q0, q1, q2, q3) = (current[0], current[1], current[2], current[3]);
                    let p01 = [0.5 * (q0[0] + q1[0]), 0.5 * (q0[1] + q1[1])];
                    let p12 = [0.5 * (q1[0] + q2[0]), 0.5 * (q1[1] + q2[1])];
                    let p23 = [0.5 * (q2[0] + q3[0]), 0.5 * (q2[1] + q3[1])];
                    let r1 = [0.5 * (p01[0] + p12[0]), 0.5 * (p01[1] + p12[1])];
                    let r2 = [0.5 * (p12[0] + p23[0]), 0.5 * (p12[1] + p23[1])];
                    let mid = [0.5 * (r1[0] + r2[0]), 0.5 * (r1[1] + r2[1])];
                    if point::inside(
                        mid,
                        node_position,
                        node_size_f,
                        &end_style,
                        &future_ctm,
                        arrow_buffer,
                    ) {
                        current = [q0, p01, r1, mid];
                    } else {
                        current = [mid, r2, p23, q3];
                        let curveto = xml::sub_element(&path, "cubic-bezier");
                        curveto.borrow_mut().set(
                            "controls",
                            &format!(
                                "{},{},{}",
                                fmt_point(p01),
                                fmt_point(r1),
                                fmt_point(mid)
                            ),
                        );
                    }
                }
            }

            set_style(&path);

            // does this loop have a label?
            if let Some(loop_edge) = loop_edge {
                let has_content = !loop_edge.borrow().children.is_empty()
                    || loop_edge
                        .borrow()
                        .text
                        .as_ref()
                        .is_some_and(|t| !t.trim().is_empty());
                if has_content {
                    let location_attr = loop_edge.borrow().get_or("label-location", "0.5");
                    let location = diagram
                        .ctx
                        .valid_eval(&location_attr)
                        .ok()
                        .and_then(|v| v.as_num().ok())
                        .unwrap_or(0.5);
                    let (anchor, anchor_ep) = if location < 0.5 {
                        (
                            eval_bezier3(&loop_curves[0], 2.0 * location),
                            eval_bezier3(&loop_curves[0], 2.0 * location + 0.0001),
                        )
                    } else {
                        (
                            eval_bezier3(&loop_curves[1], 2.0 * (location - 0.5)),
                            eval_bezier3(&loop_curves[1], 2.0 * (location + 0.0001 - 0.5)),
                        )
                    };
                    let direction = [anchor_ep[0] - anchor[0], anchor_ep[1] - anchor[1]];
                    let label_direction =
                        rotate_vec(direction, std::f64::consts::FRAC_PI_2);
                    let alignment = label::get_alignment_from_direction(label_direction);

                    let label_element = xml::deep_copy(loop_edge);
                    label_element.borrow_mut().tag = "label".to_string();
                    xml::append(&edge_group, &label_element);
                    if label_element.borrow().get("alignment").is_none() {
                        label_element.borrow_mut().set("alignment", &alignment);
                        label_element
                            .borrow_mut()
                            .set("anchor", &fmt_point(anchor));
                    }
                }
            }
        }
    }

    // the nodes
    let node_group = xml::sub_element(element, "group");
    node_group.borrow_mut().set("outline", "tactile");

    for (handle, position) in &positions {
        let node = nodes.get(handle).and_then(|n| n.clone());
        let p = xml::sub_element(&node_group, "point");
        {
            let mut pt = p.borrow_mut();
            pt.set("p", &fmt_point(*position));
            pt.set("size", &node_size);
            pt.set("at", &format!("node-{handle}"));
            match &node {
                None => {
                    pt.set("fill", &node_fill);
                    pt.set("stroke", &node_stroke);
                    pt.set("thickness", &node_thickness);
                    pt.set("style", &node_style);
                }
                Some(node) => {
                    pt.set("stroke", &node.borrow().get_or("stroke", &node_stroke));
                    pt.set(
                        "thickness",
                        &node.borrow().get_or("thickness", &node_thickness),
                    );
                    pt.set("fill", &node.borrow().get_or("fill", &node_fill));
                    pt.set("style", &node.borrow().get_or("style", &node_style));
                }
            }
        }

        if labels {
            let mut label_element = None;
            if let Some(node) = &node {
                let has_content = !node.borrow().children.is_empty()
                    || node
                        .borrow()
                        .text
                        .as_ref()
                        .is_some_and(|t| !t.trim().is_empty());
                if has_content {
                    let el = xml::deep_copy(node);
                    el.borrow_mut().tag = "label".to_string();
                    xml::append(&node_group, &el);
                    label_element = Some(el);
                }
            }
            let label_element = label_element.unwrap_or_else(|| {
                let el = xml::sub_element(&node_group, "label");
                let math_element = xml::sub_element(&el, "m");
                let text = label_dictionary
                    .get(handle)
                    .cloned()
                    .unwrap_or_else(|| handle.clone());
                math_element.borrow_mut().text = Some(text);
                el
            });
            let mut l = label_element.borrow_mut();
            l.set("p", &fmt_point(*position));
            l.set("alignment", "center");
            l.set("offset", "(0,0)");
            l.set("clear-background", "no");
        }
    }

    if auto_layout {
        crate::core::coordinates::coordinates(element, diagram, parent, outline_group);
    } else {
        group::group(element, diagram, parent, outline_group);
    }
    let _ = util::float2str(0.0);
    let _ = interp_call;
}

/// Run the requested layout algorithm and normalize the positions, returning
/// the positions and the bbox string for the <coordinates> system. Mirrors the
/// networkx-layout branch of network.py (§14.2: not coordinate-identical).
fn compute_layout(
    element: &El,
    diagram: &mut Diagram,
    nodes: &IndexMap<String, Option<El>>,
    directed_edges: &IndexMap<(String, String), Vec<Option<El>>>,
) -> Option<(IndexMap<String, Point>, String)> {
    use crate::core::network_layout::{self as layout, Graph};

    let node_names: Vec<String> = nodes.keys().cloned().collect();
    let mut edges: Vec<(String, String)> = Vec::new();
    for ((a, b), multi) in directed_edges {
        for _ in 0..multi.len() {
            edges.push((a.clone(), b.clone()));
        }
    }
    let graph = Graph::new(&node_names, &edges);

    let layout_name = element.borrow().get_or("layout", "spring");
    let seed: u64 = element
        .borrow()
        .get_or("seed", "1")
        .parse()
        .unwrap_or(1);

    let mut positions: IndexMap<String, Point> = match layout_name.as_str() {
        "spring" => layout::spring(&graph, seed),
        "bfs" => {
            let Some(start) = element.borrow().get("start") else {
                log::error!("bfs network layout needs a starting node");
                return None;
            };
            match layout::bfs(&graph, &start) {
                Some(p) => p,
                None => {
                    log::error!("bfs start node {start} is not in the network");
                    return None;
                }
            }
        }
        "spectral" => layout::spectral(&graph),
        "circular" => layout::circular(&graph),
        "random" => layout::random(&graph, seed),
        "planar" => {
            log::info!("planar network layout is approximated by a spring layout");
            layout::spring(&graph, seed)
        }
        "bipartite" => {
            let alignment = element.borrow().get_or("alignment", "horizontal");
            let Some(set_attr) = element.borrow().get("bipartite-set") else {
                log::error!("A bipartite network needs a @bipartite-set attribute");
                return None;
            };
            let set: Vec<String> = diagram
                .ctx
                .valid_eval(&set_attr)
                .ok()
                .and_then(|v| match v {
                    Value::Array(items) => Some(items.iter().map(|i| i.to_py_str()).collect()),
                    _ => None,
                })
                .unwrap_or_default();
            layout::bipartite(&graph, &set, alignment == "horizontal")
        }
        other => {
            log::error!("Unknown network layout: {other}");
            return None;
        }
    };

    // normalize: center, rotate, re-center, then scale the bbox (network.py)
    let scale: f64 = element.borrow().get_or("scale", "0.8").parse().unwrap_or(0.8);
    let rotate: f64 = element.borrow().get_or("rotate", "0").parse().unwrap_or(0.0);

    let bounds = |p: &IndexMap<String, Point>| -> ([f64; 2], [f64; 2]) {
        let xs: Vec<f64> = p.values().map(|q| q[0]).collect();
        let ys: Vec<f64> = p.values().map(|q| q[1]).collect();
        (
            [
                xs.iter().cloned().fold(f64::INFINITY, f64::min),
                ys.iter().cloned().fold(f64::INFINITY, f64::min),
            ],
            [
                xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
                ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            ],
        )
    };

    let (ll, ur) = bounds(&positions);
    let center = [0.5 * (ll[0] + ur[0]), 0.5 * (ll[1] + ur[1])];
    let mut ctm = CTM::new();
    ctm.translate(-center[0], -center[1]);
    ctm.rotate(rotate, true);
    for p in positions.values_mut() {
        *p = ctm.transform(*p);
    }

    let (ll, ur) = bounds(&positions);
    let center = [0.5 * (ll[0] + ur[0]), 0.5 * (ll[1] + ur[1])];
    let mut ctm = CTM::new();
    ctm.translate(-center[0], -center[1]);
    for p in positions.values_mut() {
        *p = ctm.transform(*p);
    }

    let (ll, ur) = bounds(&positions);
    let ll = [ll[0] / scale, ll[1] / scale];
    let ur = [ur[0] / scale, ur[1] / scale];
    let bbox_str = format!(
        "({},{},{},{})",
        py_str(ll[0]),
        py_str(ll[1]),
        py_str(ur[0]),
        py_str(ur[1])
    );
    Some((positions, bbox_str))
}

/// Build the CTM that maps the given bbox into the current coordinate box,
/// matching coordinates.py so edges/loops (drawn before <coordinates> takes
/// effect) land correctly.
fn future_ctm_for_bbox(diagram: &Diagram, bbox_str: &str) -> CTM {
    let inner = bbox_str.trim_start_matches('(').trim_end_matches(')');
    let vals: Vec<f64> = inner
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    if vals.len() != 4 {
        return diagram.ctm_ref().clone();
    }
    let (ll, ur) = ([vals[0], vals[1]], [vals[2], vals[3]]);
    let diagram_bbox = diagram.bbox();
    let mut ctm = diagram.ctm_ref().clone();
    ctm.translate(diagram_bbox[0], diagram_bbox[1]);
    ctm.scale(
        (diagram_bbox[2] - diagram_bbox[0]) / (ur[0] - ll[0]),
        (diagram_bbox[3] - diagram_bbox[1]) / (ur[1] - ll[1]),
    );
    ctm.translate(-ll[0], -ll[1]);
    ctm
}
