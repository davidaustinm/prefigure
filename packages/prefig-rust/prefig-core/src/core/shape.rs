//! Port of prefig/core/shape.py: named shapes and boolean operations. Boolean
//! ops (union/intersection/difference/symmetric-difference/convex-hull) use the
//! `geo` crate behind the `shapes` feature; results are geometrically correct
//! but not vertex-identical to Python's shapely.

use crate::core::diagram::Diagram;
use crate::core::tags;
use crate::core::utilities::{self as util};
use crate::xml::{self, El};

const ALLOWED_SHAPES: [&str; 12] = [
    "arc",
    "area-between-curves",
    "area-under-curve",
    "circle",
    "ellipse",
    "graph",
    "parametric-curve",
    "path",
    "polygon",
    "rectangle",
    "shape",
    "spline",
];

pub fn define(element: &El, diagram: &mut Diagram, _parent: &El, _outline_group: Option<&El>) {
    let children: Vec<El> = element.borrow().children.clone();
    for child in &children {
        let tag = child.borrow().tag.clone();
        if !ALLOWED_SHAPES.contains(&tag.as_str()) {
            log::error!("In <define-shapes>, {tag} does not define a shape");
            continue;
        }
        let at = child.borrow().get("at");
        if let Some(at) = at {
            let id = diagram.prepend_id_prefix(&at);
            child.borrow_mut().set("id", &id);
        }
        let dummy_parent = xml::new_element("group");
        // build the shape in svg mode even for tactile output
        let format = diagram.output_format().to_string();
        diagram.set_output_format("svg");
        let _ = tags::parse_element(child, diagram, &dummy_parent, None);
        diagram.set_output_format(&format);
        let shape = dummy_parent.borrow().children.first().cloned();
        let Some(shape) = shape else {
            continue;
        };
        {
            let mut s = shape.borrow_mut();
            s.pop_attr("stroke");
            s.pop_attr("fill");
            s.pop_attr("stroke-width");
        }
        diagram.add_shape(&shape);
    }
}

pub fn shape(element: &El, diagram: &mut Diagram, parent: &El, outline_group: Option<&El>) {
    let reference = element
        .borrow()
        .get("shapes")
        .or_else(|| element.borrow().get("shape"));
    let Some(reference) = reference else {
        log::error!("A <shape> tag needs a @shape or @shapes attribute");
        return;
    };

    let mut operation = element.borrow().get("operation");
    let shape_refs: Vec<String> = reference
        .split(',')
        .map(|r| diagram.prepend_id_prefix(r.trim()))
        .collect();

    // Python defaults a multi-shape reference with no operation to union.
    if operation.is_none() && shape_refs.len() > 1 {
        operation = Some("union".to_string());
    }

    let path = match operation {
        // a single shape with no operation is just a <use> of it
        None => {
            let use_el = xml::sub_element(parent, "use");
            let full_reference = diagram.prepend_id_prefix(&reference);
            use_el
                .borrow_mut()
                .set("href", &format!("#{full_reference}"));
            use_el
        }
        Some(operation) => {
            let mut paths: Vec<String> = Vec::new();
            for shape_ref in &shape_refs {
                match diagram.recall_shape(shape_ref) {
                    Some(shape) => {
                        if let Some(d) = shape.borrow().get("d") {
                            paths.push(d);
                        }
                    }
                    None => log::error!("{shape_ref} is not a reference to a shape"),
                }
            }
            let Some(d) = apply_operation(&operation, &paths) else {
                return;
            };
            let path = xml::sub_element(parent, "path");
            path.borrow_mut().set("d", &d);
            path
        }
    };

    let id = element.borrow().get("id");
    diagram.add_id(&path, id.as_deref());
    diagram.register_svg_element(element, &path);

    if diagram.output_format() == "tactile" {
        if element.borrow().get("stroke").is_some() {
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
    let fill = element.borrow().get_or("fill", "None");
    diagram.finish_outline(element, stroke, thickness, &fill, parent);
}

/// Apply a boolean geometry operation over the given SVG path `d` strings,
/// returning the result as a new path `d` string. Mirrors shape.py's shapely
/// calls (union/intersection/difference/symmetric-difference/convex-hull).
#[cfg(feature = "shapes")]
fn apply_operation(operation: &str, paths: &[String]) -> Option<String> {
    use geo::{BooleanOps, ConvexHull, MultiPolygon};

    let convex = operation == "convex-hull" || operation == "convex hull";
    let geometries: Vec<MultiPolygon<f64>> = paths.iter().map(|p| build_multipolygon(p)).collect();

    let result: MultiPolygon<f64> = match operation {
        "intersection" => {
            if geometries.len() < 2 {
                log::error!("Intersections require more than one shape");
                return None;
            }
            geometries
                .iter()
                .skip(1)
                .fold(geometries[0].clone(), |acc, g| acc.intersection(g))
        }
        "union" => {
            if geometries.len() < 2 {
                log::error!("Unions require more than one shape");
                return None;
            }
            geometries
                .iter()
                .skip(1)
                .fold(geometries[0].clone(), |acc, g| acc.union(g))
        }
        "difference" => {
            if geometries.len() != 2 {
                log::error!("Differences require exactly two shapes");
                return None;
            }
            geometries[0].difference(&geometries[1])
        }
        "symmetric-difference" | "sym-diff" => {
            if geometries.len() < 2 {
                log::error!("Symmetric differences require more than one shape");
                return None;
            }
            geometries
                .iter()
                .skip(1)
                .fold(geometries[0].clone(), |acc, g| acc.xor(g))
        }
        "convex-hull" | "convex hull" => {
            let first = geometries.first().cloned()?;
            let unioned = geometries.iter().skip(1).fold(first, |acc, g| acc.union(g));
            MultiPolygon::new(vec![unioned.convex_hull()])
        }
        other => {
            log::error!("Unknown shape operation: {other}");
            return None;
        }
    };
    let _ = convex;

    if result.0.is_empty() {
        log::warn!("The {operation} is empty");
        return None;
    }
    Some(multipolygon_to_path(&result))
}

#[cfg(not(feature = "shapes"))]
fn apply_operation(_operation: &str, _paths: &[String]) -> Option<String> {
    log::error!("<shape> boolean operations need the `shapes` feature (geo crate)");
    None
}

/// Parse an SVG path `d` into a MultiPolygon: each Z-closed subpath becomes one
/// polygon; quadratic and cubic Béziers are sampled into line segments (N=30,
/// matching shape.py).
#[cfg(feature = "shapes")]
fn build_multipolygon(path: &str) -> geo::MultiPolygon<f64> {
    use geo::{Coord, LineString, MultiPolygon, Polygon};

    const N: usize = 30;
    let tokens: Vec<&str> = path.split_whitespace().collect();
    let mut i = 0;
    let mut polygons: Vec<Polygon<f64>> = Vec::new();
    let mut points: Vec<Coord<f64>> = Vec::new();

    let coord = |a: &str, b: &str| Coord {
        x: a.parse::<f64>().unwrap_or(0.0),
        y: b.parse::<f64>().unwrap_or(0.0),
    };

    while i < tokens.len() {
        let token = tokens[i].to_uppercase();
        i += 1;
        match token.as_str() {
            "M" | "L" => {
                if i + 1 < tokens.len() {
                    points.push(coord(tokens[i], tokens[i + 1]));
                    i += 2;
                }
            }
            "Q" => {
                if i + 3 < tokens.len() {
                    let p0 = *points.last().unwrap_or(&Coord { x: 0.0, y: 0.0 });
                    let p1 = coord(tokens[i], tokens[i + 1]);
                    let p2 = coord(tokens[i + 2], tokens[i + 3]);
                    i += 4;
                    for k in 0..=N {
                        let t = k as f64 / N as f64;
                        let mt = 1.0 - t;
                        points.push(Coord {
                            x: mt * mt * p0.x + 2.0 * t * mt * p1.x + t * t * p2.x,
                            y: mt * mt * p0.y + 2.0 * t * mt * p1.y + t * t * p2.y,
                        });
                    }
                }
            }
            "C" => {
                if i + 5 < tokens.len() {
                    let p0 = *points.last().unwrap_or(&Coord { x: 0.0, y: 0.0 });
                    let p1 = coord(tokens[i], tokens[i + 1]);
                    let p2 = coord(tokens[i + 2], tokens[i + 3]);
                    let p3 = coord(tokens[i + 4], tokens[i + 5]);
                    i += 6;
                    for k in 0..=N {
                        let t = k as f64 / N as f64;
                        let mt = 1.0 - t;
                        points.push(Coord {
                            x: mt * mt * mt * p0.x
                                + 3.0 * t * mt * mt * p1.x
                                + 3.0 * t * t * mt * p2.x
                                + t * t * t * p3.x,
                            y: mt * mt * mt * p0.y
                                + 3.0 * t * mt * mt * p1.y
                                + 3.0 * t * t * mt * p2.y
                                + t * t * t * p3.y,
                        });
                    }
                }
            }
            "Z" if !points.is_empty() => {
                polygons.push(Polygon::new(
                    LineString::new(std::mem::take(&mut points)),
                    vec![],
                ));
            }
            _ => {}
        }
    }
    if !points.is_empty() {
        polygons.push(Polygon::new(LineString::new(points), vec![]));
    }
    MultiPolygon::new(polygons)
}

/// Serialize a MultiPolygon to an SVG path `d`, with coordinates at one decimal
/// place (matching shape.py's cleanup_str + float2str).
#[cfg(feature = "shapes")]
fn multipolygon_to_path(mp: &geo::MultiPolygon<f64>) -> String {
    use crate::core::utilities::float2str;
    let mut parts: Vec<String> = Vec::new();
    for polygon in &mp.0 {
        for ring in std::iter::once(polygon.exterior()).chain(polygon.interiors()) {
            let coords: Vec<geo::Coord<f64>> = ring.coords().copied().collect();
            if coords.is_empty() {
                continue;
            }
            let mut cmds = vec![format!(
                "M {} {}",
                float2str(coords[0].x),
                float2str(coords[0].y)
            )];
            for c in &coords[1..] {
                cmds.push(format!("L {} {}", float2str(c.x), float2str(c.y)));
            }
            cmds.push("z".to_string());
            parts.push(cmds.join(" "));
        }
    }
    parts.join(" ")
}
