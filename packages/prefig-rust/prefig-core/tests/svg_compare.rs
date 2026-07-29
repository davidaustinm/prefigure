//! Structural SVG comparison with numeric tolerance — the Rust analogue of
//! tmp-prefig-with-cpp/correctness_comparison.py.
//!
//! Two documents match when their element trees have the same shape (tags,
//! attribute names, text) and every numeric token embedded in attribute or
//! text values agrees within the given tolerance. Numeric tokens are compared
//! after splitting values on whitespace, commas, and path-command letters, so
//! `d="M 5.0 305.0"` and `points="1,2 3,4"` compare numerically.

use quick_xml::events::Event;
use quick_xml::Reader;

#[derive(Debug, Clone, PartialEq)]
struct Element {
    tag: String,
    attrs: Vec<(String, String)>, // sorted by name
    text: String,
    children: Vec<Element>,
}

fn parse(svg: &str) -> Result<Element, String> {
    let mut reader = Reader::from_str(svg);
    reader.config_mut().trim_text(true);
    let mut stack: Vec<Element> = vec![];
    let mut root: Option<Element> = None;

    let new_element = |e: &quick_xml::events::BytesStart| -> Result<Element, String> {
        let tag = String::from_utf8_lossy(e.name().as_ref()).into_owned();
        let mut attrs = Vec::new();
        for attr in e.attributes() {
            let attr = attr.map_err(|e| e.to_string())?;
            attrs.push((
                String::from_utf8_lossy(attr.key.as_ref()).into_owned(),
                String::from_utf8_lossy(&attr.value).into_owned(),
            ));
        }
        attrs.sort();
        Ok(Element {
            tag,
            attrs,
            text: String::new(),
            children: vec![],
        })
    };

    loop {
        match reader.read_event().map_err(|e| e.to_string())? {
            Event::Start(e) => stack.push(new_element(&e)?),
            Event::Empty(e) => {
                let el = new_element(&e)?;
                match stack.last_mut() {
                    Some(parent) => parent.children.push(el),
                    None => root = Some(el),
                }
            }
            Event::End(_) => {
                let el = stack.pop().ok_or("unbalanced end tag")?;
                match stack.last_mut() {
                    Some(parent) => parent.children.push(el),
                    None => root = Some(el),
                }
            }
            Event::Text(t) => {
                if let Some(el) = stack.last_mut() {
                    el.text.push_str(&t.unescape().map_err(|e| e.to_string())?);
                }
            }
            Event::Eof => break,
            _ => {} // comments, PIs, doctype
        }
    }
    root.ok_or_else(|| "no root element".to_string())
}

/// Split a value into tokens, isolating numeric runs: path-command letters and
/// punctuation become separators, numbers become parseable tokens.
fn tokenize(value: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for c in value.chars() {
        if c.is_ascii_digit() || c == '.' || c == '-' || c == '+' {
            current.push(c);
        } else {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            if !c.is_whitespace() && c != ',' {
                tokens.push(c.to_string());
            }
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn values_match(a: &str, b: &str, tol: f64) -> bool {
    if a == b {
        return true;
    }
    let (ta, tb) = (tokenize(a), tokenize(b));
    if ta.len() != tb.len() {
        return false;
    }
    ta.iter().zip(&tb).all(|(x, y)| {
        if x == y {
            return true;
        }
        match (x.parse::<f64>(), y.parse::<f64>()) {
            (Ok(nx), Ok(ny)) => (nx - ny).abs() <= tol + tol * ny.abs(),
            _ => false,
        }
    })
}

fn compare_elements(a: &Element, b: &Element, tol: f64, path: &str, diffs: &mut Vec<String>) {
    // cap the report; one layout bug shifts everything downstream
    if diffs.len() > 20 {
        return;
    }
    let here = format!("{path}/{}", a.tag);
    if a.tag != b.tag {
        diffs.push(format!("{path}: tag {} != {}", a.tag, b.tag));
        return;
    }
    let a_names: Vec<&String> = a.attrs.iter().map(|(k, _)| k).collect();
    let b_names: Vec<&String> = b.attrs.iter().map(|(k, _)| k).collect();
    if a_names != b_names {
        diffs.push(format!(
            "{here}: attribute names differ: {a_names:?} != {b_names:?}"
        ));
        return;
    }
    for ((name, va), (_, vb)) in a.attrs.iter().zip(&b.attrs) {
        if !values_match(va, vb, tol) {
            diffs.push(format!("{here}@{name}: {va:?} != {vb:?}"));
        }
    }
    if !values_match(&a.text, &b.text, tol) {
        diffs.push(format!("{here}: text {:?} != {:?}", a.text, b.text));
    }
    if a.children.len() != b.children.len() {
        diffs.push(format!(
            "{here}: child count {} != {}",
            a.children.len(),
            b.children.len()
        ));
        return;
    }
    for (ca, cb) in a.children.iter().zip(&b.children) {
        compare_elements(ca, cb, tol, &here, diffs);
    }
}

/// Compare two SVG documents; returns a list of differences (empty == match).
pub fn compare(a: &str, b: &str, tol: f64) -> Vec<String> {
    let (a, b) = match (parse(a), parse(b)) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(e), _) | (_, Err(e)) => return vec![format!("XML parse error: {e}")],
    };
    let mut diffs = Vec::new();
    compare_elements(&a, &b, tol, "", &mut diffs);
    diffs
}
