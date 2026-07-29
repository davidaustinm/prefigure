//! Port of prefig/core/read.py: load CSV data into the namespace as a table
//! (a dict of column-name → array), where numeric cells become numbers unless
//! the column is listed in `string-columns`.

use crate::core::diagram::Diagram;
use crate::value::Value;
use crate::xml::El;
use indexmap::IndexMap;
use std::collections::HashSet;

pub fn read(element: &El, diagram: &mut Diagram, _parent: &El, _outline_group: Option<&El>) {
    let Some(filename) = element.borrow().get("filename") else {
        log::error!("A <read> element needs a @filename attribute");
        return;
    };
    // resolve the path exactly like Python: data/ prefix under pretext,
    // otherwise relative to the external root if one is set
    let path = if diagram.get_environment() == "pretext" {
        format!("data/{filename}")
    } else if let Some(root) = diagram.get_external() {
        let root = root.trim_end_matches('/');
        format!("{root}/{filename}")
    } else {
        filename
    };

    let Some(name) = element.borrow().get("name") else {
        log::error!("A <read> element needs a @name attribute");
        return;
    };
    let filetype = element.borrow().get_or("type", "csv");
    if filetype == "csv" {
        load_csv(element, diagram, &path, &name);
    }
}

fn load_csv(element: &El, diagram: &mut Diagram, path: &str, name: &str) {
    let delimiter = element
        .borrow()
        .get_or("delimiter", ",")
        .chars()
        .next()
        .unwrap_or(',');
    let quotechar = element
        .borrow()
        .get_or("quotechar", "'")
        .chars()
        .next()
        .unwrap_or('\'');
    let str_cols_attr = element.borrow().get_or("string-columns", "[]");
    let str_cols: HashSet<String> = diagram
        .ctx
        .valid_eval(&str_cols_attr)
        .ok()
        .and_then(|v| match v {
            Value::Array(items) => Some(items.iter().map(|i| i.to_py_str()).collect()),
            _ => None,
        })
        .unwrap_or_default();

    let Ok(content) = std::fs::read_to_string(path) else {
        log::error!("Unable to read the data file {path}");
        return;
    };

    let mut rows = parse_csv(&content, delimiter, quotechar);
    if rows.is_empty() {
        return;
    }
    let headers = rows.remove(0);

    // one column per header, preserving header order (like the dict)
    let mut columns: IndexMap<String, Vec<Value>> = IndexMap::new();
    for header in &headers {
        columns.entry(header.clone()).or_default();
    }
    for row in &rows {
        for (i, header) in headers.iter().enumerate() {
            let cell = row.get(i).cloned().unwrap_or_default();
            let value = if str_cols.contains(header) {
                Value::Str(cell)
            } else {
                // Python: float(cell), falling back to the string
                match cell.parse::<f64>() {
                    Ok(n) => Value::Num(n),
                    Err(_) => Value::Str(cell),
                }
            };
            columns.get_mut(header).expect("header column").push(value);
        }
    }

    let table: IndexMap<String, Value> = columns
        .into_iter()
        .map(|(k, v)| (k, Value::Array(v)))
        .collect();
    diagram.ctx.enter_namespace(name, Value::Dict(table));
}

/// Minimal CSV reader matching Python's csv.reader for the single-char
/// delimiter / quotechar used here: fields may be wrapped in the quote char,
/// inside which a doubled quote is a literal quote and the delimiter is data.
fn parse_csv(content: &str, delimiter: char, quotechar: char) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut field = String::new();
    let mut row: Vec<String> = Vec::new();
    let mut in_quotes = false;
    let mut chars = content.chars().peekable();

    while let Some(c) = chars.next() {
        if in_quotes {
            if c == quotechar {
                if chars.peek() == Some(&quotechar) {
                    field.push(quotechar);
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                field.push(c);
            }
        } else if c == quotechar {
            in_quotes = true;
        } else if c == delimiter {
            row.push(std::mem::take(&mut field));
        } else if c == '\n' {
            row.push(std::mem::take(&mut field));
            rows.push(std::mem::take(&mut row));
        } else if c != '\r' {
            field.push(c);
        }
    }
    // a trailing line with no newline
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    // drop a trailing empty row from a final newline
    if rows.last().is_some_and(|r| r.len() == 1 && r[0].is_empty()) {
        rows.pop();
    }
    rows
}
