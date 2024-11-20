## Add a graphical element describing a polygon

import lxml.etree as ET
import numpy as np
import copy
import logging
from . import user_namespace as un
from . import utilities as util
from . import math_utilities as math_util
from . import arrow
from . import point
from . import label
from . import circle
from . import group

log = logging.getLogger('prefigure')

# Process a polygon tag into a graphical component
def polygon(element, diagram, parent, outline_status):
    if outline_status == 'finish_outline':
        finish_outline(element, diagram, parent)
        return

    if diagram.output_format() == 'tactile':
        if element.get('stroke') is not None:
            element.set('stroke', 'black')
        if element.get('fill') is not None:
            element.set('fill', 'lightgray')
    util.set_attr(element, 'stroke', 'none')
    util.set_attr(element, 'fill', 'none')
    util.set_attr(element, 'thickness', '2')

    # We allow the vertices to be generated programmatically
    parameter = element.get('parameter')
    points = element.get('points')
    if parameter is None:
        try:
            points = un.valid_eval(points)
        except:
            log.error(f"Error in <polygon> evaluating points={element.get('points')}")
            return
    else:
        try:
            var, expr = parameter.split('=')
            param_0, param_1 = map(un.valid_eval, expr.split('..'))
            plot_points = []
            for k in range(param_0, param_1+1):
                un.valid_eval(str(k), var)
                plot_points.append(un.valid_eval(points))
            points = plot_points
        except:
            log.error(f"Error in <polygon> generating points")
            return

    points = [diagram.transform(point) for point in points]

    radius = int(element.get('corner-radius', '0'))
    closed = element.get('closed', 'no')
    # Form an SVG path now that we have the vertices
    if radius == 0:
        p = points[0]
        d = ['M ' + util.pt2str(p)]
        for p in points[1:]:
            d.append('L ' + util.pt2str(p))
        if closed == 'yes':
            d.append('Z')
        d = ' '.join(d)
    else:
        if closed == 'yes':
            points.append(points[0])
        N = len(points) - 1  # number of segments
        cmds = ''
        for i, endpoints in enumerate(zip(points[:-1], points[1:])):
            p, q = endpoints
            u = math_util.normalize(q-p)
            p1 = p + radius*u
            p2 = q - radius*u
            if i == 0:
                if closed == 'yes':
                    cmds = 'M ' + util.pt2str(p1)
                    initial_point = p1
                    cmds += 'L ' + util.pt2str(p2)
                else:
                    cmds += 'M ' + util.pt2str(p)
                    cmds += 'L ' + util.pt2str(p2)
            if i == N - 1:
                cmds += 'Q ' + util.pt2str(p)
                cmds += ' ' + util.pt2str(p1)
                if closed == 'yes':
                    cmds += 'L ' + util.pt2str(p2)
                    cmds += 'Q ' + util.pt2str(q)
                    cmds += ' ' + util.pt2str(initial_point)
                    cmds += 'Z'
                else:
                    cmds += 'L' + util.pt2str(q)
            if i > 0 and i < N - 1:
                cmds += 'Q' + util.pt2str(p)
                cmds += ' ' + util.pt2str(p1)
                cmds += 'L' + util.pt2str(p2)
            
        d = cmds
    path = ET.Element('path')
    diagram.add_id(path, element.get('id'))
    path.set('d', d)
    util.add_attr(path, util.get_2d_attr(element))
#    path.set('type', 'polygon')
    element.set('cliptobbox', element.get('cliptobbox', 'yes'))
    util.cliptobbox(path, element, diagram)

    arrows = int(element.get('arrows', '0'))
    forward = 'marker-end'
    backward = 'marker-start'
    if element.get('reverse', 'no') == 'yes':
        forward, backward = backward, forward
    if arrows > 0:
        arrow.add_arrowhead_to_path(
            diagram,
            forward,
            path,
            arrow_width=element.get('arrow-width', None),
            arrow_angles=element.get('arrow-angles', None)
        )
    if arrows > 1:
        arrow.add_arrowhead_to_path(
            diagram,
            backward,
            path,
            arrow_width=element.get('arrow-width', None),
            arrow_angles=element.get('arrow-angles', None)
        )

    if outline_status == 'add_outline':
        diagram.add_outline(element, path, parent)
        return

    if element.get('outline', 'no') == 'yes' or diagram.output_format() == 'tactile':
        diagram.add_outline(element, path, parent)
        finish_outline(element, diagram, parent)

    else:
        parent.append(path)

def triangle(element, diagram, parent, outline_status):
    '''
    if outline_status == 'finish_outline':
        polygon(element, diagram, parent, outline_status)
        for child in element:
            if child.tag == 'point':
                point.point(element, diagram, parent, outline_status)
            if child.tag == 'label':
                label.label(element, diagram, parent, outline_status)
        return
    '''

    try:
        vertices = un.valid_eval(element.get('vertices'))
    except:
        log.error(f"Error in <triangle> evaluating vertices={element.get('vertices')}")
        return
    if len(vertices) != 3:
        log.error('A <triangle> should have exactly 3 vertices')
        return

    # We're going to turn this into a group since we may be adding
    # other components.  Plus, we want to allow appropriate outlining
    # of tactile versions
    element_cp = copy.deepcopy(element)
    element.tag = 'group'
    element.set('outline', 'tactile')

    element_cp.tag = 'polygon'
    element_cp.set('closed', 'yes')
    element_cp.set('points', element_cp.get('vertices'))
    element_cp.set('stroke', element_cp.get('stroke', 'black'))
    element.append(element_cp)

    # add angle-markers
    if element_cp.get('angle-markers', 'no') == 'yes':
        u = vertices[1]-vertices[0]
        v = vertices[2]-vertices[1]
        if u[0]*v[1] - u[1]*v[0] > 0: # check the orientation
            verts = list(vertices)
            verts.reverse()
            vertices = np.array(verts)
        for _ in range(3):
            marker = ET.SubElement(element, 'angle-marker')
            points = ['('+util.pt2long_str(p, spacer=',')+')' for p in vertices]
            points = f"({','.join(points)})"
            marker.set('points', points)
            vertices = math_util.roll(vertices)

    labels = element_cp.get('labels', None)
    alignment_dict = {}
    if labels is not None:
        labels = [l.strip() for l in labels.split(',')]
        if len(labels) < 3:
            log.error(f"A triangle needs three labels: {element.get('labels')}")
            return
        vertices = list(vertices)
        vertices += vertices[:2]
        vertices = np.array(vertices)
        for i in range(1,4):
            u = vertices[i-1] - vertices[i]
            v = vertices[i+1] - vertices[i]
            direction = -(u+v)
            alignment = label.get_alignment_from_direction(direction)
            alignment_dict[i % 3] = alignment
            
    if element_cp.get('show-vertices', 'no') == 'yes':
        for i in range(3):
            point_el = ET.SubElement(element, 'point')
            point_el.set('p', util.pt2long_str(vertices[i], spacer=','))
            fill = element_cp.get('point-fill', None)
            if fill is not None:
                point_el.set('fill', fill)
            if alignment_dict.get(i, None) is not None:
                m_tag = ET.SubElement(point_el, 'm')
                m_tag.text = labels[i]
                point_el.set('alignment', alignment_dict[i])
    else:
        if labels is not None:
            for i in range(3):
                label_el = ET.SubElement(element, 'label')
                label_el.set('anchor', util.pt2long_str(vertices[i],
                                                        spacer=','))
                label_el.set('alignment', alignment_dict[i])
                m_tag = ET.SubElement(label_el, 'm')
                m_tag.text = labels[i]

    group.group(element, diagram, parent, outline_status)

def finish_outline(element, diagram, parent):
    diagram.finish_outline(element,
                           element.get('stroke'),
                           element.get('thickness'),
                           element.get('fill', 'none'),
                           parent)
