//! XML element tree with lxml-like semantics, so ports of the Python handlers
//! read line-for-line: elements have `.text` (text before the first child),
//! `.tail` (text after the element's closing tag), element-only children, and
//! a parent pointer. Attribute insertion order is preserved.

use indexmap::IndexMap;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

pub type El = Rc<RefCell<Element>>;

#[derive(Default)]
pub struct Element {
    pub tag: String,
    pub attrs: IndexMap<String, String>,
    pub text: Option<String>,
    pub tail: Option<String>,
    pub children: Vec<El>,
    pub parent: Weak<RefCell<Element>>,
}

pub fn new_element(tag: &str) -> El {
    Rc::new(RefCell::new(Element {
        tag: tag.to_string(),
        ..Default::default()
    }))
}

/// ET.SubElement: create a child and append it.
pub fn sub_element(parent: &El, tag: &str) -> El {
    let child = new_element(tag);
    append(parent, &child);
    child
}

/// element.append(child) — moves the child under this parent.
pub fn append(parent: &El, child: &El) {
    if let Some(old_parent) = child.borrow().parent.upgrade() {
        old_parent
            .borrow_mut()
            .children
            .retain(|c| !Rc::ptr_eq(c, child));
    }
    child.borrow_mut().parent = Rc::downgrade(parent);
    parent.borrow().children.iter().for_each(|_| {});
    parent.borrow_mut().children.push(child.clone());
}

/// element.insert(index, child)
pub fn insert(parent: &El, index: usize, child: &El) {
    if let Some(old_parent) = child.borrow().parent.upgrade() {
        old_parent
            .borrow_mut()
            .children
            .retain(|c| !Rc::ptr_eq(c, child));
    }
    child.borrow_mut().parent = Rc::downgrade(parent);
    parent.borrow_mut().children.insert(index, child.clone());
}

/// parent.remove(child)
pub fn remove(parent: &El, child: &El) {
    parent
        .borrow_mut()
        .children
        .retain(|c| !Rc::ptr_eq(c, child));
    child.borrow_mut().parent = Weak::new();
}

pub fn get_parent(el: &El) -> Option<El> {
    el.borrow().parent.upgrade()
}

/// copy.deepcopy(element): a detached deep copy (tail excluded, as when
/// lxml deep-copies a subtree root the tail comes along — we keep it to
/// mirror deepcopy exactly).
pub fn deep_copy(el: &El) -> El {
    let src = el.borrow();
    let copy = Rc::new(RefCell::new(Element {
        tag: src.tag.clone(),
        attrs: src.attrs.clone(),
        text: src.text.clone(),
        tail: src.tail.clone(),
        children: Vec::new(),
        parent: Weak::new(),
    }));
    for child in &src.children {
        let child_copy = deep_copy(child);
        child_copy.borrow_mut().parent = Rc::downgrade(&copy);
        copy.borrow_mut().children.push(child_copy);
    }
    copy
}

/// All descendants in document order, including the element itself
/// (element.getiterator()).
pub fn iter_subtree(el: &El) -> Vec<El> {
    let mut out = vec![el.clone()];
    let children: Vec<El> = el.borrow().children.clone();
    for child in &children {
        out.extend(iter_subtree(child));
    }
    out
}

/// element.findall(tag): direct children with the given tag.
pub fn find_all(el: &El, tag: &str) -> Vec<El> {
    el.borrow()
        .children
        .iter()
        .filter(|c| c.borrow().tag == tag)
        .cloned()
        .collect()
}

/// element.find(tag): first direct child with the given tag.
pub fn find(el: &El, tag: &str) -> Option<El> {
    el.borrow()
        .children
        .iter()
        .find(|c| c.borrow().tag == tag)
        .cloned()
}

/// element.xpath('.//tag'): descendants (not self) with the given tag.
pub fn find_descendants(el: &El, tag: &str) -> Vec<El> {
    let mut out = Vec::new();
    let children: Vec<El> = el.borrow().children.clone();
    for child in &children {
        if child.borrow().tag == tag {
            out.push(child.clone());
        }
        out.extend(find_descendants(child, tag));
    }
    out
}

impl Element {
    pub fn get(&self, attr: &str) -> Option<String> {
        self.attrs.get(attr).cloned()
    }

    pub fn get_or(&self, attr: &str, default: &str) -> String {
        self.attrs
            .get(attr)
            .cloned()
            .unwrap_or_else(|| default.to_string())
    }

    pub fn set(&mut self, attr: &str, value: &str) {
        self.attrs.insert(attr.to_string(), value.to_string());
    }

    /// element.attrib.pop(attr, default)
    pub fn pop_attr(&mut self, attr: &str) -> Option<String> {
        self.attrs.shift_remove(attr)
    }
}

// ---------- serialization ----------

/// Escape XML text like lxml's `ET.tostring` (default ASCII encoding): the
/// markup characters plus every non-ASCII codepoint as a numeric entity, so the
/// output matches the Python reference byte-for-byte.
fn escape_into(s: &str, out: &mut String, quote: bool) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if quote => out.push_str("&quot;"),
            c if (c as u32) < 128 => out.push(c),
            c => {
                out.push_str("&#");
                out.push_str(&(c as u32).to_string());
                out.push(';');
            }
        }
    }
}

fn escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    escape_into(s, &mut out, false);
    out
}

fn escape_attr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    escape_into(s, &mut out, true);
    out
}

fn write_element(el: &El, out: &mut String, indent: Option<usize>) {
    let e = el.borrow();
    if let Some(level) = indent {
        if level > 0 {
            out.push('\n');
            out.push_str(&"  ".repeat(level));
        }
    }
    out.push('<');
    out.push_str(&e.tag);
    for (k, v) in &e.attrs {
        out.push(' ');
        out.push_str(k);
        out.push_str("=\"");
        out.push_str(&escape_attr(v));
        out.push('"');
    }
    let has_text = e.text.as_ref().is_some_and(|t| !t.is_empty());
    if e.children.is_empty() && !has_text {
        out.push_str("/>");
    } else {
        out.push('>');
        if let Some(text) = &e.text {
            out.push_str(&escape_text(text));
        }
        for child in &e.children {
            write_element(child, out, indent.map(|l| l + 1));
            let child_ref = child.borrow();
            if let Some(tail) = &child_ref.tail {
                out.push_str(&escape_text(tail));
            }
        }
        if indent.is_some() && !e.children.is_empty() {
            let level = indent.unwrap();
            out.push('\n');
            out.push_str(&"  ".repeat(level));
        }
        out.push_str("</");
        out.push_str(&e.tag);
        out.push('>');
    }
}

/// ET.tostring(el)
pub fn to_string(el: &El) -> String {
    let mut out = String::new();
    write_element(el, &mut out, None);
    out
}

/// lxml write with pretty_print=True (2-space indent). Only used when writing
/// files for humans; comparisons ignore whitespace.
pub fn to_pretty_string(el: &El) -> String {
    let mut out = String::new();
    write_element(el, &mut out, Some(0));
    out.push('\n');
    out
}

// ---------- parsing (feature-gated) ----------

#[cfg(feature = "xml-parse")]
pub fn parse_str(source: &str) -> Result<El, String> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(source);
    // no trimming: label text is whitespace-sensitive
    let mut stack: Vec<El> = Vec::new();
    let mut root: Option<El> = None;
    let mut last_closed: Option<El> = None;

    let attach = |el: El,
                  stack: &mut Vec<El>,
                  root: &mut Option<El>|
     -> Result<(), String> {
        match stack.last() {
            Some(parent) => append(parent, &el),
            None => {
                if root.is_some() {
                    return Err("multiple root elements".to_string());
                }
                *root = Some(el);
            }
        }
        Ok(())
    };

    loop {
        match reader.read_event().map_err(|e| e.to_string())? {
            Event::Start(start) => {
                let el = element_from_start(&start)?;
                attach(el.clone(), &mut stack, &mut root)?;
                stack.push(el);
                last_closed = None;
            }
            Event::Empty(start) => {
                let el = element_from_start(&start)?;
                attach(el.clone(), &mut stack, &mut root)?;
                last_closed = Some(el);
            }
            Event::End(_) => {
                last_closed = stack.pop();
            }
            Event::Text(t) => {
                let text = t.unescape().map_err(|e| e.to_string())?.into_owned();
                match &last_closed {
                    // text after a closed sibling is that sibling's tail
                    Some(sibling) => {
                        let mut s = sibling.borrow_mut();
                        s.tail = Some(s.tail.take().unwrap_or_default() + &text);
                    }
                    None => {
                        if let Some(open) = stack.last() {
                            let mut o = open.borrow_mut();
                            o.text = Some(o.text.take().unwrap_or_default() + &text);
                        }
                    }
                }
            }
            Event::CData(t) => {
                let text = String::from_utf8_lossy(&t).into_owned();
                if let Some(open) = stack.last() {
                    let mut o = open.borrow_mut();
                    o.text = Some(o.text.take().unwrap_or_default() + &text);
                }
            }
            Event::Eof => break,
            // comments, processing instructions, doctype: skipped, matching
            // how the Python pipeline ignores them
            _ => {}
        }
    }
    root.ok_or_else(|| "no root element".to_string())
}

#[cfg(feature = "xml-parse")]
fn element_from_start(start: &quick_xml::events::BytesStart) -> Result<El, String> {
    // strip namespace from the tag (Python strips namespaces up front)
    let raw_tag = String::from_utf8_lossy(start.name().as_ref()).into_owned();
    let tag = raw_tag.rsplit(':').next().unwrap_or(&raw_tag).to_string();
    let el = new_element(&tag);
    {
        let mut e = el.borrow_mut();
        for attr in start.attributes() {
            let attr = attr.map_err(|e| e.to_string())?;
            let key = String::from_utf8_lossy(attr.key.as_ref()).into_owned();
            // Drop the default namespace declaration (everything is written
            // back into the svg default namespace), but keep prefixed ones:
            // lxml re-emits xmlns:xlink on embedded MathJax output.
            if key == "xmlns" {
                continue;
            }
            let raw = String::from_utf8_lossy(&attr.value).into_owned();
            let value = quick_xml::escape::unescape(&raw)
                .map_err(|e| e.to_string())?
                .into_owned();
            e.attrs.insert(key, value);
        }
    }
    Ok(el)
}
