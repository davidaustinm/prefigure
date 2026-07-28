//! Port of prefig/core/repeat.py: repeat a block of XML over a parameter.

use crate::core::diagram::Diagram;
use crate::core::{group, label};
use crate::value::{py_str, Value};
use crate::xml::{self, El};

/// EPUB restricts id characters to [A-Za-z0-9_-]; substitute the rest.
pub fn epub_clean(s: &str) -> String {
    s.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch
            } else {
                match ch {
                    '(' | '[' | '{' => 'p',
                    ')' | ']' | '}' => 'q',
                    ',' => 'c',
                    '.' => 'd',
                    '=' => '_',
                    '#' => 'h',
                    _ => '_',
                }
            }
        })
        .collect()
}

pub fn repeat(element: &El, diagram: &mut Diagram, parent: &El, outline_group: Option<&El>) {
    let Some(parameter) = element.borrow().get("parameter") else {
        log::error!("Unable to parse parameter in <repeat>");
        return;
    };

    // "k=0..7" counts; "k in expr" iterates over an evaluated sequence
    let fields: Vec<&str> = parameter.split('=').collect();
    let (var, iterator, count): (String, Vec<Value>, bool) = if fields.len() == 2 {
        let var = fields[0].trim().to_string();
        let bounds: Vec<&str> = fields[1].split("..").collect();
        if bounds.len() != 2 {
            log::error!("Unable to parse parameter {parameter} in <repeat>");
            return;
        }
        let eval_int = |diagram: &mut Diagram, s: &str| -> Option<i64> {
            diagram
                .ctx
                .valid_eval(s)
                .ok()
                .and_then(|v| v.as_num().ok())
                .map(|n| n as i64)
        };
        let (Some(start), Some(stop)) = (
            eval_int(diagram, bounds[0]),
            eval_int(diagram, bounds[1]),
        ) else {
            log::error!("Unable to parse parameter {parameter} in <repeat>");
            return;
        };
        let iterator = (start..=stop).map(|k| Value::Num(k as f64)).collect();
        (var, iterator, true)
    } else {
        let fields: Vec<&str> = parameter.split_whitespace().collect();
        if fields.len() < 3 {
            log::error!("Unable to parse parameter {parameter} in <repeat>");
            return;
        }
        let var = fields[0].to_string();
        let expr = fields[2..].join(" ");
        let Ok(Value::Array(items)) = diagram.ctx.valid_eval(&expr) else {
            log::error!("Unable to parse parameter {parameter} in <repeat>");
            return;
        };
        (var, items, false)
    };

    // convert the element to a group holding one definition per value
    let element_cp = xml::deep_copy(element);
    let outline = element.borrow().get("outline");
    let id = element.borrow().get("id");
    {
        let mut el = element.borrow_mut();
        el.attrs.clear();
        el.text = None;
        el.children.clear();
        el.tag = "group".to_string();
    }
    if let Some(outline) = &outline {
        element.borrow_mut().set("outline", outline);
    }
    if let Some(id) = &id {
        let id = diagram.prepend_id_prefix(id);
        element.borrow_mut().set("id", &id);
        element_cp.borrow_mut().set("id", &id);
    }

    for (num, k) in iterator.iter().enumerate() {
        let k_str = match k {
            Value::Array(_) => {
                // Python: "(" + pt2long_str(k, spacer=",") + ",)" over ALL
                // components, whatever the length
                let v = k.as_vec_f64().unwrap_or_default();
                let joined = v
                    .iter()
                    .map(|c| format!("{c:.4}"))
                    .collect::<Vec<_>>()
                    .join(",");
                format!("({joined},)")
            }
            Value::Num(n) => py_str(*n),
            other => other.to_py_str(),
        };

        let k_str_clean = epub_clean(&k_str);
        let suffix_str = if count {
            format!("{var}_{k_str_clean}")
        } else {
            format!("{var}_{num}")
        };

        let definition = xml::sub_element(element, "definition");
        definition.borrow_mut().text = Some(format!("{var}={k_str}"));
        definition.borrow_mut().set("id-suffix", &suffix_str);

        let children: Vec<El> = element_cp.borrow().children.clone();
        for child in &children {
            xml::append(&definition, &xml::deep_copy(child));
        }
    }

    let mut annotation = None;
    if element_cp.borrow().get_or("annotate", "no") == "yes"
        && parent.borrow().get_or("data-outline", "no") == "no"
    {
        let a = xml::new_element("annotation");
        for attrib in ["id", "text", "circular", "sonify", "speech"] {
            if let Some(value) = element_cp.borrow().get(attrib) {
                a.borrow_mut().set(attrib, &value);
            }
        }
        for attrib in ["text", "speech"] {
            let value = a.borrow().get(attrib);
            if let Some(value) = value {
                let evaluated = label::evaluate_text(&value, &mut diagram.ctx);
                a.borrow_mut().set(attrib, &evaluated);
            }
        }
        diagram.push_to_annotation_branch(a.clone());
        annotation = Some(a);
    }

    group::group(element, diagram, parent, outline_group);

    if annotation.is_some() {
        diagram.pop_from_annotation_branch();
    }
}
