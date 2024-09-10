## Add a graphical element describing a polygon

import lxml.etree as ET
from . import user_namespace as un
from . import utilities as util
from . import math_utilities as math_util
from . import arrow

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
        points = un.valid_eval(points)
    else:
        var, expr = parameter.split('=')
        param_0, param_1 = map(un.valid_eval, expr.split('..'))
        plot_points = []
        for k in range(param_0, param_1+1):
            un.valid_eval(str(k), var)
            plot_points.append(un.valid_eval(points))
        points = plot_points

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
    path.set('type', 'polygon')
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

def finish_outline(element, diagram, parent):
    diagram.finish_outline(element,
                           element.get('stroke'),
                           element.get('thickness'),
                           element.get('fill', 'none'),
                           parent)
