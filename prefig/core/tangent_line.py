import logging
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

    # determine the interval over which we'll draw the tangent line
    bbox = diagram.bbox()
    domain = element.get('domain', None)
    if domain is None:
        domain = [bbox[0], bbox[2]]
    else: 
        domain = un.valid_eval(domain)

    # find the endpoints of the tangent line
    x1, x2 = domain
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

    if diagram.output_format() == 'tactile':
        element.set('stroke', 'black')
    else:
        util.set_attr(element, 'stroke', 'red')
    util.set_attr(element, 'thickness', '2')

    util.add_attr(line_el, util.get_1d_attr(element))
#    line_el.set('type', 'tangent line')
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
