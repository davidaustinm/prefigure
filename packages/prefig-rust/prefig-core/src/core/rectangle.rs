//! Port of prefig/core/rectangle.py.

use crate::core::ctm::CTM;
use crate::core::diagram::Diagram;
use crate::core::math_utilities::normalize;
use crate::core::utilities::{self as util, pt2str};
use crate::xml::{self, El};

pub fn rectangle(element: &El, diagram: &mut Diagram, parent: &El, outline_group: Option<&El>) {
    let ll_attr = element.borrow().get_or("lower-left", "(0,0)");
    let dims_attr = element.borrow().get_or("dimensions", "(1,1)");
    let center_attr = element.borrow().get("center");

    let eval_pair = |diagram: &mut Diagram, attr: &str| -> Option<[f64; 2]> {
        let v = diagram.ctx.valid_eval(attr).ok()?.as_vec_f64().ok()?;
        (v.len() >= 2).then(|| [v[0], v[1]])
    };
    let (Some(ll), Some(dims)) = (
        eval_pair(diagram, &ll_attr),
        eval_pair(diagram, &dims_attr),
    ) else {
        log::error!("Error parsing data in a <rectangle>");
        return;
    };
    let center = match center_attr {
        Some(attr) => {
            let Some(center) = eval_pair(diagram, &attr) else {
                log::error!("Error parsing data in a <rectangle>");
                return;
            };
            center
        }
        None => [ll[0] + 0.5 * dims[0], ll[1] + 0.5 * dims[1]],
    };

    let path = xml::sub_element(parent, "path");
    let id = element.borrow().get("id");
    diagram.add_id(&path, id.as_deref());
    diagram.register_svg_element(element, &path);

    let rotate_attr = element.borrow().get_or("rotate", "0");
    let rotate = diagram
        .ctx
        .valid_eval(&rotate_attr)
        .ok()
        .and_then(|v| v.as_num().ok())
        .unwrap_or(0.0);
    let mut ctm = CTM::new();
    ctm.translate(center[0], center[1]);
    ctm.rotate(rotate, true);
    let (dx, dy) = (dims[0] / 2.0, dims[1] / 2.0);
    let user_corners: Vec<[f64; 2]> = [[-dx, -dy], [dx, -dy], [dx, dy], [-dx, dy]]
        .iter()
        .map(|&p| ctm.transform(p))
        .collect();
    let mut corners: Vec<[f64; 2]> = user_corners
        .iter()
        .map(|&c| diagram.transform(c))
        .collect();

    let radius_attr = element.borrow().get_or("corner-radius", "0");
    let radius = diagram
        .ctx
        .valid_eval(&radius_attr)
        .ok()
        .and_then(|v| v.as_num().ok())
        .unwrap_or(0.0);
    let cmds = if radius == 0.0 {
        let mut cmds = vec!["M".to_string(), pt2str(corners[0], " ")];
        for c in &corners[1..] {
            cmds.push("L".to_string());
            cmds.push(pt2str(*c, " "));
        }
        cmds.push("Z".to_string());
        cmds
    } else {
        let mut cmds: Vec<String> = Vec::new();
        corners.push(corners[0]);
        corners.push(corners[1]);
        for i in 0..4 {
            let v1 = normalize([
                corners[i + 1][0] - corners[i][0],
                corners[i + 1][1] - corners[i][1],
            ]);
            let v2 = normalize([
                corners[i + 2][0] - corners[i + 1][0],
                corners[i + 2][1] - corners[i + 1][1],
            ]);
            let command = if cmds.is_empty() { "M" } else { "L" };
            cmds.push(command.to_string());
            cmds.push(pt2str(
                [
                    corners[i + 1][0] - radius * v1[0],
                    corners[i + 1][1] - radius * v1[1],
                ],
                " ",
            ));
            cmds.push("Q".to_string());
            cmds.push(pt2str(corners[i + 1], " "));
            cmds.push(pt2str(
                [
                    corners[i + 1][0] + radius * v2[0],
                    corners[i + 1][1] + radius * v2[1],
                ],
                " ",
            ));
        }
        cmds.push("Z".to_string());
        cmds
    };
    path.borrow_mut().set("d", &cmds.join(" "));

    if diagram.output_format() == "tactile" {
        let stroke = element.borrow().get("stroke");
        if stroke.is_some_and(|s| s != "none") {
            element.borrow_mut().set("stroke", "black");
        }
        util::set_tactile_fill(element);
    } else {
        util::set_attr(element, "stroke", "none", &mut diagram.ctx);
        util::set_attr(element, "fill", "none", &mut diagram.ctx);
    }

    util::set_attr(element, "thickness", "2", &mut diagram.ctx);
    let attrs = util::get_2d_attr(element, diagram);
    util::add_attr(&path, attrs);
    util::cliptobbox(&path, element, diagram);

    if let Some(outline_group) = outline_group {
        diagram.add_outline(element, &path, outline_group, None);
        finish_outline(element, diagram, parent);
    } else if element.borrow().get_or("outline", "no") == "yes"
        || diagram.output_format() == "tactile"
    {
        diagram.add_outline(element, &path, parent, None);
        finish_outline(element, diagram, parent);
    } else {
        xml::append(parent, &path);
    }
}

fn finish_outline(element: &El, diagram: &mut Diagram, parent: &El) {
    let stroke = element.borrow().get("stroke");
    let thickness = element.borrow().get("thickness");
    let fill = element.borrow().get_or("fill", "none");
    diagram.finish_outline(element, stroke, thickness, &fill, parent);
}
