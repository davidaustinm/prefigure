import sys
import lxml.etree as ET
from . import annotations
from . import area
from . import clip
from . import circle
from . import coordinates
from . import definition
from . import diffeqs
from . import graph
from . import grid_axes
from . import group
from . import implicit
from . import label
from . import legend
from . import line
from . import network
from . import path
from . import parametric_curve
from . import point
from . import polygon
from . import rectangle
from . import riemann_sum
from . import repeat
from . import shape
from . import slope_field
from . import tangent_line
from . import vector

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
    'define-shapes': shape.define,
    'derivative': definition.derivative,
    'ellipse': circle.ellipse,
    'graph': graph.graph,
    'grid': grid_axes.grid,
    'grid-axes': grid_axes.grid_axes,
    'group': group.group,
    'implicit-curve': implicit.implicit_curve,
    'label': label.label,
    'legend': legend.legend,
    'line': line.line,
    'network': network.network,
    'parametric-curve': parametric_curve.parametric_curve,
    'path': path.path,
    'plot-de-solution': diffeqs.plot_de_solution,
    'point': point.point,
    'polygon': polygon.polygon,
    'rectangle': rectangle.rectangle,
    'riemann-sum': riemann_sum.riemann_sum,
    'repeat': repeat.repeat,
    'shape': shape.shape,
    'slope-field': slope_field.slope_field,
    'tangent-line': tangent_line.tangent,
    'triangle': polygon.triangle,
    'vector': vector.vector
}

# apply the processing function based on the XML element's tag

def parse_element(element, diagram, root, outline_status = None):
    if element.tag is ET.Comment:
        return
    if path.is_path_tag(element.tag):
        print(f"A <{element.tag}> tag can only occur inside a <path>")
        return
    if label.is_label_tag(element.tag):
        print(f"A <{element.tag}> tag can only occur inside a <label>")
        return
    if grid_axes.is_axes_tag(element.tag):
        print(f"A <{element.tag}> tag can only occur inside a <axes> or <grid-axes>")
        return
        
    try:
        function = tag_dict[element.tag]
    except KeyError:
        print('Unknown element tag: ' + element.tag)
        sys.exit()
    function(element, diagram, root, outline_status)
