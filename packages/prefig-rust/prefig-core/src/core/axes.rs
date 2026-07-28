//! Port of prefig/core/axes.py: coordinate axes with ticks and labels.

use crate::core::ctm::AxisScale;
use crate::core::diagram::Diagram;
use crate::core::line::{mk_line, EndpointOffsets};
use crate::core::math_utilities::{fmt_g, linspace};
use crate::core::utilities::{self as util, float2str, pt2str};
use crate::core::{arrow, label};
use crate::value::{py_str, Value};
use crate::xml::{self, El};

pub fn is_axes_tag(tag: &str) -> bool {
    matches!(tag, "xlabel" | "ylabel")
}

fn label_delta(key: i64) -> f64 {
    match key {
        2 => 0.2,
        3 | 4 => 0.5,
        12..=20 => 2.0,
        _ => 1.0,
    }
}

/// Automate finding tick/label positions: returns (x0, dx, x1).
pub fn find_label_positions(range: [f64; 2], pi_format: bool) -> (f64, f64, f64) {
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
    if dx > 1.0 {
        dx *= label_delta((2.0 * distance).round_ties_even() as i64);
        dx = dx.trunc();
    } else {
        dx *= label_delta((2.0 * distance).round_ties_even() as i64);
    }
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
    (x0, dx, x1)
}

/// Label positions on a log axis: r has 2 (auto) or 3 (user) entries.
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
            2.0
        } else if width <= 10.0 {
            1.0
        } else {
            5.0 / width
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

/// A LaTeX string for x*pi.
pub fn get_pi_text(x: f64) -> String {
    if (x.abs() - 1.0).abs() < 1e-10 {
        return if x < 0.0 { r"-\pi" } else { r"\pi" }.to_string();
    }
    if (x - x.round_ties_even()).abs() < 1e-10 {
        return format!("{}\\pi", x.round_ties_even() as i64);
    }
    for denom in [4i64, 2, 3, 6] {
        let scaled = denom as f64 * x;
        if (scaled - scaled.round_ties_even()).abs() < 1e-10 {
            let num = scaled.round_ties_even() as i64;
            if denom == 4 && num % 2 != 1 && num != -1 {
                continue;
            }
            if num == -1 {
                return format!("-\\frac{{\\pi}}{{{denom}}}");
            }
            if num == 1 {
                return format!("\\frac{{\\pi}}{{{denom}}}");
            }
            if num < 0 {
                return format!("-\\frac{{{}\\pi}}{{{denom}}}", -num);
            }
            return format!("\\frac{{{num}\\pi}}{{{denom}}}");
        }
    }
    format!("{}\\pi", fmt_g(x))
}

/// State the tick-mark handler needs from the most recent <axes>.
#[derive(Clone)]
pub struct AxesInfo {
    pub y_axis_location: f64,
    pub x_axis_location: f64,
    pub top_labels: bool,
    pub right_labels: bool,
    pub h_tick_direction: f64,
    pub v_tick_direction: f64,
    pub ticksize: [f64; 2],
    pub stroke: String,
    pub thickness: String,
}

struct AxesBuilder {
    tactile: bool,
    stroke: String,
    thickness: String,
    axes: El,
    axes_attribute: Option<String>,
    horizontal_axis: bool,
    vertical_axis: bool,
    clear_background: String,
    decorations: String,
    h_pi_format: bool,
    v_pi_format: bool,
    ticksize: [f64; 2],
    bbox: [f64; 4],
    position_tolerance: f64,
    arrows: i64,
    h_tick_group: El,
    v_tick_group: El,

    y_axis_location: f64,
    y_axis_offsets: [f64; 2],
    h_zero_include: bool,
    top_labels: bool,
    h_exclude: Vec<f64>,
    h_zero_label: bool,
    h_tick_direction: f64,

    x_axis_location: f64,
    x_axis_offsets: [f64; 2],
    v_zero_include: bool,
    right_labels: bool,
    v_exclude: Vec<f64>,
    v_zero_label: bool,
    v_tick_direction: f64,
}

pub fn axes(element: &El, diagram: &mut Diagram, parent: &El, _outline_group: Option<&El>) {
    let tactile = diagram.output_format() == "tactile";
    let stroke = element.borrow().get_or("stroke", "black");
    let thickness = element.borrow().get_or("thickness", "2");

    let id = element.borrow().get_or("id", "axes");
    let default_id = diagram.prepend_id_prefix(&id);
    let axes_g = xml::sub_element(parent, "g");
    {
        let mut a = axes_g.borrow_mut();
        a.set("id", &default_id);
        a.set("stroke", &stroke);
        a.set("stroke-width", &thickness);
    }
    util::cliptobbox(&axes_g, element, diagram);
    diagram.register_svg_element(element, &axes_g);

    let mut axes_attribute = element.borrow().get("axes");
    if axes_attribute.as_deref() == Some("all") {
        axes_attribute = None;
        element.borrow_mut().pop_attr("axes");
    }
    let horizontal_axis = element.borrow().get_or("axes", "horizontal") == "horizontal";
    let vertical_axis = element.borrow().get_or("axes", "vertical") == "vertical";

    let clear_background = element.borrow().get_or("clear-background", "no");
    let decorations = element.borrow().get_or("decorations", "yes");
    let h_pi_format = element.borrow().get_or("h-pi-format", "no") == "yes";
    let v_pi_format = element.borrow().get_or("v-pi-format", "no") == "yes";
    let ticksize = if tactile {
        [18.0, 0.0]
    } else {
        let tick_attr = element.borrow().get("tick-size");
        match tick_attr {
            Some(attr) => match diagram.ctx.valid_eval(&attr) {
                Ok(Value::Array(_)) => {
                    let v = diagram
                        .ctx
                        .valid_eval(&attr)
                        .ok()
                        .and_then(|v| v.as_vec_f64().ok())
                        .unwrap_or(vec![3.0, 3.0]);
                    [v[0], v[1]]
                }
                Ok(v) => {
                    let s = v.as_num().unwrap_or(3.0);
                    [s, s]
                }
                Err(_) => [3.0, 3.0],
            },
            None => [3.0, 3.0],
        }
    };

    let bbox = diagram.bbox();
    let arrows: i64 = match element.borrow().get_or("arrows", "0").parse() {
        Ok(a) => a,
        Err(_) => {
            log::error!("Error in <axes> parsing arrows");
            0
        }
    };

    let mut builder = AxesBuilder {
        tactile,
        stroke,
        thickness,
        axes: axes_g,
        axes_attribute,
        horizontal_axis,
        vertical_axis,
        clear_background,
        decorations,
        h_pi_format,
        v_pi_format,
        ticksize,
        bbox,
        position_tolerance: 1e-10,
        arrows,
        h_tick_group: xml::new_element("g"),
        v_tick_group: xml::new_element("g"),
        y_axis_location: 0.0,
        y_axis_offsets: [0.0, 0.0],
        h_zero_include: false,
        top_labels: false,
        h_exclude: Vec::new(),
        h_zero_label: false,
        h_tick_direction: 1.0,
        x_axis_location: 0.0,
        x_axis_offsets: [0.0, 0.0],
        v_zero_include: false,
        right_labels: false,
        v_exclude: Vec::new(),
        v_zero_label: false,
        v_tick_direction: 1.0,
    };

    builder.position_axes(element, diagram);
    builder.apply_axis_labels(element, diagram, parent);

    if element.borrow().get_or("bounding-box", "no") == "yes" {
        let rect = xml::sub_element(&builder.axes, "rect");
        let ul = diagram.transform([builder.bbox[0], builder.bbox[3]]);
        let lr = diagram.transform([builder.bbox[2], builder.bbox[1]]);
        let mut r = rect.borrow_mut();
        r.set("x", &float2str(ul[0]));
        r.set("y", &float2str(ul[1]));
        r.set("width", &float2str(lr[0] - ul[0]));
        r.set("height", &float2str(lr[1] - ul[1]));
        r.set("fill", "none");
    }

    if builder.horizontal_axis {
        builder.add_h_axis(diagram);
        builder.horizontal_ticks(element, diagram);
        builder.h_labels(element, diagram, parent);
    }
    if builder.vertical_axis {
        builder.add_v_axis(diagram);
        builder.vertical_ticks(element, diagram);
        builder.v_labels(element, diagram, parent);
    }

    diagram.axes_info = Some(AxesInfo {
        y_axis_location: builder.y_axis_location,
        x_axis_location: builder.x_axis_location,
        top_labels: builder.top_labels,
        right_labels: builder.right_labels,
        h_tick_direction: builder.h_tick_direction,
        v_tick_direction: builder.v_tick_direction,
        ticksize: builder.ticksize,
        stroke: builder.stroke.clone(),
        thickness: builder.thickness.clone(),
    });
}

impl AxesBuilder {
    fn position_axes(&mut self, element: &El, diagram: &Diagram) {
        let scales = diagram.get_scales();

        self.y_axis_location = 0.0;
        self.y_axis_offsets = [0.0, 0.0];
        self.h_zero_include = false;
        self.top_labels = false;
        if self.bbox[1] * self.bbox[3] >= 0.0 {
            if self.bbox[3] <= 0.0 {
                self.top_labels = true;
                self.y_axis_location = self.bbox[3];
                if self.bbox[3] < 0.0 {
                    self.y_axis_offsets = [0.0, -5.0];
                }
            } else if self.bbox[1].abs() > 1e-10 {
                self.y_axis_location = self.bbox[1];
                self.y_axis_offsets = [5.0, 0.0];
            }
        }

        let h_frame = element.borrow().get("h-frame");
        if h_frame.as_deref() == Some("bottom") {
            self.y_axis_location = self.bbox[1];
            self.y_axis_offsets = [0.0, 0.0];
            self.h_zero_include = true;
        }
        if h_frame.as_deref() == Some("top") {
            self.y_axis_location = self.bbox[3];
            self.y_axis_offsets = [0.0, 0.0];
            self.h_zero_include = true;
            self.top_labels = true;
        }

        if scales[1] == AxisScale::Log {
            self.y_axis_offsets = [0.0, 0.0];
            self.h_zero_include = true;
        }

        self.h_exclude = Vec::new();
        self.h_zero_label = element.borrow().get_or("h-zero-label", "no") == "yes";
        if !self.h_zero_include
            && self.axes_attribute.as_deref() != Some("horizontal")
            && !self.h_zero_label
        {
            self.h_exclude.push(0.0);
        }

        self.h_tick_direction = if self.top_labels { -1.0 } else { 1.0 };

        self.x_axis_location = 0.0;
        self.x_axis_offsets = [0.0, 0.0];
        self.v_zero_include = false;
        self.right_labels = false;
        if self.bbox[0] * self.bbox[2] >= 0.0 {
            if self.bbox[2] <= 0.0 {
                self.right_labels = true;
                self.x_axis_location = self.bbox[2];
                if self.bbox[2] < 0.0 {
                    self.x_axis_offsets = [0.0, -10.0];
                }
            } else if self.bbox[0].abs() > 1e-10 {
                self.x_axis_location = self.bbox[0];
                self.x_axis_offsets = [10.0, 0.0];
            }
        }

        let v_frame = element.borrow().get("v-frame");
        if v_frame.as_deref() == Some("left") {
            self.x_axis_location = self.bbox[0];
            self.x_axis_offsets = [0.0, 0.0];
            self.v_zero_include = true;
        }
        if v_frame.as_deref() == Some("right") {
            self.x_axis_location = self.bbox[2];
            self.x_axis_offsets = [0.0, 0.0];
            self.v_zero_include = true;
            self.right_labels = true;
        }

        if scales[1] == AxisScale::Log {
            self.x_axis_offsets = [0.0, 0.0];
            self.v_zero_include = true;
        }

        self.v_exclude = Vec::new();
        self.v_zero_label = element.borrow().get_or("v-zero-label", "no") == "yes";
        if !self.v_zero_include
            && self.axes_attribute.as_deref() != Some("vertical")
            && !self.v_zero_label
        {
            self.v_exclude.push(0.0);
        }

        self.v_tick_direction = if self.right_labels { -1.0 } else { 1.0 };
    }

    fn apply_axis_labels(&mut self, element: &El, diagram: &mut Diagram, parent: &El) {
        let xlabel = element.borrow().get("xlabel");
        if let Some(xlabel) = xlabel {
            let el = xml::new_element("label");
            let math_element = xml::sub_element(&el, "m");
            math_element.borrow_mut().text = Some(xlabel);
            el.borrow_mut().set("clear-background", "no");
            el.borrow_mut().set(
                "p",
                &format!(
                    "({},{})",
                    py_str(self.bbox[2]),
                    py_str(self.y_axis_location)
                ),
            );
            el.borrow_mut().set("alignment", "xl");
            if self.arrows > 0 {
                if self.tactile {
                    el.borrow_mut().set("offset", "(-6,6)");
                } else {
                    el.borrow_mut().set("offset", "(-2,2)");
                }
            }
            el.borrow_mut()
                .set("clear-background", &self.clear_background);
            label::label(&el, diagram, parent, None);
        }

        let ylabel = element.borrow().get("ylabel");
        if let Some(ylabel) = ylabel {
            let el = xml::new_element("label");
            let math_element = xml::sub_element(&el, "m");
            math_element.borrow_mut().text = Some(ylabel);
            el.borrow_mut().set("clear-background", "no");
            el.borrow_mut().set(
                "p",
                &format!(
                    "({},{})",
                    py_str(self.x_axis_location),
                    py_str(self.bbox[3])
                ),
            );
            el.borrow_mut().set("alignment", "se");
            if self.arrows > 0 {
                el.borrow_mut().set("offset", "(2,-2)");
            }
            el.borrow_mut()
                .set("clear-background", &self.clear_background);
            label::label(&el, diagram, parent, None);
        }

        let children: Vec<El> = element.borrow().children.clone();
        for child in &children {
            let tag = child.borrow().tag.clone();
            if tag == "xlabel" {
                child.borrow_mut().tag = "label".to_string();
                child.borrow_mut().set("user-coords", "no");
                let anchor = diagram.transform([self.bbox[2], self.y_axis_location]);
                child.borrow_mut().set("anchor", &pt2str(anchor, ","));
                if child.borrow().get("alignment").is_none() {
                    child.borrow_mut().set("alignment", "east");
                }
                if child.borrow().get("offset").is_none() {
                    if self.arrows > 0 {
                        child.borrow_mut().set("offset", "(2,0)");
                    } else {
                        child.borrow_mut().set("offset", "(1,0)");
                    }
                }
                label::label(child, diagram, parent, None);
                continue;
            }
            if tag == "ylabel" {
                child.borrow_mut().tag = "label".to_string();
                child.borrow_mut().set("user-coords", "no");
                let anchor = diagram.transform([self.x_axis_location, self.bbox[3]]);
                child.borrow_mut().set("anchor", &pt2str(anchor, ","));
                if child.borrow().get("alignment").is_none() {
                    child.borrow_mut().set("alignment", "north");
                    if child.borrow().get("offset").is_none() {
                        if self.arrows > 0 {
                            child.borrow_mut().set("offset", "(0,2)");
                        } else {
                            child.borrow_mut().set("offset", "(0,1)");
                        }
                    }
                }
                label::label(child, diagram, parent, None);
                continue;
            }
            log::info!("{tag} element is not allowed inside a <label>");
        }
    }

    fn add_h_axis(&mut self, diagram: &mut Diagram) {
        let left_axis = diagram.transform([self.bbox[0], self.y_axis_location]);
        let right_axis = diagram.transform([self.bbox[2], self.y_axis_location]);

        let h_line = mk_line(
            left_axis,
            right_axis,
            diagram,
            None,
            Some(&EndpointOffsets::Along(self.x_axis_offsets)),
            false,
        );
        h_line.borrow_mut().set("stroke", &self.stroke);
        h_line.borrow_mut().set("stroke-width", &self.thickness);
        if self.arrows > 0 {
            arrow::add_arrowhead_to_path(diagram, "marker-end", &h_line, None, None);
        }
        if self.arrows > 1 {
            arrow::add_arrowhead_to_path(diagram, "marker-start", &h_line, None, None);
        }
        xml::append(&self.axes, &h_line);
    }

    fn add_v_axis(&mut self, diagram: &mut Diagram) {
        let bottom_axis = diagram.transform([self.x_axis_location, self.bbox[1]]);
        let top_axis = diagram.transform([self.x_axis_location, self.bbox[3]]);

        let v_line = mk_line(
            bottom_axis,
            top_axis,
            diagram,
            None,
            Some(&EndpointOffsets::Along(self.y_axis_offsets)),
            false,
        );
        v_line.borrow_mut().set("stroke", &self.stroke);
        v_line.borrow_mut().set("stroke-width", &self.thickness);
        if self.arrows > 0 {
            arrow::add_arrowhead_to_path(diagram, "marker-end", &v_line, None, None);
        }
        if self.arrows > 1 {
            arrow::add_arrowhead_to_path(diagram, "marker-start", &v_line, None, None);
        }
        xml::append(&self.axes, &v_line);
    }

    fn tick_positions(
        &self,
        diagram: &mut Diagram,
        attr_value: &str,
        scale: AxisScale,
    ) -> Option<Vec<f64>> {
        let v = diagram
            .ctx
            .valid_eval(attr_value)
            .ok()?
            .as_vec_f64()
            .ok()?;
        if scale == AxisScale::Log {
            Some(find_log_positions(&v))
        } else {
            let n = ((v[2] - v[0]) / v[1]).round_ties_even() as usize;
            Some(linspace(v[0], v[2], n))
        }
    }

    fn horizontal_ticks(&mut self, element: &El, diagram: &mut Diagram) {
        let Some(hticks) = element.borrow().get("hticks") else {
            return;
        };
        xml::append(&self.axes, &self.h_tick_group);
        diagram.add_id(&self.h_tick_group, None);

        let scale = diagram.get_scales()[0];
        let Some(x_positions) = self.tick_positions(diagram, &hticks, scale) else {
            log::error!("Error in <axes> parsing hticks={hticks}");
            return;
        };

        for x in x_positions {
            if x < self.bbox[0] || x > self.bbox[2] {
                continue;
            }
            if self.excluded(x, scale, &self.h_exclude) {
                continue;
            }
            let p = diagram.transform([x, self.y_axis_location]);
            let line_el = mk_line(
                [p[0], p[1] + self.h_tick_direction * self.ticksize[0]],
                [p[0], p[1] - self.h_tick_direction * self.ticksize[1]],
                diagram,
                None,
                None,
                false,
            );
            xml::append(&self.h_tick_group, &line_el);
        }
    }

    fn vertical_ticks(&mut self, element: &El, diagram: &mut Diagram) {
        let Some(vticks) = element.borrow().get("vticks") else {
            return;
        };
        xml::append(&self.axes, &self.v_tick_group);
        diagram.add_id(&self.v_tick_group, None);

        let scale = diagram.get_scales()[1];
        let Some(y_positions) = self.tick_positions(diagram, &vticks, scale) else {
            log::error!("Error in <axes> parsing vticks={vticks}");
            return;
        };

        for y in y_positions {
            if y < self.bbox[1] || y > self.bbox[3] {
                continue;
            }
            if self.excluded(y, scale, &self.v_exclude) {
                continue;
            }
            let p = diagram.transform([self.x_axis_location, y]);
            let line_el = mk_line(
                [p[0] - self.v_tick_direction * self.ticksize[0], p[1]],
                [p[0] + self.v_tick_direction * self.ticksize[1], p[1]],
                diagram,
                None,
                None,
                false,
            );
            xml::append(&self.v_tick_group, &line_el);
        }
    }

    fn excluded(&self, x: f64, scale: AxisScale, exclude: &[f64]) -> bool {
        exclude.iter().any(|&p| {
            let dist = if scale == AxisScale::Log {
                (x.log10() - p.log10()).abs()
            } else {
                (x - p).abs()
            };
            dist < self.position_tolerance
        })
    }

    fn h_labels(&mut self, element: &El, diagram: &mut Diagram, parent: &El) {
        let hlabels = element.borrow().get("hlabels");
        if self.decorations == "no" && hlabels.is_none() {
            return;
        }

        let mut h_exclude = self.h_exclude.clone();
        let scale = diagram.get_scales()[0];
        let h_positions = match &hlabels {
            None => {
                let positions = if scale == AxisScale::Log {
                    find_log_positions(&[self.bbox[0], self.bbox[2]])
                } else {
                    let (x0, dx, x1) =
                        find_label_positions([self.bbox[0], self.bbox[2]], self.h_pi_format);
                    let n = ((x1 - x0) / dx).round_ties_even() as usize;
                    linspace(x0, x1, n)
                };
                h_exclude.push(self.bbox[0]);
                h_exclude.push(self.bbox[2]);
                positions
            }
            Some(attr) => {
                let Some(mut positions) = self.tick_positions(diagram, attr, scale) else {
                    log::error!("Error in <axes> parsing hlabels={attr}");
                    return;
                };
                if self.h_pi_format {
                    for p in &mut positions {
                        *p /= std::f64::consts::PI;
                    }
                }
                positions
            }
        };
        let h_scale = if self.h_pi_format {
            std::f64::consts::PI
        } else {
            1.0
        };

        if xml::get_parent(&self.h_tick_group).is_none() {
            xml::append(&self.axes, &self.h_tick_group);
        }

        if self.h_zero_label {
            h_exclude.retain(|&p| p != 0.0);
        }

        let commas = element.borrow().get_or("label-commas", "yes") == "yes";

        for x in h_positions {
            if x < self.bbox[0] || x > self.bbox[2] {
                continue;
            }
            if self.excluded(x * h_scale, scale, &h_exclude) {
                continue;
            }

            let xlabel = xml::new_element("label");
            let math_element = xml::sub_element(&xlabel, "m");
            if scale == AxisScale::Log {
                let x_text = x.log10();
                let frac = x_text.rem_euclid(1.0);
                let prefix = 10f64.powf(frac).round_ties_even() as i64;
                let text = if prefix != 1 {
                    format!("{prefix}\\cdot10^{{{}}}", fmt_g(x_text.floor()))
                } else {
                    format!("10^{{{}}}", fmt_g(x_text))
                };
                math_element.borrow_mut().text = Some(text);
                xlabel.borrow_mut().set("scale", "0.8");
            } else {
                math_element.borrow_mut().text = Some(label_text(x, commas, diagram));
            }
            if self.h_pi_format {
                math_element.borrow_mut().text = Some(get_pi_text(x));
            }

            xlabel.borrow_mut().set(
                "p",
                &format!(
                    "({},{})",
                    py_str(x * h_scale),
                    py_str(self.y_axis_location)
                ),
            );
            if self.tactile {
                if self.top_labels {
                    xlabel.borrow_mut().set("alignment", "hat");
                } else {
                    xlabel.borrow_mut().set("alignment", "ha");
                }
                xlabel.borrow_mut().set("offset", "(0,0)");
            } else if self.top_labels {
                xlabel.borrow_mut().set("alignment", "north");
                xlabel.borrow_mut().set("offset", "(0,7)");
            } else {
                xlabel.borrow_mut().set("alignment", "south");
                xlabel.borrow_mut().set("offset", "(0,-7)");
            }

            xlabel
                .borrow_mut()
                .set("clear-background", &self.clear_background);
            label::label(&xlabel, diagram, parent, None);

            let p = diagram.transform([x * h_scale, self.y_axis_location]);
            let line_el = mk_line(
                [p[0], p[1] + self.h_tick_direction * self.ticksize[0]],
                [p[0], p[1] - self.h_tick_direction * self.ticksize[1]],
                diagram,
                None,
                None,
                false,
            );
            xml::append(&self.h_tick_group, &line_el);
        }
    }

    fn v_labels(&mut self, element: &El, diagram: &mut Diagram, parent: &El) {
        let vlabels = element.borrow().get("vlabels");
        if self.decorations == "no" && vlabels.is_none() {
            return;
        }

        let mut v_exclude = self.v_exclude.clone();
        let scale = diagram.get_scales()[1];
        let v_positions = match &vlabels {
            None => {
                let positions = if scale == AxisScale::Log {
                    find_log_positions(&[self.bbox[1], self.bbox[3]])
                } else {
                    let (y0, dy, y1) =
                        find_label_positions([self.bbox[1], self.bbox[3]], self.v_pi_format);
                    let n = ((y1 - y0) / dy).round_ties_even() as usize;
                    linspace(y0, y1, n)
                };
                v_exclude.push(self.bbox[1]);
                v_exclude.push(self.bbox[3]);
                positions
            }
            Some(attr) => {
                let Some(mut positions) = self.tick_positions(diagram, attr, scale) else {
                    log::error!("Error in <axes> parsing vlabels={attr}");
                    return;
                };
                if self.v_pi_format {
                    for p in &mut positions {
                        *p /= std::f64::consts::PI;
                    }
                }
                positions
            }
        };
        let v_scale = if self.v_pi_format {
            std::f64::consts::PI
        } else {
            1.0
        };

        if xml::get_parent(&self.v_tick_group).is_none() {
            xml::append(&self.axes, &self.v_tick_group);
        }

        if element.borrow().get_or("v-zero-label", "no") == "yes" {
            v_exclude.retain(|&p| p != 0.0);
        }

        let commas = element.borrow().get_or("label-commas", "yes") == "yes";

        for y in v_positions {
            if y < self.bbox[1] || y > self.bbox[3] {
                continue;
            }
            if self.excluded(y * v_scale, scale, &v_exclude) {
                continue;
            }

            let ylabel = xml::new_element("label");
            let math_element = xml::sub_element(&ylabel, "m");
            if scale == AxisScale::Log {
                let y_text = y.log10();
                let frac = y_text.rem_euclid(1.0);
                let prefix = 10f64.powf(frac).round_ties_even() as i64;
                let text = if prefix != 1 {
                    format!("{prefix}\\cdot10^{{{}}}", fmt_g(y_text.floor()))
                } else {
                    format!("10^{{{}}}", fmt_g(y_text))
                };
                math_element.borrow_mut().text = Some(text);
                ylabel.borrow_mut().set("scale", "0.8");
            } else {
                math_element.borrow_mut().text = Some(label_text(y, commas, diagram));
            }
            if self.v_pi_format {
                math_element.borrow_mut().text = Some(get_pi_text(y));
            }
            ylabel.borrow_mut().set(
                "p",
                &format!(
                    "({},{})",
                    py_str(self.x_axis_location),
                    py_str(y * v_scale)
                ),
            );

            if self.tactile {
                if self.right_labels {
                    ylabel.borrow_mut().set("alignment", "east");
                    ylabel.borrow_mut().set("offset", "(25, 0)");
                } else {
                    ylabel.borrow_mut().set("alignment", "va");
                    ylabel.borrow_mut().set("offset", "(-25, 0)");
                }
            } else if self.right_labels {
                ylabel.borrow_mut().set("alignment", "east");
                ylabel.borrow_mut().set("offset", "(7,0)");
            } else {
                ylabel.borrow_mut().set("alignment", "west");
                ylabel.borrow_mut().set("offset", "(-7,0)");
            }

            ylabel
                .borrow_mut()
                .set("clear-background", &self.clear_background);
            label::label(&ylabel, diagram, parent, None);

            let p = diagram.transform([self.x_axis_location, y * v_scale]);
            let line_el = mk_line(
                [p[0] - self.v_tick_direction * self.ticksize[0], p[1]],
                [p[0] + self.v_tick_direction * self.ticksize[1], p[1]],
                diagram,
                None,
                None,
                false,
            );
            xml::append(&self.v_tick_group, &line_el);
        }
    }
}

/// A LaTeX \text{...} for an axis label number, with comma grouping.
pub fn label_text(x: f64, commas: bool, diagram: &Diagram) -> String {
    let (prefix, x) = if x < 0.0 { ("-", -x) } else { ("", x) };
    let mut text = fmt_g(x);

    // %g may produce exponential notation; expand it
    if text.contains('e') {
        let integer = x.floor();
        let fraction = x - integer;
        let suffix = if fraction > 1e-14 {
            fmt_g(fraction)[1..].to_string()
        } else {
            String::new()
        };
        let mut int_part = String::new();
        let mut integer = integer as i64;
        while integer >= 10 {
            int_part = format!("{}{}", integer % 10, int_part);
            integer /= 10;
        }
        text = format!("{integer}{int_part}{suffix}");
    }

    if !commas {
        return format!("\\text{{{prefix}{text}}}");
    }

    let comma_include = if diagram.get_environment() == "pyodide" {
        ","
    } else {
        "{,}"
    };
    let (mut text, mut suffix) = match text.find('.') {
        Some(period) => (text[..period].to_string(), text[period..].to_string()),
        None => (text, String::new()),
    };
    while text.len() > 3 {
        let split = text.len() - 3;
        suffix = format!("{comma_include}{}{suffix}", &text[split..]);
        text.truncate(split);
    }
    format!("\\text{{{prefix}{text}{suffix}}}")
}

/// The <tick-mark> handler.
pub fn tick_mark(element: &El, diagram: &mut Diagram, parent: &El, outline_group: Option<&El>) {
    let axis = element.borrow().get_or("axis", "horizontal");
    let tactile = diagram.output_format() == "tactile";
    let location_attr = element.borrow().get_or("location", "0");
    let location_value = diagram.ctx.valid_eval(&location_attr).ok();

    let info = diagram.axes_info.clone();
    let y_axis_location = info.as_ref().map(|i| i.y_axis_location).unwrap_or(0.0);
    let x_axis_location = info.as_ref().map(|i| i.x_axis_location).unwrap_or(0.0);
    let top_labels = info.as_ref().map(|i| i.top_labels).unwrap_or(false);
    let right_labels = info.as_ref().map(|i| i.right_labels).unwrap_or(false);

    let location: [f64; 2] = match &location_value {
        Some(Value::Array(_)) => {
            let v = location_value
                .as_ref()
                .unwrap()
                .as_vec_f64()
                .unwrap_or(vec![0.0, 0.0]);
            [v[0], v[1]]
        }
        Some(v) => {
            let l = v.as_num().unwrap_or(0.0);
            if axis == "horizontal" {
                [l, y_axis_location]
            } else {
                [x_axis_location, l]
            }
        }
        None => [0.0, 0.0],
    };
    let p = diagram.transform(location);

    let size_attr = element.borrow().get("size");
    let mut size: [f64; 2] = match size_attr {
        Some(attr) => match diagram.ctx.valid_eval(&attr) {
            Ok(Value::Array(_)) => {
                let v = diagram
                    .ctx
                    .valid_eval(&attr)
                    .ok()
                    .and_then(|v| v.as_vec_f64().ok())
                    .unwrap_or(vec![3.0, 3.0]);
                [v[0], v[1]]
            }
            Ok(v) => {
                let s = v.as_num().unwrap_or(3.0);
                [s, s]
            }
            Err(_) => [3.0, 3.0],
        },
        None => [3.0, 3.0],
    };
    if tactile {
        size = [18.0, 0.0];
    }

    let line_el = if axis == "horizontal" {
        let tick_direction = info.as_ref().map(|i| i.h_tick_direction).unwrap_or(1.0);
        mk_line(
            [p[0], p[1] + tick_direction * size[0]],
            [p[0], p[1] - tick_direction * size[1]],
            diagram,
            None,
            None,
            false,
        )
    } else {
        let tick_direction = info.as_ref().map(|i| i.v_tick_direction).unwrap_or(1.0);
        mk_line(
            [p[0] - tick_direction * size[0], p[1]],
            [p[0] + tick_direction * size[1], p[1]],
            diagram,
            None,
            None,
            false,
        )
    };

    diagram.register_svg_element(element, &line_el);
    let mut thickness = element.borrow().get("thickness");
    if thickness.is_none() {
        thickness = Some(
            info.as_ref()
                .map(|i| i.thickness.clone())
                .unwrap_or_else(|| "2".to_string()),
        );
    }
    let mut stroke = element.borrow().get("stroke");
    if stroke.is_none() {
        stroke = Some(
            info.as_ref()
                .map(|i| i.stroke.clone())
                .unwrap_or_else(|| "black".to_string()),
        );
    }
    let (thickness, stroke) = if tactile {
        ("2".to_string(), "black".to_string())
    } else {
        (thickness.unwrap(), stroke.unwrap())
    };
    line_el.borrow_mut().set("stroke-width", &thickness);
    line_el.borrow_mut().set("stroke", &stroke);

    let has_label = label::has_label(element);
    let parent = if has_label {
        let g = xml::sub_element(parent, "g");
        let id = element.borrow().get("id");
        diagram.add_id(&g, id.as_deref());
        g
    } else {
        let id = element.borrow().get("id");
        diagram.add_id(&line_el, id.as_deref());
        parent.clone()
    };
    xml::append(&parent, &line_el);

    if has_label {
        let el_copy = xml::deep_copy(element);
        let (align, off) = if axis == "horizontal" {
            if tactile {
                if top_labels {
                    ("hat", "(0,0)")
                } else {
                    ("ha", "(0,0)")
                }
            } else if top_labels {
                ("north", "(0,7)")
            } else {
                ("south", "(0,-7)")
            }
        } else if tactile {
            if right_labels {
                ("east", "(25,0)")
            } else {
                ("va", "(-25,0)")
            }
        } else if right_labels {
            ("east", "(7,0)")
        } else {
            ("west", "(-7,0)")
        };

        if el_copy.borrow().get("alignment").is_none() {
            el_copy.borrow_mut().set("alignment", align);
            if el_copy.borrow().get("offset").is_none() {
                el_copy.borrow_mut().set("offset", off);
            }
        }
        el_copy.borrow_mut().set("user-coords", "no");
        el_copy.borrow_mut().set("anchor", &pt2str(p, ","));
        label::label(&el_copy, diagram, &parent, outline_group);
    }
}
