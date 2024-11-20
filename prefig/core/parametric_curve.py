## Add a graphical element describing a parametric curve

import lxml.etree as ET
import logging
from . import user_namespace as un
from . import utilities as util
from . import arrow

log = logging.getLogger('prefigure')

def parametric_curve(element, diagram, parent, outline_status):
    if outline_status == 'finish_outline':
        finish_outline(element, diagram, parent)
        return

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

    N = int(element.get('N', '100'))

    t = domain[0]
    dt = (domain[1]-domain[0])/N
    p = diagram.transform(f(t))
    points = ['M ' + util.pt2str(p)]
    for _ in range(N):
        t += dt
        p = diagram.transform(f(t))
        points.append('L ' + util.pt2str(p))
    if element.get('closed', 'no') == 'yes':
        points.append('Z')
    d = ' '.join(points)

    if diagram.output_format() == 'tactile':
        element.set('stroke', 'black')
        if element.get('fill') is not None:
            element.set('fill', 'lightgray')
    else:
        util.set_attr(element, 'stroke', 'blue')
        util.set_attr(element, 'fill', 'none')
    util.set_attr(element, 'thickness', '2')

    path = ET.Element('path')
    diagram.add_id(path, element.get('id'))
    path.set('d', d)
    util.add_attr(path, util.get_2d_attr(element))
#    path.set('type', 'parametric curve')

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
