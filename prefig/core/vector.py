import lxml.etree as ET
import logging
import math
import numpy as np
from . import user_namespace as un
from . import utilities as util
from . import math_utilities as math_util
from . import arrow

log = logging.getLogger('prefigure')

# Add a graphical element describing a vector
def vector(element, diagram, parent, outline_status):
    if outline_status == 'finish_outline':
        finish_outline(element, diagram, parent)
        return

    # v describes the mathematical vector (displacement),
    # which is scaled by scale, tail is the location of 
    # the tail
    try:
        v = un.valid_eval(element.get('v'))
    except:
        log.error(f"Error parsing vector attribute @v={element.get('v')}")
        return

    tail = un.valid_eval(element.get('tail', '[0,0]'))
    scale = un.valid_eval(element.get('scale', '1'))
    v = scale * v
    w = v + tail

    # Specify where we want the head to appear.  By default,
    # it's at the tip of the vector, but it can be anywhere
    # along the shaft at a location specified by 0 <= t <= 1
    t = element.get('head-location', None)
    if t is not None:
        t = float(t)
        head_loc = (1-t)*tail + t * w

    if diagram.output_format() == 'tactile':
        element.set('fill', 'black')
        element.set('stroke', 'black')
    else:
        util.set_attr(element, 'stroke', 'black')
        util.set_attr(element, 'fill', 'none')
    util.set_attr(element, 'thickness', '3')

    vector = ET.Element('path')
    diagram.add_id(vector, element.get('id'))
    diagram.register_svg_element(element, vector)
    util.add_attr(vector, util.get_2d_attr(element))

    # Now add the head using an SVG marker
    if t is not None:
        location = 'marker-mid'
    else:
        location = 'marker-end'
    arrow_id = arrow.add_arrowhead_to_path(
        diagram,
        location,
        vector,
        arrow_width=element.get('arrow-width', None),
        arrow_angles=element.get('arrow-angles', None)
    )

    # we need to pull the tip of the vector in a bit to accommodate
    # the arrowhead
    p0 = diagram.transform(tail)
    p1 = diagram.transform(w)
    diff = p1 - p0
    length = math_util.length(diff)
    angle = math.atan2(diff[1], diff[0])

    arrow_head_length = arrow.get_arrow_length(arrow_id)
    thickness = un.valid_eval(element.get('thickness'))
    if location == 'marker-end':
        length -= thickness * arrow_head_length
        p1 = length * np.array([math.cos(angle),math.sin(angle)]) + p0

    # Here is the shaft of the vector.  If the head is not at
    # the tip, we add a waypoint along the line where the head
    # will appear
    cmds = []

    cmds.append('M ' + util.pt2str(p0))
    if t is not None:
        cmds.append('L ' + util.pt2str(diagram.transform(head_loc)))
    cmds.append('L ' + util.pt2str(p1))
    d = ' '.join(cmds)
    vector.set('d', d)

    if outline_status == 'add_outline':
        diagram.add_outline(element, vector, parent)
        return

    if element.get('outline', 'no') == 'yes' or diagram.output_format() == 'tactile':
        diagram.add_outline(element, vector, parent)
        finish_outline(element, diagram, parent)
    else:
        parent.append(vector)

def finish_outline(element, diagram, parent):
    diagram.finish_outline(element,
                           element.get('stroke'),
                           element.get('thickness'),
                           element.get('fill', 'none'),
                           parent)
