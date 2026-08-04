//! Port of prefig/core/statistics.py: scatter plots and histograms. Both
//! rewrite themselves into a <repeat> of simpler elements and re-dispatch.

use crate::core::diagram::Diagram;
use crate::core::math_utilities::linspace;
use crate::core::tags;
use crate::value::Value;
use crate::xml::{self, El};

pub fn scatter(element: &El, diagram: &mut Diagram, parent: &El, outline_group: Option<&El>) {
    let points: Value;
    let data_name = element.borrow().get("data");
    if let Some(data_name) = data_name {
        let Some(Value::Dict(data)) = diagram.ctx.retrieve(&data_name).cloned() else {
            log::error!("A <scatter> @data must name a data table");
            return;
        };
        let Some(x_field) = element.borrow().get("x") else {
            log::error!("A <scatter> defined from a data source needs an @x attribute");
            return;
        };
        let Some(y_field) = element.borrow().get("y") else {
            log::error!("A <scatter> defined from a data source needs a @y attribute");
            return;
        };

        let filter_attr = element.borrow().get("filter");
        let (x_data, y_data) = if let Some(attr) = filter_attr {
            let Some((field, value)) = diagram.ctx.valid_eval(&attr).ok().and_then(|v| {
                let Value::Array(items) = v else { return None };
                (items.len() == 2).then(|| (items[0].to_py_str(), items[1].clone()))
            }) else {
                log::error!("Error in <scatter> parsing filter={attr}");
                return;
            };
            (
                filter_column(&data, &x_field, &field, &value),
                filter_column(&data, &y_field, &field, &value),
            )
        } else {
            (
                data.get(&x_field).cloned().unwrap_or(Value::Array(vec![])),
                data.get(&y_field).cloned().unwrap_or(Value::Array(vec![])),
            )
        };
        points = zip_columns(&x_data, &y_data);
    } else {
        let Some(pts) = element.borrow().get("points") else {
            log::error!("A <scatter> needs a @data or @points attribute");
            return;
        };
        let Ok(value) = diagram.ctx.valid_eval(&pts) else {
            log::error!("Error in <scatter> parsing points={pts}");
            return;
        };
        points = value;
    }
    diagram.ctx.enter_namespace("__scatter_points", points);

    // build the <point> template and turn ourselves into a <repeat>
    let point_element = xml::deep_copy(element);
    {
        let mut p = point_element.borrow_mut();
        p.tag = "point".to_string();
        p.set("p", "point");
    }
    let handle = element.borrow().get("at");
    if let Some(handle) = &handle {
        point_element
            .borrow_mut()
            .set("at", &format!("{handle}-point"));
    }
    let point_text = element.borrow().get("point-text");
    if let Some(point_text) = point_text {
        point_element.borrow_mut().set("annotate", "yes");
        point_element.borrow_mut().set("text", &point_text);
    }

    {
        let mut el = element.borrow_mut();
        el.tag = "repeat".to_string();
        el.set("parameter", "point in __scatter_points");
    }
    xml::append(element, &point_element);

    let _ = tags::parse_element(element, diagram, parent, outline_group);
}

pub fn histogram(element: &El, diagram: &mut Diagram, parent: &El, outline_group: Option<&El>) {
    let Some(data_attr) = element.borrow().get("data") else {
        log::error!("A <histogram> needs a @data attribute");
        return;
    };
    let Some(data) = diagram
        .ctx
        .valid_eval(&data_attr)
        .ok()
        .and_then(|v| v.as_vec_f64().ok())
    else {
        log::error!("Error in <histogram> parsing data={data_attr}");
        return;
    };

    let minimum = diagram
        .ctx
        .valid_eval(&element.borrow().get_or("min", "0"))
        .ok()
        .and_then(|v| v.as_num().ok())
        .unwrap_or(0.0);
    let maximum = match element.borrow().get("max") {
        Some(attr) => diagram
            .ctx
            .valid_eval(&attr)
            .ok()
            .and_then(|v| v.as_num().ok())
            .unwrap_or(0.0),
        None => data.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
    };
    let bins = diagram
        .ctx
        .valid_eval(&element.borrow().get_or("bins", "20"))
        .ok()
        .and_then(|v| v.as_num().ok())
        .unwrap_or(20.0) as usize;

    let hist = ndimage_histogram(&data, minimum, maximum, bins);
    let x_values = linspace(minimum, maximum, bins);
    let delta_x = (maximum - minimum) / bins as f64;

    diagram.ctx.enter_namespace(
        "__histogram_x",
        Value::Array(x_values.iter().map(|&x| Value::Num(x)).collect()),
    );
    diagram.ctx.enter_namespace(
        "__histogram_y",
        Value::Array(hist.iter().map(|&y| Value::Num(y)).collect()),
    );
    diagram
        .ctx
        .enter_namespace("__delta_x", Value::Num(delta_x));

    let bin_element = xml::deep_copy(element);
    {
        let mut b = bin_element.borrow_mut();
        b.tag = "rectangle".to_string();
        b.set("lower-left", "(__histogram_x[bin_num],0)");
        b.set("dimensions", "(__delta_x,__histogram_y[bin_num])");
    }
    let handle = element.borrow().get("at");
    if let Some(handle) = &handle {
        bin_element.borrow_mut().set("at", &format!("{handle}-bin"));
    }
    let bin_text = element.borrow().get("bin-text");
    if let Some(bin_text) = bin_text {
        bin_element.borrow_mut().set("annotate", "yes");
        bin_element.borrow_mut().set("text", &bin_text);
    }

    {
        let mut el = element.borrow_mut();
        el.tag = "repeat".to_string();
        el.set("parameter", &format!("bin_num=0..{}", bins - 1));
    }
    xml::append(element, &bin_element);

    let _ = tags::parse_element(element, diagram, parent, outline_group);
}

fn filter_column(
    data: &indexmap::IndexMap<String, Value>,
    column: &str,
    mask_field: &str,
    value: &Value,
) -> Value {
    let (Some(Value::Array(col)), Some(Value::Array(mask))) =
        (data.get(column), data.get(mask_field))
    else {
        return Value::Array(vec![]);
    };
    let selected: Vec<Value> = col
        .iter()
        .zip(mask)
        .filter(|(_, m)| eq(m, value))
        .map(|(d, _)| d.clone())
        .collect();
    Value::Array(selected)
}

fn eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Num(x), Value::Num(y)) => x == y,
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        _ => false,
    }
}

fn zip_columns(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Array(xs), Value::Array(ys)) => Value::Array(
            xs.iter()
                .zip(ys)
                .map(|(x, y)| Value::Array(vec![x.clone(), y.clone()]))
                .collect(),
        ),
        _ => Value::Array(vec![]),
    }
}

/// scipy.ndimage.histogram: count values in `bins` equal-width bins over
/// [min, max]; the last bin is closed on the right (numpy.histogram semantics).
fn ndimage_histogram(data: &[f64], minimum: f64, maximum: f64, bins: usize) -> Vec<f64> {
    let mut counts = vec![0.0; bins];
    if bins == 0 || maximum <= minimum {
        return counts;
    }
    let width = (maximum - minimum) / bins as f64;
    for &v in data {
        if v < minimum || v > maximum {
            continue;
        }
        let mut bin = ((v - minimum) / width).floor() as usize;
        if bin >= bins {
            bin = bins - 1;
        }
        counts[bin] += 1.0;
    }
    counts
}
