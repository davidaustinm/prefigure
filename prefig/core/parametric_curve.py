## Add a graphical element describing a parametric curve

import lxml.etree as ET
import logging
from . import user_namespace as un
from . import utilities as util
from . import math_utilities as math_util
from . import arrow

log = logging.getLogger('prefigure')
separation_tolerance = 5

def parametric_curve(element, diagram, parent, outline_group):

    try:
        f = un.valid_eval(element.get('function'))
    except:
        log.error(f"Error in <parametric-curve> defining function={element.get('function')}")
        return
    try:
        domain = un.valid_eval(element.get('domain'))
    except:
        log.error(f"Error in <parametric-curve> defining domain={element.get('domain')}")
        return

    arrows = int(element.get('arrows', '0'))

    N = int(element.get('N', '100'))
    t = domain[0]
    dt = (domain[1]-domain[0])/N
    p = diagram.transform(f(t))
    points = ['M ' + util.pt2str(p)]
    for _ in range(N):
        points += take_step(diagram, f, t, dt)
        t += dt

    if element.get('closed', 'no') == 'yes':
        points.append('Z')

    if arrows > 0 and element.get('arrow-location', None) is not None:
        arrow_location = un.valid_eval(element.get('arrow-location'))
        num_pts = 5
        t = arrow_location - num_pts*dt
        p = diagram.transform(f(t))
        points.append('M ' + util.pt2str(p))
        for _ in range(num_pts):
            t += dt
            p = diagram.transform(f(t))
            points.append('L ' + util.pt2str(p))

    d = ' '.join(points)

    if diagram.output_format() == 'tactile':
        element.set('stroke', 'black')
        util.set_tactile_fill(element)
    else:
        util.set_attr(element, 'stroke', 'blue')
        util.set_attr(element, 'fill', 'none')
    util.set_attr(element, 'thickness', '2')

    path = ET.Element('path')
    diagram.add_id(path, element.get('id'))
    diagram.register_svg_element(element, path)

    path.set('d', d)
    util.add_attr(path, util.get_2d_attr(element))

    element.set('cliptobbox', element.get('cliptobbox', 'yes'))
    util.cliptobbox(path, element, diagram)

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

    if outline_group is not None:
        diagram.add_outline(element, path, outline_group)
        finish_outline(element, diagram, parent)
    elif (element.get('outline', 'no') == 'yes'
          or diagram.output_format() == 'tactile'):
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

def take_step(diagram, f, t0, dt):
    last_p = diagram.transform(f(t0))
    t1 = t0 + dt
    p = diagram.transform(f(t1))
    if math_util.length(p - last_p) < separation_tolerance:
        return ['L ' + util.pt2str(p)]
    dt /= 2
    return take_step(diagram, f, t0, dt) + take_step(diagram, f, t0+dt, dt)
