//! Port of prefig/core/path.py: author-driven paths with sub-commands and
//! decorations (coil, zigzag, wave, ragged, capacitor).

use crate::core::ctm::CTM;
use crate::core::diagram::Diagram;
use crate::core::math_utilities::length;
use crate::core::utilities::{self as util, pt2long_str, pt2str};
use crate::core::{arrow, tags};
use crate::value::Value;
use crate::xml::{self, El};

type Point = [f64; 2];

pub fn is_path_tag(tag: &str) -> bool {
    matches!(
        tag,
        "moveto"
            | "rmoveto"
            | "lineto"
            | "rlineto"
            | "horizontal"
            | "vertical"
            | "cubic-bezier"
            | "quadratic-bezier"
            | "smooth-cubic"
            | "smooth-quadratic"
    )
}

pub fn path(element: &El, diagram: &mut Diagram, parent: &El, outline_group: Option<&El>) {
    if diagram.output_format() == "tactile" {
        if element.borrow().get("stroke").is_some() {
            element.borrow_mut().set("stroke", "black");
        }
        util::set_tactile_fill(element);
    }
    util::set_attr(element, "stroke", "none", &mut diagram.ctx);
    util::set_attr(element, "fill", "none", &mut diagram.ctx);
    util::set_attr(element, "thickness", "2", &mut diagram.ctx);

    let mut cmds: Vec<String> = vec!["M".to_string()];
    let Some(start_attr) = element.borrow().get("start") else {
        log::error!("A <path> element needs a @start attribute");
        return;
    };
    let Some(user_start) = eval_point(diagram, &start_attr) else {
        log::error!("Error in <path> defining start={start_attr}");
        return;
    };
    let mut current_point = user_start;
    let start = diagram.transform(user_start);
    cmds.push(pt2str(start, " "));

    let children: Vec<El> = element.borrow().children.clone();
    for child in &children {
        log::debug!("Processing {} inside <path>", child.borrow().tag);
        if !process_tag(child, diagram, &mut cmds, &mut current_point) {
            log::error!("Error in <path> processing subelements");
            return;
        }
    }

    if element.borrow().get_or("closed", "no") == "yes" {
        cmds.push("Z".to_string());
    }
    let d = cmds.join(" ");
    let path = xml::new_element("path");
    let id = element.borrow().get("id");
    diagram.add_id(&path, id.as_deref());
    diagram.register_svg_element(element, &path);
    path.borrow_mut().set("d", &d);
    let attrs = util::get_2d_attr(element, diagram);
    util::add_attr(&path, attrs);
    let clip = element.borrow().get_or("cliptobbox", "yes");
    element.borrow_mut().set("cliptobbox", &clip);
    util::cliptobbox(&path, element, diagram);

    let arrows: i64 = element
        .borrow()
        .get_or("arrows", "0")
        .parse()
        .unwrap_or(0);
    let (mut forward, mut backward) = ("marker-end", "marker-start");
    if element.borrow().get_or("reverse", "no") == "yes" {
        std::mem::swap(&mut forward, &mut backward);
    }
    let arrow_width = element.borrow().get("arrow-width");
    let arrow_angles = element.borrow().get("arrow-angles");
    if arrows > 0 {
        arrow::add_arrowhead_to_path(
            diagram,
            forward,
            &path,
            arrow_width.as_deref(),
            arrow_angles.as_deref(),
        );
    }
    if arrows > 1 {
        arrow::add_arrowhead_to_path(
            diagram,
            backward,
            &path,
            arrow_width.as_deref(),
            arrow_angles.as_deref(),
        );
    }
    if element.borrow().get_or("mid-arrow", "no") == "yes" {
        arrow::add_arrowhead_to_path(diagram, "marker-mid", &path, None, None);
    }

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

fn eval_point(diagram: &mut Diagram, attr: &str) -> Option<Point> {
    let v = diagram.ctx.valid_eval(attr).ok()?.as_vec_f64().ok()?;
    (v.len() >= 2).then(|| [v[0], v[1]])
}

fn eval_num(diagram: &mut Diagram, attr: &str) -> Option<f64> {
    diagram.ctx.valid_eval(attr).ok()?.as_num().ok()
}

fn distance_heading_point(diagram: &mut Diagram, child: &El) -> Option<Point> {
    let distance_attr = child.borrow().get("distance")?;
    let distance = eval_num(diagram, &distance_attr)?;
    let heading_attr = child.borrow().get_or("heading", "0");
    let mut heading = eval_num(diagram, &heading_attr)?;
    if child.borrow().get_or("degrees", "yes") == "yes" {
        heading = heading.to_radians();
    }
    Some([distance * heading.cos(), distance * heading.sin()])
}

fn process_tag(
    child: &El,
    diagram: &mut Diagram,
    cmds: &mut Vec<String>,
    current_point: &mut Point,
) -> bool {
    let tag = child.borrow().tag.clone();

    if tag == "moveto" {
        let user_point = if child.borrow().get("distance").is_some() {
            match distance_heading_point(diagram, child) {
                Some(p) => p,
                None => return false,
            }
        } else {
            let point_attr = child.borrow().get("point").unwrap_or_default();
            match eval_point(diagram, &point_attr) {
                Some(p) => p,
                None => {
                    log::error!("Error in <moveto> defining point={point_attr}");
                    return false;
                }
            }
        };
        let point = diagram.transform(user_point);
        cmds.push("M".to_string());
        cmds.push(pt2str(point, " "));
        *current_point = user_point;
        return true;
    }

    if tag == "rmoveto" {
        let user_point = if child.borrow().get("distance").is_some() {
            match distance_heading_point(diagram, child) {
                Some(p) => p,
                None => return false,
            }
        } else {
            let point_attr = child.borrow().get("point").unwrap_or_default();
            match eval_point(diagram, &point_attr) {
                Some(p) => p,
                None => {
                    log::error!("Error in <rmoveto> defining point={point_attr}");
                    return false;
                }
            }
        };
        *current_point = [
            current_point[0] + user_point[0],
            current_point[1] + user_point[1],
        ];
        let point = diagram.transform(*current_point);
        cmds.push("M".to_string());
        cmds.push(pt2str(point, " "));
        return true;
    }

    if tag == "horizontal" {
        let distance_attr = child.borrow().get("distance").unwrap_or_default();
        let Some(distance) = eval_num(diagram, &distance_attr) else {
            log::error!("Error in <horizontal> defining distance={distance_attr}");
            return false;
        };
        let user_point = [current_point[0] + distance, current_point[1]];
        child.borrow_mut().tag = "lineto".to_string();
        child
            .borrow_mut()
            .set("point", &pt2long_str(user_point, ","));
    }

    if child.borrow().tag == "vertical" {
        let distance_attr = child.borrow().get("distance").unwrap_or_default();
        let Some(distance) = eval_num(diagram, &distance_attr) else {
            log::error!("Error in <vertical> defining distance={distance_attr}");
            return false;
        };
        let user_point = [current_point[0], current_point[1] + distance];
        child.borrow_mut().tag = "lineto".to_string();
        child
            .borrow_mut()
            .set("point", &pt2long_str(user_point, ","));
    }

    if child.borrow().tag == "rlineto" {
        let user_point = if child.borrow().get("distance").is_some() {
            match distance_heading_point(diagram, child) {
                Some(p) => p,
                None => return false,
            }
        } else {
            let point_attr = child.borrow().get("point").unwrap_or_default();
            match eval_point(diagram, &point_attr) {
                Some(p) => p,
                None => {
                    log::error!("Error in <rlineto> defining point={point_attr}");
                    return false;
                }
            }
        };
        let user_point = [
            current_point[0] + user_point[0],
            current_point[1] + user_point[1],
        ];
        child.borrow_mut().tag = "lineto".to_string();
        child
            .borrow_mut()
            .set("point", &pt2long_str(user_point, ","));
    }

    if child.borrow().tag == "lineto" {
        if child.borrow().get("point").is_none() && child.borrow().get("distance").is_some() {
            let Some(user_point) = distance_heading_point(diagram, child) else {
                return false;
            };
            child
                .borrow_mut()
                .set("point", &pt2long_str(user_point, ","));
        }

        if child.borrow().get("decoration").is_some() {
            return decorate(child, diagram, current_point, cmds);
        }

        let point_attr = child.borrow().get("point").unwrap_or_default();
        let Some(user_point) = eval_point(diagram, &point_attr) else {
            log::error!("Error in <lineto> defining point={point_attr}");
            return false;
        };
        let point = diagram.transform(user_point);
        cmds.push("L".to_string());
        cmds.push(pt2str(point, " "));
        *current_point = user_point;
        return true;
    }

    if tag == "cubic-bezier" || tag == "quadratic-bezier" {
        cmds.push(if tag == "cubic-bezier" { "C" } else { "Q" }.to_string());
        let controls_attr = child.borrow().get("controls").unwrap_or_default();
        let controls = diagram.ctx.valid_eval(&controls_attr).ok().and_then(|v| {
            let Value::Array(items) = v else { return None };
            items
                .iter()
                .map(|i| {
                    let v = i.as_vec_f64().ok()?;
                    (v.len() >= 2).then(|| [v[0], v[1]])
                })
                .collect::<Option<Vec<Point>>>()
        });
        let Some(user_control_pts) = controls else {
            log::error!("Error in <{tag}> defining controls={controls_attr}");
            return false;
        };
        let control_strs: Vec<String> = user_control_pts
            .iter()
            .map(|&p| pt2str(diagram.transform(p), " "))
            .collect();
        cmds.push(control_strs.join(" "));
        *current_point = user_control_pts[user_control_pts.len() - 1];
        return true;
    }

    if tag == "arc" {
        let center_attr = child.borrow().get("center").unwrap_or_default();
        let radius_attr = child.borrow().get("radius").unwrap_or_default();
        let range_attr = child.borrow().get("range").unwrap_or_default();
        let center = eval_point(diagram, &center_attr);
        let radius = eval_num(diagram, &radius_attr);
        let range = diagram
            .ctx
            .valid_eval(&range_attr)
            .ok()
            .and_then(|v| v.as_vec_f64().ok());
        let (Some(center), Some(radius), Some(mut angular_range)) = (center, radius, range)
        else {
            log::error!("Error in <arc> defining data: @center, @radius, or @range");
            return false;
        };
        if child.borrow().get_or("degrees", "yes") == "yes" {
            angular_range = angular_range.iter().map(|a| a.to_radians()).collect();
        }
        let n = 100;
        let mut t = angular_range[0];
        let dt = (angular_range[1] - angular_range[0]) / n as f64;
        let user_start = [center[0] + radius * t.cos(), center[1] + radius * t.sin()];
        cmds.push("L".to_string());
        cmds.push(pt2str(diagram.transform(user_start), " "));
        for _ in 0..n {
            t += dt;
            let user_point = [center[0] + radius * t.cos(), center[1] + radius * t.sin()];
            cmds.push("L".to_string());
            cmds.push(pt2str(diagram.transform(user_point), " "));
        }
        return true;
    }

    if tag == "repeat" {
        let Some(parameter) = child.borrow().get("parameter") else {
            return false;
        };
        let parsed = parameter.split_once('=').and_then(|(var, expr)| {
            let (start, stop) = expr.split_once("..")?;
            let start = eval_num(diagram, start)? as i64;
            let stop = eval_num(diagram, stop)? as i64;
            Some((var.trim().to_string(), start, stop))
        });
        let Some((var, start, stop)) = parsed else {
            log::error!("Error in <repeat> defining parameter={parameter}");
            return false;
        };
        for k in start..=stop {
            let _ = diagram
                .ctx
                .valid_eval_named(&k.to_string(), Some(&var), true);
            let sub_children: Vec<El> = child.borrow().children.clone();
            for sub_child in &sub_children {
                if !process_tag(sub_child, diagram, cmds, current_point) {
                    return false;
                }
            }
        }
        return true;
    }

    if matches!(tag.as_str(), "graph" | "parametric-curve" | "polygon" | "spline") {
        let dummy_parent = xml::new_element("group");
        let _ = tags::parse_element(child, diagram, &dummy_parent, None);
        let first_child = dummy_parent.borrow().children.first().cloned();
        let Some(first_child) = first_child else {
            return false;
        };
        let mut child_cmds = first_child.borrow().get_or("d", "").trim().to_string();
        if child_cmds.starts_with('M') {
            child_cmds = format!("L{}", &child_cmds[1..]);
        }
        if child_cmds.ends_with('Z') {
            child_cmds = child_cmds[..child_cmds.len() - 1].trim().to_string();
        }
        cmds.push(child_cmds.clone());
        let coordinates: Vec<&str> = child_cmds.split_whitespace().collect();
        if coordinates.len() >= 2 {
            let final_point = [
                coordinates[coordinates.len() - 2].parse().unwrap_or(0.0),
                coordinates[coordinates.len() - 1].parse().unwrap_or(0.0),
            ];
            *current_point = diagram.inverse_transform(final_point);
        }
        return true;
    }

    log::warn!("Unknown tag in <path>: {tag}");
    true
}

/// numpy's MT19937 random_sample, for the ragged decoration's seeded jitter.
struct NumpyRandom {
    mt: [u32; 624],
    index: usize,
}

impl NumpyRandom {
    fn new(seed: u32) -> NumpyRandom {
        let mut mt = [0u32; 624];
        mt[0] = seed;
        for i in 1..624 {
            mt[i] = 1812433253u32
                .wrapping_mul(mt[i - 1] ^ (mt[i - 1] >> 30))
                .wrapping_add(i as u32);
        }
        NumpyRandom { mt, index: 624 }
    }

    fn generate(&mut self) {
        for i in 0..624 {
            let y = (self.mt[i] & 0x8000_0000) | (self.mt[(i + 1) % 624] & 0x7fff_ffff);
            let mut next = y >> 1;
            if y & 1 != 0 {
                next ^= 0x9908_b0df;
            }
            self.mt[i] = self.mt[(i + 397) % 624] ^ next;
        }
        self.index = 0;
    }

    fn next_u32(&mut self) -> u32 {
        if self.index >= 624 {
            self.generate();
        }
        let mut y = self.mt[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^= y >> 18;
        y
    }

    /// numpy random_sample: 53-bit double in [0, 1).
    fn random(&mut self) -> f64 {
        let a = (self.next_u32() >> 5) as f64;
        let b = (self.next_u32() >> 6) as f64;
        (a * 67108864.0 + b) / 9007199254740992.0
    }
}

fn decorate(
    child: &El,
    diagram: &mut Diagram,
    current_point: &mut Point,
    cmds: &mut Vec<String>,
) -> bool {
    let point_attr = child.borrow().get("point").unwrap_or_default();
    let Some(user_point) = eval_point(diagram, &point_attr) else {
        return false;
    };
    let mut ctm = CTM::new();
    let p0 = diagram.transform(*current_point);
    let p1 = diagram.transform(user_point);
    let diff = [p1[0] - p0[0], p1[1] - p0[1]];
    let seg_length = length(diff);
    ctm.translate(p0[0], p0[1]);
    ctm.rotate(diff[1].atan2(diff[0]), false);

    let decoration = child.borrow().get_or("decoration", "");
    let decoration_data: Vec<String> = decoration
        .split(';')
        .map(|d| d.trim().to_string())
        .collect();
    let data: std::collections::HashMap<String, String> = decoration_data[1..]
        .iter()
        .filter_map(|d| {
            d.split_once('=')
                .map(|(k, v)| (k.to_string(), v.to_string()))
        })
        .collect();

    let kind = decoration_data[0].as_str();
    let periodic = matches!(kind, "coil" | "zigzag" | "wave");
    if periodic {
        let dimensions = data
            .get("dimensions")
            .and_then(|d| {
                diagram
                    .ctx
                    .valid_eval(d)
                    .ok()
                    .and_then(|v| v.as_vec_f64().ok())
            })
            .unwrap_or(vec![10.0, 5.0]);
        let location = data
            .get("center")
            .and_then(|d| eval_num(diagram, d))
            .unwrap_or(0.5);
        let mut number = match data.get("number") {
            None => ((seg_length - dimensions[0] / 2.0) / dimensions[0]).floor(),
            Some(n) => eval_num(diagram, n).unwrap_or(1.0),
        };

        match kind {
            "coil" => {
                let mut half_fraction = (number + 0.5) * dimensions[0] / seg_length;
                while location - half_fraction < 0.0 || location + half_fraction > 1.0 {
                    number -= 1.0;
                    half_fraction = (number + 0.5) * dimensions[0] / seg_length;
                }
                let start_coil = seg_length * (location - half_fraction);
                let coil_length = 2.0 * half_fraction * seg_length;

                let n = 40;
                let dt = 2.0 * std::f64::consts::PI / n as f64;
                let mut t: f64 = 0.0;
                let x_init = start_coil;
                let mut x_pos = x_init + dimensions[0] / 2.0;
                let iterates = ((number + 0.5) * n as f64).floor() as usize;
                cmds.push("L".to_string());
                cmds.push(pt2str(ctm.transform([x_init, 0.0]), " "));
                let dx = (coil_length - dimensions[0]) / iterates as f64;
                let mut x = x_pos;
                for _ in 0..iterates {
                    let y = -dimensions[1] * t.sin();
                    x_pos += dx;
                    x = x_pos - dimensions[0] / 2.0 * t.cos();
                    t += dt;
                    cmds.push("L".to_string());
                    cmds.push(pt2str(ctm.transform([x, y]), " "));
                }
                cmds.push("L".to_string());
                cmds.push(pt2str(ctm.transform([x, 0.0]), " "));
                cmds.push("L".to_string());
                cmds.push(pt2str(ctm.transform([seg_length, 0.0]), " "));
            }
            "zigzag" | "wave" => {
                let mut half_fraction = number * dimensions[0] / seg_length;
                while location - half_fraction < 0.0 || location + half_fraction > 1.0 {
                    number -= 1.0;
                    half_fraction = number * dimensions[0] / seg_length;
                }
                let start = seg_length * (location - half_fraction);
                let deco_length = 2.0 * half_fraction * seg_length;

                let n = if kind == "zigzag" { 4 } else { 30 };
                let dt = 2.0 * std::f64::consts::PI / n as f64;
                let mut t: f64 = 0.0;
                let mut x_pos = start;
                let iterates = (number * n as f64).floor() as usize;
                cmds.push("L".to_string());
                cmds.push(pt2str(ctm.transform([x_pos, 0.0]), " "));
                let dx = deco_length / iterates as f64;
                for _ in 0..iterates {
                    t += dt;
                    x_pos += dx;
                    let y = -dimensions[1] * t.sin();
                    cmds.push("L".to_string());
                    cmds.push(pt2str(ctm.transform([x_pos, y]), " "));
                }
                cmds.push("L".to_string());
                cmds.push(pt2str(ctm.transform([x_pos, 0.0]), " "));
                cmds.push("L".to_string());
                cmds.push(pt2str(ctm.transform([seg_length, 0.0]), " "));
            }
            _ => {}
        }
    }

    if kind == "ragged" {
        let Some(offset) = data.get("offset").and_then(|d| eval_num(diagram, d)) else {
            log::error!("Error in retrieving the step and an offset in a ragged decoration");
            return false;
        };
        let Some(step) = data.get("step").and_then(|d| eval_num(diagram, d)) else {
            log::error!("Error in retrieving the step and an offset in a ragged decoration");
            return false;
        };
        let seed: u32 = data
            .get("seed")
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);
        let mut rng = NumpyRandom::new(seed);
        let mut x_pos = 0.0;
        cmds.push("L".to_string());
        cmds.push(pt2str(ctm.transform([0.0, 0.0]), " "));
        while x_pos < seg_length - step {
            let y_pos = 2.0 * offset * (rng.random() - 0.5);
            x_pos += (0.5 * rng.random() + 0.75) * step;
            cmds.push("L".to_string());
            cmds.push(pt2str(ctm.transform([x_pos, y_pos]), " "));
        }
        cmds.push("L".to_string());
        cmds.push(pt2str(ctm.transform([seg_length, 0.0]), " "));
    }

    if kind == "capacitor" {
        let dimensions = data
            .get("dimensions")
            .and_then(|d| {
                diagram
                    .ctx
                    .valid_eval(d)
                    .ok()
                    .and_then(|v| v.as_vec_f64().ok())
            })
            .unwrap_or(vec![10.0, 5.0]);
        let location = data
            .get("center")
            .and_then(|d| eval_num(diagram, d))
            .unwrap_or(0.5);
        let x_mid = seg_length * location;
        let x0 = x_mid - dimensions[0] / 2.0;
        let x1 = x_mid + dimensions[0] / 2.0;

        for (cmd, p) in [
            ("L", [x0, 0.0]),
            ("M", [x0, dimensions[1]]),
            ("L", [x0, -dimensions[1]]),
            ("M", [x1, dimensions[1]]),
            ("L", [x1, -dimensions[1]]),
            ("M", [x1, 0.0]),
            ("L", [seg_length, 0.0]),
        ] {
            cmds.push(cmd.to_string());
            cmds.push(pt2str(ctm.transform(p), " "));
        }
    }

    *current_point = user_point;
    true
}

fn finish_outline(element: &El, diagram: &mut Diagram, parent: &El) {
    let stroke = element.borrow().get("stroke");
    let thickness = element.borrow().get("thickness");
    let fill = element.borrow().get_or("fill", "none");
    diagram.finish_outline(element, stroke, thickness, &fill, parent);
}
