//! Port of prefig/core/clip.py: clip contents to a defined shape.

use crate::core::diagram::Diagram;
use crate::xml::{self, El};

pub fn clip(element: &El, diagram: &mut Diagram, parent: &El, outline_group: Option<&El>) {
    let Some(shape_ref) = element.borrow().get("shape") else {
        log::error!("A <clip> tag needs a @shape attribute");
        return;
    };
    let shape_ref = diagram.prepend_id_prefix(&shape_ref);
    let Some(shape) = diagram.recall_shape(&shape_ref) else {
        log::error!("Cannot clip to shape whose name is {shape_ref}");
        return;
    };

    let clip = xml::new_element("clipPath");
    xml::append(&clip, &shape);
    let clip_id = format!("{shape_ref}-clip");
    clip.borrow_mut().set("id", &clip_id);
    diagram.add_reusable(&clip);

    let outline_sub = outline_group.map(|og| {
        let sub = xml::sub_element(og, "g");
        sub.borrow_mut()
            .set("clip-path", &format!("url(#{clip_id})"));
        sub
    });

    let group = xml::sub_element(parent, "g");
    group
        .borrow_mut()
        .set("clip-path", &format!("url(#{clip_id})"));

    diagram.parse(element, &group, outline_sub.as_ref());
}
