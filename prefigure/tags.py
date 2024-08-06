import sys
import lxml.etree as ET
from prefigure import annotations
from prefigure import area
from prefigure import clip
from prefigure import circle
from prefigure import coordinates
from prefigure import definition
from prefigure import diffeqs
from prefigure import graph
from prefigure import grid_axes
from prefigure import group
from prefigure import implicit
from prefigure import label
from prefigure import line
from prefigure import network
from prefigure import parametric_curve
from prefigure import point
from prefigure import polygon
from prefigure import rectangle
from prefigure import riemann_sum
from prefigure import repeat
from prefigure import slope_field
from prefigure import tangent_line
from prefigure import vector

# this dictionary associates tags to a function that processes
#   elements having that tag

tag_dict = {
    'angle-marker': circle.angle,
    'annotations': annotations.annotations,
    'arc': circle.arc,
    'area-between-curves': area.area_between_curves,
    'area-under-curve': area.area_under_curve,
    'axes': grid_axes.axes,
    'caption': label.caption,
    'circle': circle.circle,
    'clip': clip.clip,
    'coordinates': coordinates.coordinates,
    'de-solve': diffeqs.de_solve,
    'definition': definition.definition,
    'derivative': definition.derivative,
    'ellipse': circle.ellipse,
    'graph': graph.graph,
    'grid': grid_axes.grid,
    'grid-axes': grid_axes.grid_axes,
    'group': group.group,
    'implicit-curve': implicit.implicit_curve,
    'label': label.label,
    'line': line.line,
    'network': network.network,
    'parametric-curve': parametric_curve.parametric_curve,
    'plot-de-solution': diffeqs.plot_de_solution,
    'point': point.point,
    'polygon': polygon.polygon,
    'rectangle': rectangle.rectangle,
    'riemann-sum': riemann_sum.riemann_sum,
    'repeat': repeat.repeat,
    'slope-field': slope_field.slope_field,
    'tangent-line': tangent_line.tangent,
    'vector': vector.vector,
}

# apply the processing function based on the XML element's tag

def parse_element(element, diagram, root, outline_status = None):
    if element.tag is ET.Comment:
        return
    try:
        function = tag_dict[element.tag]
    except KeyError:
        print('Unknown element tag: ' + element.tag)
        sys.exit()
    function(element, diagram, root, outline_status)
