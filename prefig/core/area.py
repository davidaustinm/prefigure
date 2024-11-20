# Supports graphical elements describing regions in the plane

import lxml.etree as ET
import logging
from . import user_namespace as un
from . import utilities as util

log = logging.getLogger('prefigure')

# Area under a graph and between two graphs
def area_between_curves(element, diagram, parent, outline_status):
    if outline_status == 'finish_outline':
        finish_outline(element, diagram, parent)
        return

    util.set_attr(element, 'stroke', 'black')
    util.set_attr(element, 'fill', 'lightgray')
    util.set_attr(element, 'thickness', '2')
    if diagram.output_format() == 'tactile':
        element.set('stroke', 'black')
        element.set('fill', 'lightgray')

    # Retrieve the two functions and construct the area
    functions = element.get('functions')
    if functions is not None:
        try:
            f, g = list(un.valid_eval(element.get('functions')))
        except:
            log.error(f"Error in <area> parsing functions={element.get('functions')}")
            return
    else:
        try:
            f = un.valid_eval(element.get('function1'))
        except:
            log.error(f"Error in <area> defining function1={element.get('function1')}")
            return
        try:
            g = un.valid_eval(element.get('function2'))
        except:
            log.error(f"Error in <area> defining function2={element.get('function2')}")
            return

    N = int(element.get('N', '100'))

    try:
        domain = element.get('domain')
    except:
        log.error(f"Error in <area> parsing domain={element.get('domain')}")
        return
    bbox = diagram.bbox()
    if domain is None:
        domain = [bbox[0], bbox[2]]
    else:
        domain = un.valid_eval(domain)

    dx = (domain[1]-domain[0])/N
    x = domain[0]
    p = diagram.transform((x, f(x)))
    cmds = ['M ' + util.pt2str(p)]  
    for _ in range(N+1):
        p = diagram.transform((x,f(x)))
        cmds.append('L ' + util.pt2str(p))
        x += dx
    for _ in range(N+1):
        x -= dx
        p = diagram.transform((x, g(x)))
        cmds.append('L ' + util.pt2str(p))
    cmds.append('Z')
    d = ' '.join(cmds)

    path = ET.Element('path')
    diagram.add_id(path, element.get('id'))
    path.set('d', d)
    util.add_attr(path, util.get_2d_attr(element))
#    path.set('type', 'area between curves')

    if outline_status == 'add_outline':
        diagram.add_outline(element, path, parent)
        return

    if element.get('outline', 'no') == 'yes' or diagram.output_format() == 'tactile':
        diagram.add_outline(element, path, parent)
        finish_outline(element, diagram, parent)
    else:
        parent.append(path)

def area_under_curve(element, diagram, parent, outline_status):
    element.set('function1', element.get('function', 'none'))
    un.define('__zero(x) = 0')
    element.set('function2', '__zero')
    area_between_curves(element, diagram, parent, outline_status)

def finish_outline(element, diagram, parent):
    diagram.finish_outline(element,
                           element.get('stroke'),
                           element.get('thickness'),
                           element.get('fill', 'none'),
                           parent)
