import logging
import lxml.etree as ET
import numpy as np
from . import user_namespace as un
from . import utilities as util
from . import calculus
from . import line

log = logging.getLogger('prefigure')

# Add a graphical element representing the tangent line to a graph
def tangent(element, diagram, parent, outline_status):
    if outline_status == 'finish_outline':
        finish_outline(element, diagram, parent)
        return

    # set up the linear function describing the tangent line
    try:
        function = un.valid_eval(element.get('function'))
    except:
        log.error(f"Error retrieving tangent-line attribute @function={element.get('function')}")
        return

    try:
        a = un.valid_eval(element.get('point'))
    except:
        log.error(f"Error parsing tangent-line attribute @point={element.get('point')}")
        return

    y0 = function(a)
    m = calculus.derivative(function, a)

    def tangent(x):
        return y0 + m*(x-a)

    name = element.get('name', None)
    if name is not None:
        un.enter_namespace(name, tangent)

    # determine the interval over which we'll draw the tangent line
    bbox = diagram.bbox()
    domain = element.get('domain', None)
    if domain is None:
        domain = [bbox[0], bbox[2]]
    else: 
        domain = un.valid_eval(domain)

    scales = diagram.get_scales()
    x1, x2 = domain
    if scales[0] == 'linear' and scales[1] == 'linear':
        # find the endpoints of the tangent line
        y1 = tangent(x1)
        y2 = tangent(x2)
        p1 = (x1, y1)
        p2 = (x2, y2)
        if element.get('infinite') == 'yes' or element.get('domain') is None:
            p1, p2 = line.infinite_line(p1, p2, diagram)
        if p1 is None:
            return

        # construct the graphical line element from those points and attributes
        line_el = line.mk_line(p1, p2, diagram, element.get('id'))

    else:
        line_el = ET.Element('path')
        if scales[0] == 'log':
            x_positions = np.logspace(np.log10(x1), np.log(x2), 101)
        else:
            x_positions = np.linspace(x1, x2, 101)
        cmds = []
        next_cmd = 'M'
        for x in x_positions:
            y = tangent(x)
            if y < 0 and scales[1] == 'log':
                next_cmd = 'M'
                continue
            cmds += [next_cmd, util.pt2str(diagram.transform((x, y)))]
            next_cmd = 'L'
        line_el.set('d', ' '.join(cmds))

    if diagram.output_format() == 'tactile':
        element.set('stroke', 'black')
    else:
        util.set_attr(element, 'stroke', 'red')
    util.set_attr(element, 'thickness', '2')

    util.add_attr(line_el, util.get_1d_attr(element))
    element.set('cliptobbox', 'yes')
    util.cliptobbox(line_el, element, diagram)

    if outline_status == 'add_outline':
        diagram.add_outline(element, line_el, parent)
        return

    if element.get('outline', 'no') == 'yes' or diagram.output_format() == 'tactile':
        diagram.add_outline(element, line_el, parent)
        finish_outline(element, diagram, parent)
    else:
        parent.append(line_el)

def finish_outline(element, diagram, parent):
    diagram.finish_outline(element,
                           element.get('stroke'),
                           element.get('thickness'),
                           element.get('fill', 'none'),
                           parent)
