//! Port of prefig/core/image.py: embed raster and SVG images.

use crate::core::diagram::Diagram;
use crate::core::utilities::float2str;
use crate::core::{ctm, group};
use crate::value::{py_str, Value};
use crate::xml::{self, El};

fn file_type_for(suffix: &str) -> Option<&'static str> {
    match suffix {
        "jpg" | "jpeg" => Some("jpeg"),
        "png" => Some("png"),
        "gif" => Some("gif"),
        "svg" => Some("svg"),
        _ => None,
    }
}

const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(BASE64_CHARS[(n >> 18) as usize & 63] as char);
        out.push(BASE64_CHARS[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            BASE64_CHARS[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            BASE64_CHARS[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

pub fn image(
    element: &El,
    diagram: &mut Diagram,
    parent: &El,
    outline_group: Option<&El>,
) -> Result<(), String> {
    if element.borrow().children.is_empty() {
        log::error!("An <image> must contain content to replace the image in a tactile build");
        return Ok(());
    }

    if diagram.output_format() == "tactile" {
        element.borrow_mut().tag = "group".to_string();
        group::group(element, diagram, parent, outline_group);
        return Ok(());
    }

    let Some(mut source) = element.borrow().get("source") else {
        log::error!("An <image> needs a @source attribute");
        return Ok(());
    };

    let eval_pair = |diagram: &mut Diagram, attr: &str| -> Option<[f64; 2]> {
        let v = diagram.ctx.valid_eval(attr).ok()?.as_vec_f64().ok()?;
        (v.len() >= 2).then(|| [v[0], v[1]])
    };
    let ll_attr = element.borrow().get_or("lower-left", "(0,0)");
    let dims_attr = element.borrow().get_or("dimensions", "(1,1)");
    let (Some(mut ll), Some(dims)) = (eval_pair(diagram, &ll_attr), eval_pair(diagram, &dims_attr))
    else {
        log::error!("Error parsing placement data in an <image>");
        return Ok(());
    };
    let center_attr = element.borrow().get("center");
    let center = match center_attr {
        Some(attr) => {
            let Some(center) = eval_pair(diagram, &attr) else {
                log::error!("Error parsing placement data in an <image>");
                return Ok(());
            };
            ll = [center[0] - 0.5 * dims[0], center[1] - 0.5 * dims[1]];
            center
        }
        None => [ll[0] + 0.5 * dims[0], ll[1] + 0.5 * dims[1]],
    };
    let rotation_attr = element.borrow().get_or("rotate", "0");
    let rotation = diagram
        .ctx
        .valid_eval(&rotation_attr)
        .ok()
        .and_then(|v| v.as_num().ok())
        .unwrap_or(0.0);
    let scale_attr = element.borrow().get_or("scale", "1");
    let scale = diagram.ctx.valid_eval(&scale_attr).ok();

    let file_type = element
        .borrow()
        .get("filetype")
        .as_deref()
        .and_then(file_type_for)
        .or_else(|| source.rsplit('.').next().and_then(file_type_for));
    let Some(file_type) = file_type else {
        log::error!("Cannot determine the type of image in {source}");
        return Ok(());
    };

    let ll_svg = diagram.transform(ll);
    let ur_svg = diagram.transform([ll[0] + dims[0], ll[1] + dims[1]]);
    let center_svg = diagram.transform(center);
    let width = ur_svg[0] - ll_svg[0];
    let height = -(ur_svg[1] - ll_svg[1]);

    if diagram.get_environment() == "pretext" {
        source = format!("data/{source}");
    } else if let Some(assets_dir) = diagram.get_external() {
        let assets_dir = assets_dir.trim();
        source = if assets_dir.ends_with('/') {
            format!("{assets_dir}{source}")
        } else {
            format!("{assets_dir}/{source}")
        };
    }

    let opacity_attr = element.borrow().get("opacity");
    let opacity = opacity_attr.and_then(|attr| {
        diagram
            .ctx
            .valid_eval(&attr)
            .ok()
            .and_then(|v| v.as_num().ok())
    });

    if file_type == "svg" {
        // Python raises here if the file is missing, aborting the siblings
        let svg_source = std::fs::read_to_string(&source)
            .map_err(|_| format!("No such file or directory: '{source}'"))?;
        let svg_root = xml::parse_str(&svg_source)
            .map_err(|e| format!("Unable to parse the SVG image {source}: {e}"))?;
        let svg_width = svg_root.borrow().get("width");
        let svg_height = svg_root.borrow().get("height");

        let object = xml::sub_element(parent, "foreignObject");
        {
            let mut o = object.borrow_mut();
            o.set("x", &float2str(center_svg[0] - width / 2.0));
            o.set("y", &float2str(center_svg[1] - height / 2.0));
            o.set("width", &float2str(width));
            o.set("height", &float2str(height));
        }
        svg_root.borrow_mut().set("width", "100%");
        svg_root.borrow_mut().set("height", "100%");
        if svg_root.borrow().get("viewBox").is_none() {
            svg_root.borrow_mut().set(
                "viewBox",
                &format!(
                    "0 0 {} {}",
                    svg_width.unwrap_or_default(),
                    svg_height.unwrap_or_default()
                ),
            );
        }
        xml::append(&object, &svg_root);
        let id = element.borrow().get("id");
        diagram.add_id(&object, id.as_deref());
        svg_root.borrow_mut().pop_attr("id");
        if let Some(opacity) = opacity {
            object.borrow_mut().set("opacity", &float2str(opacity));
        }
        return Ok(());
    }

    let image_el = xml::sub_element(parent, "image");
    diagram.register_svg_element(element, &image_el);
    {
        let mut i = image_el.borrow_mut();
        i.set("x", &float2str(-width / 2.0));
        i.set("y", &float2str(-height / 2.0));
        i.set("width", &float2str(width));
        i.set("height", &float2str(height));
        if let Some(opacity) = opacity {
            i.set("opacity", &float2str(opacity));
        }
    }
    let id = element.borrow().get("id");
    diagram.add_id(&image_el, id.as_deref());

    // Python raises here if the file is missing, aborting the siblings
    let bytes =
        std::fs::read(&source).map_err(|_| format!("No such file or directory: '{source}'"))?;
    let encoded = base64_encode(&bytes);
    image_el
        .borrow_mut()
        .set("href", &format!("data:image/{file_type};base64,{encoded}"));

    let mut transform_pieces = vec![ctm::translatestr(center_svg[0], center_svg[1])];
    match &scale {
        Some(Value::Array(_)) => {
            if let Some(s) = scale.as_ref().and_then(|v| v.as_vec_f64().ok()) {
                transform_pieces.push(ctm::scalestr(s[0], s[1]));
            }
        }
        Some(v) => {
            // Python compares the evaluated number to the string '1'
            // (`scale != '1'`), which is always true, so a scalar scale is
            // always emitted — even scale(1).
            if let Ok(s) = v.as_num() {
                transform_pieces.push(format!("scale({})", py_str(s)));
            }
        }
        None => {}
    }
    if rotation != 0.0 {
        transform_pieces.push(format!("rotate({})", py_str(-rotation)));
    }
    image_el
        .borrow_mut()
        .set("transform", &transform_pieces.join(" "));
    Ok(())
}
