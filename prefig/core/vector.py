import lxml.etree as ET
import logging
import math
import numpy as np
import copy
from . import user_namespace as un
from . import utilities as util
from . import math_utilities as math_util
from . import arrow
from . import label

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

    diagram.register_source_data(element, 'v', v)
    diagram.register_source_data(element, 'head', w)

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
    diagram.register_source_data(element, 'angle', angle)

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
        original_parent = parent
        parent = add_label(element, diagram, parent)
        parent.append(vector)

        # no label has been added if the parent hasn't changed
        if original_parent == parent:
            diagram.register_svg_element(element, vector)
            return

        diagram.register_svg_element(element, parent)
        # if there is a label, then the id is on the outer <g> element
        # so we need to remove it from the children
        if element.get('id', 'none') == parent.get('id'):
            element.attrib.pop('id')
        for child in parent:
            if child.get('id', None) is not None:
                child.attrib.pop('id')

def finish_outline(element, diagram, parent):
    original_parent = parent
    parent = add_label(element, diagram, parent)

    # if we've added a label, remove the id's from element under the parent <g>
    if original_parent != parent:
        for child in parent:
            if child.get('id', None) is not None:
                child.attrib.pop('id')

    diagram.finish_outline(element,
                           element.get('stroke'),
                           element.get('thickness'),
                           element.get('fill', 'none'),
                           parent)

# dictionary for alignments and offsets based on half-quadrant
alignment_dict = {0:('se',-1),
                  1:('nw',1),
                  2:('ne',-1),
                  3:('sw',1),
                  4:('nw',-1),
                  5:('se',1),
                  6:('sw',-1),
                  7:('ne',1)}

def add_label(element, diagram, parent):
    # Is there a label associated with point?
    text = element.text

    # is there a label here?
    has_text = text is not None and len(text.strip()) > 0
    all_comments = all([subel.tag is ET.Comment for subel in element])
    if has_text or not all_comments:
        # If there's a label, we'll bundle the label and vector in a group
        group = ET.SubElement(parent, 'g')
        diagram.add_id(group, element.get('id'))

        # Now we'll create a new XML element describing the label
        el = copy.deepcopy(element)
        el.tag = 'label'

        # determine alignment of the label
        alignment = element.get('alignment', None)
        user_offset = element.get('offset', None)
        angle = diagram.get_source_data(element, 'angle')
        if alignment is None:
            angle_degrees = math.degrees(-angle)
            while angle_degrees < 0:
                angle_degrees += 360
            half_quadrant = math.floor(angle_degrees / 45)
            alignment, offset_dir = alignment_dict[half_quadrant]
            el.set('alignment', alignment)

            normal = (math.cos(-angle), math.sin(-angle))
            direction = offset_dir * np.array((-normal[1], normal[0]))
            offset = 4 * direction
            if user_offset is not None:
                offset += un.valid_eval(user_offset)
            el.set('abs-offset', util.np2str(offset))
        else:
            displacement = label.alignment_displacement[alignment]
            def_offset = np.array([4*(displacement[0]+0.5),
                                   4*(displacement[1]-0.5)])
            if user_offset is not None:
                def_offset += un.valid_eval(user_offset)
            el.set('offset', util.np2str(def_offset))

        head = diagram.get_source_data(element, 'head')
        el.set('anchor', util.pt2long_str(head, spacer=','))

        # add the label graphical element to the group
        label.label(el, diagram, group)
        return group
    else:
        return parent
