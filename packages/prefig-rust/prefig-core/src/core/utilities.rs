//! Port of prefig/core/utilities.py: attribute helpers and float formatting.

use crate::core::diagram::Diagram;
use crate::evaluator::ExpressionContext;
use crate::value::{py_str, Value};
use crate::xml::El;

pub fn get_color(color: Option<&str>) -> String {
    match color {
        None => "none".to_string(),
        Some("gray") => "#777".to_string(),
        Some("lightgray") => "#ccc".to_string(),
        Some("darkgray") => "#333".to_string(),
        Some(other) => other.to_string(),
    }
}

pub const TEXTURES: [&str; 6] = [
    "horizontal",
    "vertical",
    "diagonal",
    "backdiagonal",
    "dot",
    "diamond",
];

pub fn add_attr(element: &El, attrs: Vec<(String, String)>) {
    let mut el = element.borrow_mut();
    for (k, v) in attrs {
        el.set(&k, &v);
    }
}

/// util.get_attr: evaluate the attribute, falling back to the raw string when
/// it isn't an expression (e.g. color names). Vectors join with %.4f.
pub fn get_attr(element: &El, attr: &str, default: &str, ctx: &mut ExpressionContext) -> String {
    let raw = element.borrow().get_or(attr, default);
    match ctx.valid_eval(&raw) {
        Ok(Value::Array(items)) => {
            let parts: Result<Vec<String>, _> = items
                .iter()
                .map(|v| v.as_num().map(float2longstr))
                .collect();
            match parts {
                Ok(parts) => parts.join(","),
                Err(_) => raw,
            }
        }
        Ok(value) => value.to_py_str(),
        Err(_) => raw,
    }
}

/// util.set_attr: evaluate (with ${...} substitution) and write back.
pub fn set_attr(element: &El, attr: &str, default: &str, ctx: &mut ExpressionContext) {
    let value = get_attr(element, attr, default, ctx);
    let value = crate::core::label::evaluate_text(&value, ctx);
    element.borrow_mut().set(attr, &value);
}

/// util.get_1d_attr: stroke/opacity/thickness/dash attributes for paths.
pub fn get_1d_attr(element: &El, ctx: &mut ExpressionContext) -> Vec<(String, String)> {
    let el = element.borrow();
    let mut d = Vec::new();
    if let Some(stroke) = el.get("stroke") {
        d.push(("stroke".to_string(), get_color(Some(&stroke))));
    }
    for (attr, svg_attr) in [
        ("stroke-opacity", "stroke-opacity"),
        ("opacity", "opacity"),
        ("thickness", "stroke-width"),
    ] {
        if let Some(value) = el.get(attr) {
            let evaluated = ctx
                .valid_eval(&value)
                .map(|v| v.to_py_str())
                .unwrap_or(value);
            d.push((svg_attr.to_string(), evaluated));
        }
    }
    for (attr, svg_attr) in [
        ("miterlimit", "stroke-miterlimit"),
        ("linejoin", "stroke-linejoin"),
        ("linecap", "stroke-linecap"),
        ("dash", "stroke-dasharray"),
    ] {
        if let Some(value) = el.get(attr) {
            d.push((svg_attr.to_string(), value));
        }
    }
    d.push(("fill".to_string(), el.get_or("fill", "none")));
    d
}

pub fn set_tactile_fill(element: &El) {
    let fill = element.borrow().get_or("fill", "none");
    if fill.starts_with("url") {
        return;
    }
    if fill == "white" || fill == "none" {
        element.borrow_mut().set("fill", &fill);
    } else {
        element.borrow_mut().set("fill", "lightgray");
    }
}

/// util.get_2d_attr: 1d attributes plus fill handling (colors and textures).
pub fn get_2d_attr(element: &El, diagram: &mut Diagram) -> Vec<(String, String)> {
    let mut d = get_1d_attr(element, &mut diagram.ctx);
    let fill_color = get_color(element.borrow().get("fill").as_deref());
    let texture = element.borrow().get("fill-pattern");
    if let Some(texture) = texture {
        if TEXTURES.contains(&texture.as_str()) {
            let url = diagram.add_texture(&texture, &fill_color);
            let fill = format!("url(#{url})");
            set_pair(&mut d, "fill", &fill);
            element.borrow_mut().set("fill", &fill);
        } else {
            log::error!("{texture} is not a recognized texture");
        }
    } else {
        set_pair(&mut d, "fill", &fill_color);
        element.borrow_mut().set("fill", &fill_color);
    }
    if let Some(rule) = element.borrow().get("fill-rule") {
        d.push(("fill-rule".to_string(), rule));
    }
    let fill_opacity = element.borrow().get("fill-opacity");
    if let Some(value) = fill_opacity {
        let evaluated = diagram
            .ctx
            .valid_eval(&value)
            .map(|v| v.to_py_str())
            .unwrap_or(value);
        d.push(("fill-opacity".to_string(), evaluated));
    }
    d
}

fn set_pair(d: &mut Vec<(String, String)>, key: &str, value: &str) {
    match d.iter_mut().find(|(k, _)| k == key) {
        Some(pair) => pair.1 = value.to_string(),
        None => d.push((key.to_string(), value.to_string())),
    }
}

pub fn cliptobbox(g_element: &El, element: &El, diagram: &Diagram) {
    if element.borrow().get_or("cliptobbox", "no") == "no" {
        return;
    }
    let id = diagram.get_clippath();
    g_element
        .borrow_mut()
        .set("clip-path", &format!("url(#{id})"));
}

pub fn float2str(x: f64) -> String {
    format!("{x:.1}")
}

pub fn float2longstr(x: f64) -> String {
    format!("{x:.4}")
}

pub fn pt2str(p: [f64; 2], spacer: &str) -> String {
    format!("{:.1}{}{:.1}", p[0], spacer, p[1])
}

pub fn pt2str_paren(p: [f64; 2], spacer: &str) -> String {
    format!("({:.1}{}{:.1})", p[0], spacer, p[1])
}

pub fn pt2long_str(p: [f64; 2], spacer: &str) -> String {
    format!("{:.4}{}{:.4}", p[0], spacer, p[1])
}

/// util.np2str: "(x,y)" with one decimal place.
pub fn np2str(p: [f64; 2]) -> String {
    pt2str_paren(p, ",")
}

/// str() of a Python number that may be an int (helper for f-strings like
/// '({},{})'.format(x, y) in the Python source).
pub fn num_str(x: f64) -> String {
    py_str(x)
}
