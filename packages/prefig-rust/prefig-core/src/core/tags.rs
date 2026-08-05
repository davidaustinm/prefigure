//! Port of prefig/core/tags.py: dispatch an XML element to its handler.

use crate::core::diagram::Diagram;
use crate::core::{
    annotations, area, axes, circle, clip, coordinates, ctm_handlers, definition, diffeqs, graph,
    grid_axes, group, image, implicit, label, legend, line, network, parametric_curve, path, point,
    polygon, read, rectangle, repeat, riemann_sum, shape, slope_field, statistics, tangent_line,
    vector,
};
use crate::xml::El;

const PATH_TAGS: [&str; 10] = [
    "moveto",
    "rmoveto",
    "lineto",
    "rlineto",
    "horizontal",
    "vertical",
    "cubic-bezier",
    "quadratic-bezier",
    "smooth-cubic",
    "smooth-quadratic",
];

pub fn parse_element(
    element: &El,
    diagram: &mut Diagram,
    root: &El,
    outline_group: Option<&El>,
) -> Result<(), String> {
    let tag = element.borrow().tag.clone();

    if PATH_TAGS.contains(&tag.as_str()) {
        log::warn!("A <{tag}> tag can only occur inside a <path>");
        return Ok(());
    }
    if label::is_label_tag(&tag) {
        log::warn!("A <{tag}> tag can only occur inside a <label>");
        return Ok(());
    }
    if axes::is_axes_tag(&tag) {
        log::warn!("A <{tag}> tag can only occur inside a <axes> or <grid-axes>");
        return Ok(());
    }

    match tag.as_str() {
        "angle-marker" => circle::angle(element, diagram, root, outline_group),
        "annotations" => annotations::annotations(element, diagram, root, outline_group),
        "area-between-curves" => area::area_between_curves(element, diagram, root, outline_group),
        "area-under-curve" => area::area_under_curve(element, diagram, root, outline_group),
        "arc" => circle::arc(element, diagram, root, outline_group),
        "axes" => axes::axes(element, diagram, root, outline_group),
        "caption" => label::caption(element, diagram, root, outline_group),
        "center" => ctm_handlers::transform_center(element, diagram, root, outline_group),
        "change-basis" => ctm_handlers::transform_basis(element, diagram, root, outline_group),
        "circle" => circle::circle(element, diagram, root, outline_group),
        "clip" => clip::clip(element, diagram, root, outline_group),
        "contour" => implicit::implicit_curve(element, diagram, root, outline_group),
        "coordinates" => coordinates::coordinates(element, diagram, root, outline_group),
        "de-solve" => diffeqs::de_solve(element, diagram, root, outline_group),
        "define-shapes" => shape::define(element, diagram, root, outline_group),
        "definition" => definition::definition(element, diagram, root, outline_group),
        "derivative" => definition::derivative(element, diagram, root, outline_group),
        "ellipse" => circle::ellipse(element, diagram, root, outline_group),
        "graph" => graph::graph(element, diagram, root, outline_group),
        "image" => image::image(element, diagram, root, outline_group)?,
        "histogram" => statistics::histogram(element, diagram, root, outline_group),
        "implicit-curve" => implicit::implicit_curve(element, diagram, root, outline_group),
        "grid" => grid_axes::grid(element, diagram, root, outline_group),
        "grid-axes" => grid_axes::grid_axes(element, diagram, root, outline_group),
        "group" => group::group(element, diagram, root, outline_group),
        "label" => label::label(element, diagram, root, outline_group),
        "legend" => legend::legend(element, diagram, root, outline_group),
        "network" => network::network(element, diagram, root, outline_group),
        "parametric-curve" => {
            parametric_curve::parametric_curve(element, diagram, root, outline_group)
        }
        "path" => path::path(element, diagram, root, outline_group),
        "polygon" => polygon::polygon_handler(element, diagram, root, outline_group),
        "line" => line::line(element, diagram, root, outline_group),
        "plot-de-solution" => diffeqs::plot_de_solution(element, diagram, root, outline_group),
        "point" => point::point(element, diagram, root, outline_group),
        "read" => read::read(element, diagram, root, outline_group),
        "rectangle" => rectangle::rectangle(element, diagram, root, outline_group),
        "riemann-sum" => riemann_sum::riemann_sum(element, diagram, root, outline_group),
        "repeat" => repeat::repeat(element, diagram, root, outline_group),
        "rotate" => ctm_handlers::transform_rotate(element, diagram, root, outline_group),
        "scale" => ctm_handlers::transform_scale(element, diagram, root, outline_group),
        "scale3d" => ctm_handlers::transform_scale3d(element, diagram, root, outline_group),
        "scatter" => statistics::scatter(element, diagram, root, outline_group),
        "set-eye" => ctm_handlers::set_eye(element, diagram, root, outline_group),
        "shape" => shape::shape(element, diagram, root, outline_group),
        "slope-field" => slope_field::slope_field(element, diagram, root, outline_group),
        "spline" => polygon::spline(element, diagram, root, outline_group),
        "tangent-line" => tangent_line::tangent(element, diagram, root, outline_group),
        "tick-mark" => axes::tick_mark(element, diagram, root, outline_group),
        "transform" => ctm_handlers::transform_group(element, diagram, root, outline_group),
        "triangle" => polygon::triangle(element, diagram, root, outline_group),
        "translate" => ctm_handlers::transform_translate(element, diagram, root, outline_group),
        "translate3d" => ctm_handlers::transform_translate3d(element, diagram, root, outline_group),
        "vector" => vector::vector(element, diagram, root, outline_group),
        "vector-field" => slope_field::vector_field(element, diagram, root, outline_group),
        _ => {
            log::error!("Unknown element tag: {tag}");
        }
    }
    Ok(())
}
