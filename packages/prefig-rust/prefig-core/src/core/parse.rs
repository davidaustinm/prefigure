//! Port of prefig/core/parse.py: build a Diagram from a <diagram> element.

use crate::core::diagram::Diagram;
use crate::core::label::LabelState;
use crate::xml::{self, El};
use std::collections::HashSet;

/// The main work of constructing a diagram; returns (svg, annotations).
#[allow(clippy::too_many_arguments)]
pub fn mk_diagram(
    element: &El,
    format: &str,
    publication: Option<El>,
    filename: &str,
    suppress_caption: bool,
    diagram_number: Option<i64>,
    environment: &str,
    labels: LabelState,
) -> Result<(String, Option<String>), String> {
    let mut diagram = Diagram::new(
        element.clone(),
        filename,
        diagram_number,
        format,
        publication,
        suppress_caption,
        environment,
        labels,
    );

    log::debug!("Initializing PreFigure diagram");
    diagram.begin_figure()?;
    log::debug!("Processing PreFigure elements");
    let root = diagram.root.clone();
    diagram.parse(element, &root, None);
    log::debug!("Positioning labels");
    diagram.place_labels();
    log::debug!("Writing the diagram and any annotations");
    diagram.annotate_source();
    Ok(diagram.end_figure_to_string())
}

pub fn check_duplicate_handles(element: &El, handles: &mut HashSet<String>) {
    let children: Vec<El> = element.borrow().children.clone();
    for child in children {
        for attr in ["id", "at"] {
            if let Some(id) = child.borrow().get(attr) {
                if handles.contains(&id) {
                    log::warn!("Duplicate handle: {id}.  Unexpected behavior could result.");
                } else {
                    handles.insert(id);
                }
            }
        }
        check_duplicate_handles(&child, handles);
    }
}

/// Find the <diagram> elements in a parsed document (namespaces are already
/// stripped by the parser).
pub fn find_diagrams(tree: &El) -> Vec<El> {
    let mut diagrams = Vec::new();
    if tree.borrow().tag == "diagram" {
        diagrams.push(tree.clone());
    }
    diagrams.extend(xml::find_descendants(tree, "diagram"));
    diagrams
}
