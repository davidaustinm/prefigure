import lxml.etree as ET
import numpy as np
import math
import logging
from . import utilities as util
from . import math_utilities as math_util
from . import user_namespace as un
import copy
from . import arrow
from . import group
from . import label
from . import CTM

log = logging.getLogger('prefigure')

# Process a line XML element into an SVG line element
def line(element, diagram, parent, outline_status):
    if outline_status == 'finish_outline':
        finish_outline(element, diagram, parent)
        return

    endpts = element.get('endpoints', None)
    if endpts is None:
        try:
            p1 = un.valid_eval(element.get('p1'))
        except:
            log.error(f"Error in <line> parsing p1={element.get('p1')}")
            return
        try:
            p2 = un.valid_eval(element.get('p2'))
        except:
            log.error(f"Error in <line> parsing p2={element.get('p2')}")
            return
    else:
        try:
            p1, p2 = un.valid_eval(endpts)
        except:
            log.error(f"Error in <line> parsing endpoints={element.get('endpoints')}")
            return

    endpoint_offsets = None
    if element.get('infinite', 'no') == 'yes':
        p1, p2 = infinite_line(p1, p2, diagram)
        if p1 is None:  # the line doesn't hit the bounding box
            return
    else:
        endpoint_offsets = element.get('endpoint-offsets', None)
        if endpoint_offsets is not None:
            try:
                endpoint_offsets = un.valid_eval(endpoint_offsets)
            except:
                log.error(f"Error in <line> parsing endpoint-offsets={element.get('endpoint-offsets')}")
                return

    line = mk_line(p1, p2, diagram, element.get('id', None), 
                   endpoint_offsets=endpoint_offsets)

    # we need to hold on to the endpoints in case the line is labelled
    # these are endpoints in SVG coordinates
    x1 = float(line.get('x1'))
    x2 = float(line.get('x2'))
    y1 = float(line.get('y1'))
    y2 = float(line.get('y2'))

    q1 = np.array((x1, y1))
    q2 = np.array((x2, y2))
    diagram.save_data(element, {'q1': q1, 'q2': q2})

    # now add the graphical attributes
    util.set_attr(element, 'stroke', 'black')
    util.set_attr(element, 'thickness', '2')
    if diagram.output_format() == 'tactile':
        element.set('stroke', 'black')
    util.add_attr(line, util.get_1d_attr(element))

    arrows = int(element.get('arrows', '0'))
    forward = 'marker-end'
    backward = 'marker-start'
    if element.get('reverse', 'no') == 'yes':
        forward, backward = backward, forward
    if arrows > 0:
        arrow.add_arrowhead_to_path(
            diagram,
            forward,
            line,
            arrow_width=element.get('arrow-width', None),
            arrow_angles=element.get('arrow-angles', None)
        )
    if arrows > 1:
        arrow.add_arrowhead_to_path(
            diagram,
            backward,
            line,
            arrow_width=element.get('arrow-width', None),
            arrow_angles=element.get('arrow-angles', None)
        )

    if element.get('additional-arrows', None) is not None:
        additional = un.valid_eval(element.get('additional-arrows'))
        if not isinstance(additional, np.ndarray):
            additional = np.array([additional])
        list_additional = list(additional)
        list_additional.sort()

        line.tag = "path"
        cmds = ['M', line.get('x1'), line.get('y1')]
        p1 = np.array([float(line.get('x1')), float(line.get('y1'))])
        p2 = np.array([float(line.get('x2')), float(line.get('y2'))])
        for additional in list_additional:
            p = (1-additional)*p1 + additional*p2
            cmds += ['L', util.pt2str(p)]
        cmds += ['L', line.get('x2'), line.get('y2')]
        line.set('d', ' '.join(cmds))
        arrow.add_arrowhead_to_path(
            diagram,
            'marker-mid',
            line,
            arrow_width=element.get('arrow-width', None),
            arrow_angles=element.get('arrow-angles', None)
        )
        
    util.cliptobbox(line, element, diagram)

    if outline_status == 'add_outline':
        diagram.add_outline(element, line, parent)
        return

    if element.get('outline', 'no') == 'yes' or diagram.output_format() == 'tactile':
        diagram.add_outline(element, line, parent)
        finish_outline(element, diagram, parent)
    else:
        original_parent = parent
        parent = add_label(element, diagram, parent)
        parent.append(line)

        # if no label has been added, then we're done
        if original_parent == parent:
            return

        # if there is a label, then the id is on the outer <g> element
        # so we need to remove it from the children
        remove_id(parent)

def finish_outline(element, diagram, parent):
    original_parent = parent
    parent = add_label(element, diagram, parent)

    # if we've added a label, remove the id's from element under the parent <g>
    if original_parent != parent:
        remove_id(parent)

    diagram.finish_outline(element,
                           element.get('stroke'),
                           element.get('thickness'),
                           element.get('fill', 'none'),
                           parent)

def remove_id(el):
    for child in el:
        if child.get('id', None) is not None:
            child.attrib.pop('id')
        remove_id(child)

# We'll be adding lines in other places so we'll use this more widely
def mk_line(p0, p1, diagram, id = None, endpoint_offsets = None, user_coords = True):
    line = ET.Element('line')
    diagram.add_id(line, id)
    if user_coords:
        p0 = diagram.transform(p0)
        p1 = diagram.transform(p1)
    if endpoint_offsets is not None:
        if len(endpoint_offsets.shape) == 1:
            u = math_util.normalize(p1-p0)
            p0 = p0 + endpoint_offsets[0] * u
            p1 = p1 + endpoint_offsets[1] * u
        else:
            p0[0] += endpoint_offsets[0][0]
            p0[1] -= endpoint_offsets[0][1]
            p1[0] += endpoint_offsets[1][0]
            p1[1] -= endpoint_offsets[1][1]

    line.set('x1', util.float2str(p0[0]))
    line.set('y1', util.float2str(p0[1]))
    line.set('x2', util.float2str(p1[0]))
    line.set('y2', util.float2str(p1[1]))
    return line

# if a line is "infinite," find the points where it intersects the bounding box
def infinite_line(p0, p1, diagram, slope = None):
    ctm, bbox = diagram.ctm_stack[-1]
    p0 = np.array(p0)
    p1 = np.array(p1)
    if slope is not None:
        p = p0
        v = np.array([1, slope])
    else:
        p = p0
        v = p1 - p0
    t_max = math.inf
    t_min = -math.inf
    if v[0] != 0:
        t0 = (bbox[0]-p[0])/v[0]
        t1 = (bbox[2]-p[0])/v[0]
        if t0 > t1:
            t0, t1 = t1, t0
        t_max = min(t1, t_max)
        t_min = max(t0, t_min)
    if v[1] != 0:
        t0 = (bbox[1]-p[1])/v[1]
        t1 = (bbox[3]-p[1])/v[1]
        if t0 > t1:
            t0, t1 = t1, t0
        t_max = min(t1, t_max)
        t_min = max(t0, t_min)
    if t_min > t_max:
        return None, None
    return [p + t * v for t in [t_min, t_max]]

def add_label(element, diagram, parent):
    # Is there a label associated with point?
    text = element.text

    # is there a label here?
    has_text = text is not None and len(text.strip()) > 0
    all_comments = all([subel.tag is ET.Comment for subel in element])
    if has_text or not all_comments:
        # If there's a label, we'll bundle the label and point in a group
        parent_group = ET.SubElement(parent, 'g')
        diagram.add_id(parent_group, element.get('id'))

        # Now we'll create a new XML element describing the label
        el = copy.deepcopy(element)
        el.tag = 'label'

        data = diagram.retrieve_data(element)
        q1 = data['q1']
        q2 = data['q2']

        label_location = un.valid_eval(element.get("label-location", "0.5"))
        if label_location < 0:
            label_location = -label_location
            q1, q2 = q2, q1

        el.set('user-coords', 'no')
        diff = q2 - q1
        d = math_util.length(diff)
        angle = math.degrees(math.atan2(diff[1], diff[0]))
        if diagram.output_format() == "tactile":
            anchor = q1 + label_location * diff
            el.set("anchor", f"({anchor[0]}, {anchor[1]})")
            direction = (diff[1], diff[0])
            alignment = label.get_alignment_from_direction(direction)
            el.set("alignment", alignment)
            label.label(el, diagram, parent_group)
        else:
            tform = CTM.translatestr(*q1)
            tform += ' ' + CTM.rotatestr(-angle)
            distance = d * label_location
            g = ET.SubElement(parent_group, "g")
            g.set("transform", tform)
            el.set("anchor", f"({distance},0)")
            el.set("alignment", "north")
            label.label(el, diagram, g)

        return parent_group

    else:
        return parent

