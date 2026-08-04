//! Port of prefig/core/annotations.py: the accessibility annotation tree.

use crate::core::diagram::Diagram;
use crate::xml::{self, El};
use std::collections::HashMap;

fn el_key(el: &El) -> usize {
    std::rc::Rc::as_ptr(el) as usize
}

// `parent`/`outline_group` are part of the shared element-handler signature; this
// handler only threads them through its own recursion, but must still accept them.
#[allow(clippy::only_used_in_recursion)]
pub fn annotations(element: &El, diagram: &mut Diagram, parent: &El, outline_group: Option<&El>) {
    // tactile diagrams have no annotations
    if diagram.output_format() == "tactile" && diagram.get_environment() != "pyodide" {
        return;
    }

    // a top-level annotations element carrying text acts as the top annotation
    if element.borrow().get("text").is_some() {
        element.borrow_mut().tag = "annotation".to_string();
        element.borrow_mut().set("ref", "figure");
        let root = xml::new_element("annotations");
        xml::append(&root, element);
        annotations(&root, diagram, parent, outline_group);
        return;
    }

    diagram.initialize_annotations();

    let default_annotations = diagram.get_default_annotations();
    let mut defaults_added = false;
    let children: Vec<El> = element.borrow().children.clone();
    for subelement in &children {
        if !defaults_added {
            for (index, annotation) in default_annotations.iter().enumerate() {
                xml::insert(subelement, index, annotation);
            }
            defaults_added = true;
        }
        annotate(subelement, diagram, None);
    }
}

pub fn annotate(element: &El, diagram: &mut Diagram, parent: Option<&El>) {
    let parent = match parent {
        Some(p) => p.clone(),
        None => match diagram.get_annotations_root() {
            Some(root) => root,
            None => return,
        },
    };

    let ref_attr = element.borrow().get("ref");
    match ref_attr {
        Some(ref_attr) => {
            let ref_id = diagram.prepend_id_prefix(&ref_attr);
            element.borrow_mut().set("id", &ref_id);
            element.borrow_mut().pop_attr("ref");
        }
        None => log::info!("An annotation has an empty attribute ref"),
    }
    element.borrow_mut().pop_attr("annotate");

    // a reference to an annotation branch created by <repeat annotate="yes">?
    let id = element.borrow().get_or("id", "none");
    let id = diagram.prepend_id_prefix(&id);
    if let Some(branch) = diagram.get_annotation_branch(&id) {
        annotate(&branch, diagram, Some(&parent));
        return;
    }

    let annotation = xml::new_element("annotation");
    diagram.add_annotation(&annotation);
    annotation
        .borrow_mut()
        .set("id", &element.borrow().get_or("id", "none"));

    let mut active = false;
    let attrs: Vec<(String, String)> = element
        .borrow()
        .attrs
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    for (key, value) in &attrs {
        if key == "text" {
            active = true;
            annotation.borrow_mut().set("speech2", value);
        } else {
            annotation.borrow_mut().set(key, value);
        }
    }

    let kind = if !element.borrow().children.is_empty() {
        "grouped"
    } else if active {
        "active"
    } else {
        "passive"
    };
    let el = xml::sub_element(&annotation, kind);
    el.borrow_mut().text = element.borrow().get("id");

    // position within the parent
    let toplevel = parent.borrow().tag == "annotations";
    let pos = xml::new_element("position");
    let position = if active {
        if toplevel {
            parent.borrow().children.len()
        } else {
            let children_el = match xml::find(&parent, "children") {
                Some(c) => c,
                None => xml::sub_element(&parent, "children"),
            };
            let position = children_el.borrow().children.len() + 1;
            let child = xml::sub_element(&children_el, "active");
            child.borrow_mut().text = annotation.borrow().get("id");
            position
        }
    } else {
        0
    };
    pos.borrow_mut().text = Some(position.to_string());
    xml::append(&annotation, &pos);

    // register with the parent's components
    if !toplevel {
        let components = match xml::find(&parent, "components") {
            Some(c) => c,
            None => xml::sub_element(&parent, "components"),
        };
        let component = xml::sub_element(&components, if active { "active" } else { "passive" });
        component.borrow_mut().text = annotation.borrow().get("id");
    }

    // descend the tree
    let children: Vec<El> = element.borrow().children.clone();
    for subelement in &children {
        annotate(subelement, diagram, Some(&annotation));
    }

    if !toplevel {
        let parents = xml::new_element("parents");
        let kind = if xml::find(&parent, "grouped").is_some() {
            "grouped"
        } else {
            "active"
        };
        let comp = xml::sub_element(&parents, kind);
        comp.borrow_mut().text = parent.borrow().get("id");
        xml::append(&annotation, &parents);
    }

    if element.borrow().get_or("sonify", "no") == "yes" {
        let sonification = xml::sub_element(&annotation, "sonification");
        let active_el = xml::sub_element(&sonification, "ACTIVE");
        active_el.borrow_mut().text = element.borrow().get("id");
    }
}

/// Speech-friendly pronunciation for a tag (`pronounciations` table).
fn pronounciation(tag: &str) -> &str {
    match tag {
        "de-solve" => "D E solve",
        "define-shapes" => "define shapes",
        "angle-marker" => "angle marker",
        "area-between-curves" => "area between curves",
        "area-under-curve" => "area under curve",
        "grid-axes" => "grid axes",
        "implicit-curve" => "implicit curve",
        "parametric-curve" => "parametric curve",
        "plot-de-solution" => "plot D E solution",
        "riemann-sum" => "Riemann sum",
        "slope-field" => "slope field",
        "tick-mark" => "tick mark",
        "tangent-line" => "tangent line",
        "vector-field" => "vector field",
        other => other,
    }
}

/// Elements whose label text should be read out (`labeled_elements`).
fn is_labeled_element(tag: &str) -> bool {
    matches!(
        tag,
        "label"
            | "point"
            | "xlabel"
            | "ylabel"
            | "angle-marker"
            | "tick-mark"
            | "item"
            | "node"
            | "edge"
    )
}

/// Speech for a label sub-element tag, if it is one (`label_subelements`).
fn label_subelement_speech(tag: &str) -> Option<&'static str> {
    match tag {
        "m" => Some("math"),
        "b" => Some("bold"),
        "it" => Some("italics"),
        "plain" => Some("plain"),
        "newline" => Some("new line"),
        _ => None,
    }
}

/// Port of diagram_to_speech: rewrite a pristine copy of the source diagram
/// into an annotation tree, describing each element and (via `source_to_svg`)
/// linking it to the SVG element it produced. Mutates `diagram` in place.
pub fn diagram_to_speech(diagram: &El, source_to_svg: &HashMap<usize, El>) {
    let mut element_num = 0;
    // Snapshot in document order: the <annotation> children we append below are
    // added during the walk and Python's live iterator skips them (tag ==
    // 'annotation'); a snapshot simply never visits them.
    for element in xml::iter_subtree(diagram) {
        let tag = element.borrow().tag.clone();
        if tag == "annotation" {
            continue;
        }
        // strip label sub-elements (m/b/it/plain/newline) from the tree
        if label_subelement_speech(&tag).is_some() {
            if let Some(parent) = xml::get_parent(&element) {
                xml::remove(&parent, &element);
            }
            continue;
        }

        // save the author's attributes, then clear them
        let attribs: Vec<(String, String)> = element
            .borrow()
            .attrs
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        element.borrow_mut().attrs.clear();

        let intro = if tag == "diagram" {
            element.borrow_mut().set("ref", "figure");
            "This prefigure source file begins with a diagram having these attributes: ".to_string()
        } else if tag == "definition" {
            element
                .borrow_mut()
                .set("ref", &format!("element-{element_num}"));
            let text = element.borrow().text.clone().unwrap_or_default();
            format!("A definition element defining {}", text.trim())
        } else if is_labeled_element(&tag) {
            element
                .borrow_mut()
                .set("ref", &format!("element-{element_num}"));
            let tag_speech = pronounciation(&tag).to_string();
            let label_text = label_to_speech(&element);
            let intro = if !label_text.is_empty() {
                if attribs.is_empty() {
                    format!(
                        "A {tag_speech} element with label {label_text}.  \
                         The element has no attributes."
                    )
                } else {
                    format!(
                        "A {tag_speech} element with label {label_text}.  \
                         There are these attributes: "
                    )
                }
            } else if attribs.is_empty() {
                format!("A {tag_speech} element with no attributes.")
            } else {
                format!("A {tag_speech} element with these attributes: ")
            };
            element.borrow_mut().text = None;
            intro
        } else {
            element
                .borrow_mut()
                .set("ref", &format!("element-{element_num}"));
            let tag_speech = pronounciation(&tag).to_string();
            if attribs.is_empty() {
                format!("A {tag_speech} element with no attributes")
            } else {
                format!("A {tag_speech} element with these attributes: ")
            }
        };

        let text = format!("{intro}{}", attributes_to_speech(&attribs));
        element.borrow_mut().set("text", &text);
        element_num += 1;
        element.borrow_mut().tag = "annotation".to_string();

        if let Some(svg) = source_to_svg.get(&el_key(&element)) {
            if let Some(svg_id) = svg.borrow().get("id") {
                let child = xml::sub_element(&element, "annotation");
                child.borrow_mut().set("ref", &svg_id);
            }
        }
    }
}

fn attributes_to_speech(attribs: &[(String, String)]) -> String {
    attribs
        .iter()
        .map(|(key, value)| format!("{key} has value {value}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn label_to_speech(element: &El) -> String {
    let mut strings: Vec<String> = Vec::new();
    if let Some(text) = element.borrow().text.clone() {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            strings.push(trimmed.to_string());
        }
    }
    let children: Vec<El> = element.borrow().children.clone();
    for child in &children {
        let ctag = child.borrow().tag.clone();
        let child_speech = label_subelement_speech(&ctag)
            .map(|s| s.to_string())
            .unwrap_or(ctag);
        strings.push(format!("begin {child_speech}"));
        let ctext = child.borrow().text.clone().unwrap_or_default();
        strings.push(ctext.trim().to_string());
        strings.push(format!("end {child_speech}"));
        if let Some(tail) = child.borrow().tail.clone() {
            strings.push(tail.trim().to_string());
        }
    }
    strings.join(" ")
}
