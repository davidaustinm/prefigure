import lxml.etree as ET
import math
import numpy as np
import copy
from . import user_namespace as un
from . import utilities as util
from . import math_utilities as math_util
from . import CTM
from . import arrow
from . import label

# Add graphical elements related to circles
def circle(element, diagram, parent, outline_status):
    if outline_status == 'finish_outline':
        finish_outline(element, diagram, parent)
        return

    center = un.valid_eval(element.get('center'))
    radius = un.valid_eval(element.get('radius', '1'))

    # We could use an SVG ellipse, but we're going to use a path for
    # unions and intersections
    circle = ET.Element('path')
    diagram.add_id(circle, element.get('id'))

    N = un.valid_eval(element.get('N', '100'))
    cmds = make_path(diagram,
                     center,
                     (radius, radius),
                     (0,360),
                     N=N)
    cmds.append('Z')
    circle.set('d', ' '.join(cmds))

    if diagram.output_format() == 'tactile':
        if element.get('stroke') is not None:
            element.set('stroke', 'black')
        if element.get('fill') is not None:
            element.set('fill', 'lightgray')
    else:
        element.set('stroke', element.get('stroke', 'none'))
        element.set('fill', element.get('fill', 'none'))
    element.set('thickness', element.get('thickness', '2'))
    util.add_attr(circle, util.get_2d_attr(element))
#    circle.set('type', 'circle')
    util.cliptobbox(circle, element, diagram)

    if outline_status == 'add_outline':
        diagram.add_outline(element, circle, parent)
        return

    if element.get('outline', 'no') == 'yes' or diagram.output_format() == 'tactile':
        diagram.add_outline(element, circle, parent)
        finish_outline(element, diagram, parent)
    else:
        parent.append(circle)

def finish_outline(element, diagram, parent):
    parent = add_label(element, diagram, parent)
    for child in parent:
        if child.get('id', None) is not None:
            child.attrib.pop('id')

    diagram.finish_outline(element,
                           element.get('stroke'),
                           element.get('thickness'),
                           element.get('fill'),
                           parent)    

def ellipse(element, diagram, parent, outline_status):
    if outline_status == 'finish_outline':
        finish_outline(element, diagram, parent)
        return

    center = un.valid_eval(element.get('center'))
    axes_length = un.valid_eval(element.get('axes', '(1,1)'))
    rotate = un.valid_eval(element.get('rotate', '0'))
    if element.get('degrees', 'yes') == 'no':
        rotate = math.degrees(rotate)

    N = un.valid_eval(element.get('N', '100'))
    circle = ET.Element('path')
    diagram.add_id(circle, element.get('id'))
    cmds = make_path(diagram,
                     center,
                     axes_length,
                     (0, 360),
                     rotate=rotate,
                     N=N)

    cmds.append('Z')
    circle.set('d', ' '.join(cmds))

    if diagram.output_format() == 'tactile':
        if element.get('stroke') is not None:
            element.set('stroke', 'black')
        if element.get('fill') is not None:
            element.set('fill', 'lightgray')
    else:
        element.set('stroke', element.get('stroke', 'none'))
        element.set('fill', element.get('fill', 'none'))
    element.set('thickness', element.get('thickness', '2'))
    util.add_attr(circle, util.get_2d_attr(element))
#    circle.set('type', 'ellipse')
    util.cliptobbox(circle, element, diagram)

    if outline_status == 'add_outline':
        diagram.add_outline(element, circle, parent)
        return

    if element.get('outline', 'no') == 'yes' or diagram.output_format() == 'tactile':
        diagram.add_outline(element, circle, parent)
        finish_outline(element, diagram, parent)
    else:
        parent.append(circle)

def arc(element, diagram, parent, outline_status):
    if outline_status == 'finish_outline':
        finish_outline(element, diagram, parent)
        return

    if diagram.output_format() == 'tactile':
        if element.get('stroke') is not None:
            element.set('stroke', 'black')
        if element.get('fill', 'none') != 'none':
            element.set('fill', 'lightgray')
    else:
        element.set('stroke', element.get('stroke', 'none'))
        element.set('fill', element.get('fill', 'none'))
    element.set('thickness', element.get('thickness','2'))

    if element.get('points', None) is not None:
        points = element.get('points')
        points = un.valid_eval(points)
        center = points[1]
        v = points[0] - points[1]
        u = points[2] - points[1]
        start = math.degrees(math.atan2(v[1], v[0]))
        stop = math.degrees(math.atan2(u[1], u[0]))
        if stop < start:
            stop += 360
        angular_range = (start, stop)
        element.set('degrees', 'yes')
    else:
        center = un.valid_eval(element.get('center'))
        angular_range = un.valid_eval(element.get('range'))
    radius = un.valid_eval(element.get('radius'))
    sector = element.get('sector', 'no') == 'yes'

    if element.get('degrees', 'yes') == 'no':
        angular_range = [math.degrees(a) for a in angular_range]

    N = un.valid_eval(element.get('N', '100'))

    arc = ET.Element('path')
    diagram.add_id(arc, element.get('id'))

    cmds = make_path(diagram,
                     center,
                     (radius, radius),
                     angular_range,
                     N=N)

    if sector:
        cmds += ['L', util.pt2str(diagram.transform(center)), 'Z']

    arc.set('d', ' '.join(cmds))
    util.add_attr(arc, util.get_2d_attr(element))
#    arc.set('type', 'arc')
    util.cliptobbox(arc, element, diagram)

    arrows = int(element.get('arrows', '0'))
    forward = 'marker-end'
    backward = 'marker-start'
    if element.get('reverse', 'no') == 'yes':
        forward, backward = backward, forward
    
    if arrows > 0:
        arrow.add_arrowhead_to_path(
            diagram,
            forward,
            arc,
            arrow_width=element.get('arrow-width', None),
            arrow_angles=element.get('arrow-angles', None)
        )
    if arrows > 1:
        arrow.add_arrowhead_to_path(
            diagram,
            backward,
            arc,
            arrow_width=element.get('arrow-width', None),
            arrow_angles=element.get('arrow-angles', None)
        )

    if outline_status == 'add_outline':
        diagram.add_outline(element, arc, parent, outline_width=2)
        return

    if element.get('outline', 'no') == 'yes' or diagram.output_format() == 'tactile':
        diagram.add_outline(element, arc, parent, outline_width=4)
        finish_outline(element, diagram, parent)
    else:
        parent.append(arc)

def make_path(diagram,
              center,
              axes_length,
              angular_range,
              rotate = 0,
              N=100):
    ctm = CTM.CTM()
    ctm.translate(*center)
    ctm.rotate(rotate)
    ctm.scale(*axes_length)
    angular_range = [math.radians(angle) for angle in angular_range]
    t = angular_range[0]
    dt = (angular_range[1]-angular_range[0])/N
    cmds = []
    for _ in range(N):
        point = ctm.transform((math.cos(t), math.sin(t)))
        point = diagram.transform(point)
        point
        command = 'L'
        if len(cmds) == 0:
            command = 'M'
        cmds += [command, util.pt2str(point)]
        t += dt
    return cmds

# Alexei's angle marker
def angle(element, diagram, parent, outline_status):
    if outline_status == 'finish_outline':
        finish_outline(element, diagram, parent)
        return

    element.set('stroke', element.get('stroke', 'black'))
    if diagram.output_format() == 'tactile':
        if element.get('fill') is not None:
            element.set('fill', 'lightgray')
    else:
        element.set('fill', element.get('fill', 'none'))
    element.set('thickness', element.get('thickness','2'))

    points = element.get('points', None)
    if points is None:
        p = un.valid_eval(element.get('p'))
        p1 = un.valid_eval(element.get('p1'))
        p2 = un.valid_eval(element.get('p2'))
    else:
        points = un.valid_eval(points)
        p = points[1]
        p1 = points[0]
        p2 = points[2]
    radius = un.valid_eval(element.get('radius','30'))

    # is this a right angle
    u = math_util.normalize(p1 - p)
    v = math_util.normalize(p2 - p)
    right = abs(math_util.dot(u, v)) < 0.001

    # convert to svg coordinates

    p = diagram.transform(p)
    p1 = diagram.transform(p1)
    p2 = diagram.transform(p2)

    # Define vectors from p to p1 and p2, normalized
    v1 = math_util.normalize(p1 - p)
    v2 = math_util.normalize(p2 - p)

    # To determine the orientation, look at the z-component of cross product.
    # Keep in mind that y-axis in svg is directed down. large_arc_flag is 0 if the 
    # arc is supposed to be small

    large_arc_flag = int(v1[0]*v2[1] - v1[1]*v2[0] > 0)

    # It may make sense to have the default radius depend on the measure of the angle,
    # unless the user overrides it
    if large_arc_flag:
        angle = 2*np.pi - math.acos(np.dot(v1,v2))
    else:
        angle = math.acos(np.dot(v1,v2))

    # heuristically determined radius
    default_radius = int(27/angle)
    default_radius = min(30, default_radius)
    default_radius = max(15, default_radius)

    if diagram.output_format() == 'tactile':
        default_radius *= 1.5

    radius = un.valid_eval(element.get('radius', str(default_radius)))

    if np.all(np.isclose(v1 + v2, np.zeros(2))):  # is the angle = 180?
        direction = np.array([v1[1], -v1[0]])
    else:
        direction = math_util.normalize(v1+v2)*(-1)**large_arc_flag
    label_location = p + direction*radius
    element.set('label-location', util.pt2str(label_location, spacer=','))
    if element.get('alignment', None) is None:
        element.set('alignment', 
                    label.get_alignment_from_direction([direction[0], -direction[1]]))
    else:
        if element.get('alignment').strip() == 'e':
            element.set('alignment', 'east')
    initial_point = v1*radius + p
    final_point = v2*radius + p
    initial_point_str = util.pt2str(initial_point)
    final_point_str = util.pt2str(final_point)

    d = 'M ' + initial_point_str
    d += ' A ' + util.pt2str((radius, radius)) + ' 0 '
    d += str(large_arc_flag) + ' 0 ' + final_point_str

    # is this a right angle?
    if right and math.degrees(angle) < 180:
        ctm = CTM.CTM()
        ctm.translate(*p)
        '''
        angle = math.atan2(v1[1],v1[0])
        ctm.rotate(angle, units="rad")
        d = 'M ' + util.pt2str(ctm.transform((radius,0)))
        d += ' L ' + util.pt2str(ctm.transform((radius, -radius)))
        d += ' L ' + util.pt2str(ctm.transform((0, -radius)))
        '''
        d = 'M ' + util.pt2str(ctm.transform(radius*v1))
        d += ' L ' + util.pt2str(ctm.transform(radius*(v1+v2)))
        d += ' L ' + util.pt2str(ctm.transform(radius*v2))
    arc = ET.Element('path')
    diagram.add_id(arc, element.get('id'))
    arc.set('d', d)

    util.add_attr(arc, util.get_1d_attr(element))
#    arc.set('type', 'arc')
    util.cliptobbox(arc, element, diagram)

    if element.get('arrow', None) is not None:
        if element.get('reverse', 'no') == 'yes':
            arrow.add_arrowhead_to_path(
                diagram,
                'marker-end',
                arc,
                arrow_width=element.get('arrow-width', None),
                arrow_angles=element.get('arrow-angles', None)
            )
        else:
            arrow.add_arrowhead_to_path(
                diagram,
                'marker-start',
                arc,
                arrow_width=element.get('arrow-width', None),
                arrow_angles=element.get('arrow-angles', None)
            )
    if outline_status == 'add_outline':
        diagram.add_outline(element, arc, parent, outline_width=2)
        return

    if element.get('outline', 'no') == 'yes' or diagram.output_format() == 'tactile':
        diagram.add_outline(element, arc, parent, outline_width=4)
        finish_outline(element, diagram, parent)
    else:
        parent = add_label(element, diagram, parent)
        if element.get('id', 'none') == parent.get('id'):
            element.attrib.pop('id')
        parent.append(arc)
        for child in parent:
            if child.get('id', None) is not None:
                child.attrib.pop('id')

def add_label(element, diagram, parent):
    # Is there a label associated with the marker?
    text = element.text

    # is there a label here?
    has_text = text is not None and len(text.strip()) > 0
    all_comments = all([subel.tag is ET.Comment for subel in element])
    if has_text or not all_comments:
        # If there's a label, we'll bundle the label and the angle mark in a group
        group = ET.SubElement(parent, 'g')
        diagram.add_id(group, element.get('id'))
#        group.set('type', 'labeled-angle-marker')

        # Now we'll create a new XML element describing the label
        el = copy.deepcopy(element)
        el.tag = 'label'
        el.set('alignment', element.get('alignment'))
        el.set('p', element.get('label-location'))
        el.set('user-coords', 'no')
        if element.get('offset', None) is not None:
            el.set('offset', element.get('offset'))

        # add the label graphical element to the group
        label.label(el, diagram, group)
        return group
    else:
        return parent

