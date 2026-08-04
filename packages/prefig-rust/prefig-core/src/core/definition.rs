//! Port of prefig/core/definition.py.

use crate::core::diagram::Diagram;
use crate::xml::El;

pub fn definition(element: &El, diagram: &mut Diagram, parent: &El, outline_group: Option<&El>) {
    let substitution = element.borrow().get_or("substitution", "yes") == "yes";
    let text = element.borrow().text.clone().unwrap_or_default();
    if let Err(e) = diagram
        .ctx
        .define_with_substitution(text.trim(), substitution)
    {
        log::error!("Error in definition: {e}");
    }

    let id_suffix = element.borrow().get("id-suffix");
    if let Some(id_suffix) = id_suffix {
        // this definition is part of a repeat
        diagram.push_id_suffix(&format!("-{id_suffix}"));
        diagram.parse(element, parent, outline_group);
        diagram.pop_id_suffix();
    }
}

pub fn derivative(element: &El, diagram: &mut Diagram, _parent: &El, _outline_group: Option<&El>) {
    let function_attr = element.borrow().get("function").unwrap_or_default();
    let f = match diagram.ctx.valid_eval(&function_attr) {
        Ok(f @ crate::value::Value::Function(_)) => f,
        Ok(_) | Err(_) => {
            log::error!("Error retrieving function in <derivative>");
            return;
        }
    };
    let Some(name) = element.borrow().get("name") else {
        log::error!("A <derivative> element needs a name attribute");
        return;
    };
    diagram.ctx.register_derivative(&name, f);
}
