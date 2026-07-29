//! Port of prefig/core/coordinates.py: nested coordinate systems.

use crate::core::ctm::AxisScale;
use crate::core::diagram::Diagram;
use crate::core::utilities::float2str;
use crate::value::Value;
use crate::xml::{self, El};

pub fn coordinates(element: &El, diagram: &mut Diagram, root: &El, outline_group: Option<&El>) {
    let current_bbox = diagram.bbox();
    let destination_attr = element.borrow().get("destination");
    let destination: [f64; 4] = match &destination_attr {
        None => current_bbox,
        Some(attr) => {
            let Some(v) = diagram
                .ctx
                .valid_eval(attr)
                .ok()
                .and_then(|v| v.as_vec_f64().ok())
            else {
                log::error!("Error in <coordinates> parsing destination={attr}");
                return;
            };
            [v[0], v[1], v[2], v[3]]
        }
    };

    let lower_left_clip = diagram.transform([destination[0], destination[1]]);
    let upper_right_clip = diagram.transform([destination[2], destination[3]]);

    let dest_dx = upper_right_clip[0] - lower_left_clip[0];
    let dest_dy = -(upper_right_clip[1] - lower_left_clip[1]);

    let bbox_attr = element.borrow().get("bbox").unwrap_or_default();
    let Some(bbox_v) = diagram
        .ctx
        .valid_eval(&bbox_attr)
        .ok()
        .and_then(|v| v.as_vec_f64().ok())
    else {
        log::error!("Error in <coordinates> parsing bbox={bbox_attr}");
        return;
    };
    let mut bbox: [f64; 4] = [bbox_v[0], bbox_v[1], bbox_v[2], bbox_v[3]];

    let ratio_attr = element.borrow().get("aspect-ratio");
    if let Some(attr) = ratio_attr {
        let Some(ratio) = diagram
            .ctx
            .valid_eval(&attr)
            .ok()
            .and_then(|v| v.as_num().ok())
        else {
            log::error!("Error in <coordinates> parsing aspect-ratio={attr}");
            return;
        };
        if element.borrow().get_or("preserve-y-range", "no") == "yes" {
            let box_dy = bbox[3] - bbox[1];
            let y_scale = dest_dy / box_dy;
            let x_scale = ratio * y_scale;
            let box_dx = dest_dx / x_scale;
            bbox = [bbox[0], bbox[1], bbox[0] + box_dx, bbox[3]];
        } else {
            let box_dx = bbox[2] - bbox[0];
            let x_scale = dest_dx / box_dx;
            let y_scale = x_scale / ratio;
            let box_dy = dest_dy / y_scale;
            bbox = [bbox[0], bbox[1], bbox[2], bbox[1] + box_dy];
        }
    }

    let clippath = xml::new_element("clipPath");
    let clip_box = xml::sub_element(&clippath, "rect");
    {
        let mut c = clip_box.borrow_mut();
        c.set("x", &float2str(lower_left_clip[0]));
        c.set("y", &float2str(upper_right_clip[1]));
        c.set("width", &float2str(upper_right_clip[0] - lower_left_clip[0]));
        c.set("height", &float2str(lower_left_clip[1] - upper_right_clip[1]));
    }
    diagram.push_clippath(clippath);

    let scales_attr = element.borrow().get_or("scales", "linear");
    let scales = match scales_attr.as_str() {
        "semilogx" => [AxisScale::Log, AxisScale::Linear],
        "semilogy" => [AxisScale::Linear, AxisScale::Log],
        "loglog" => [AxisScale::Log, AxisScale::Log],
        _ => [AxisScale::Linear, AxisScale::Linear],
    };

    let mut ctm = diagram.ctm_ref().clone();
    diagram.push_scales(scales);
    let mut scaled_bbox = bbox;
    if scales[0] == AxisScale::Log {
        scaled_bbox[0] = scaled_bbox[0].log10();
        scaled_bbox[2] = scaled_bbox[2].log10();
        ctm.set_log_x();
    }
    if scales[1] == AxisScale::Log {
        scaled_bbox[1] = scaled_bbox[1].log10();
        scaled_bbox[3] = scaled_bbox[3].log10();
        ctm.set_log_y();
    }

    ctm.translate(destination[0], destination[1]);
    ctm.scale(
        (destination[2] - destination[0]) / (scaled_bbox[2] - scaled_bbox[0]),
        (destination[3] - destination[1]) / (scaled_bbox[3] - scaled_bbox[1]),
    );
    ctm.translate(-scaled_bbox[0], -scaled_bbox[1]);
    diagram.ctx.enter_namespace(
        "bbox",
        Value::Array(scaled_bbox.iter().map(|&b| Value::Num(b)).collect()),
    );

    diagram.push_ctm(ctm, bbox);
    diagram.parse(element, root, outline_group);
    diagram.pop_ctm();
    diagram.pop_clippath();
    diagram.pop_scales();
}
