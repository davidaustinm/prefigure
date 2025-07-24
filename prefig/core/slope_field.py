import lxml.etree as ET
import numpy as np
import math
import copy
import logging
from . import user_namespace as un
from . import utilities
from . import grid_axes
from . import group
from . import math_utilities as math_util
from . import calculus

log = logging.getLogger('prefigure')

# Add a graphical element for slope fields
def slope_field(element, diagram, parent, outline_status):
    if outline_status == 'finish_outline':
        finish_outline(element, diagram, parent)
        return

    try:
        f = un.valid_eval(element.get('function'))
    except:
        log.error(f"Error retrieving slope-field function={element.get('function')}")
        return
    bbox = diagram.bbox()

    # We're going to turn this element into a group and add lines to it
    element.tag = "group"
    if element.get('outline', 'no') == 'yes':
        element.set('outline', 'always')

    # Now we'll construct a line with all the graphical information
    # and make copies of it
    line_template = ET.Element('line')

    if diagram.output_format() == 'tactile':
        line_template.set('stroke', 'black')
    else:
        line_template.set('stroke', element.get('stroke', 'blue'))
    line_template.set('thickness', element.get('thickness', '2'))
    if element.get('arrows', 'no') == 'yes':
        line_template.set('arrows', '1')

    if element.get('arrow-width', None) is not None:
        line_template.set('arrow-width', element.get('arrow-width'))
    if element.get('arrow-angles', None) is not None:
        line_template.set('arrow-angles', element.get('arrow-angles'))

    # Now we'll construct each of the lines in the slope field
    system = element.get('system', None) == 'yes'
    spacings = element.get('spacings', None)
    if spacings is not None:
        try:
            spacings = un.valid_eval(spacings)
            rx, ry = spacings
        except:
            log.error(f"Error parsing slope-field attribute @spacings={element.get('spacings')}")
            return
    else:   
        rx = grid_axes.find_gridspacing((bbox[0], bbox[2]))
        ry = grid_axes.find_gridspacing((bbox[1], bbox[3]))

    x = rx[0]
    while x <= rx[2]:
        y = ry[0]
        while y <= ry[2]:
            line = copy.deepcopy(line_template)
            if system:
                change = f(0, [x,y])
                if math_util.length(change) > 1e-05:
                    element.append(line)
                if abs(change[0]) < 1e-08:
                    dx = 0
                    dy = ry[1]/4
                    if change[1] < 0:
                        dy *= -1
                else:
                    slope = change[1]/change[0]
                    dx = rx[1]/4
                    dy = slope*dx
                    if abs(dy) > ry[1]/4:
                        dy = ry[1]/4
                        dx = dy/slope
                    if change[0] * dx < 0:
                        dx *= -1
                        dy *= -1
            else:
                dx = None
                try:
                    slope = f(x,y)
                except ZeroDivisionError:
                    dx = 0
                    dy = ry[1]/4
                if dx is None:
                    dx = rx[1]/4
                    dy = slope*dx
                    if abs(dy) > ry[1]/4:
                        dy = ry[1]/4
                        dx = dy/slope
                    if dx < 0:
                        dx *= -1
                        dy *= -1
                element.append(line)
            x0 = x - dx
            x1 = x + dx
            y0 = y - dy
            y1 = y + dy
            line.set('p1', utilities.pt2long_str((x0,y0), spacer=','))
            line.set('p2', utilities.pt2long_str((x1,y1), spacer=','))
            y += ry[1]
        x += rx[1]

    group.group(element, diagram, parent, outline_status)

# Add a graphical element for slope fields
def vector_field(element, diagram, parent, outline_status):
    if outline_status == 'finish_outline':
        finish_outline(element, diagram, parent)
        return

    try:
        f = un.valid_eval(element.get('function'))
    except:
        log.error(f"Error retrieving slope-field function={element.get('function')}")
        return
    bbox = diagram.bbox()

    # We're going to turn this element into a group and add lines to it
    element.tag = "group"
    if element.get('outline', 'no') == 'yes':
        element.set('outline', 'always')

    # Now we'll construct a line with all the graphical information
    # and make copies of it
    line_template = ET.Element('line')

    if diagram.output_format() == 'tactile':
        line_template.set('stroke', 'black')
    else:
        line_template.set('stroke', element.get('stroke', 'blue'))
    line_template.set('thickness', element.get('thickness', '2'))
    if element.get('arrows', 'yes') == 'yes':
        line_template.set('arrows', '1')

    if element.get('arrow-width', None) is not None:
        line_template.set('arrow-width', element.get('arrow-width'))
    if element.get('arrow-angles', None) is not None:
        line_template.set('arrow-angles', element.get('arrow-angles'))

    spacings = element.get('spacings', None)
    if spacings is not None:
        try:
            spacings = un.valid_eval(spacings)
            rx, ry = spacings
        except:
            log.error(f"Error parsing slope-field attribute @spacings={element.get('spacings')}")
            return
    else:
        rx = grid_axes.find_gridspacing((bbox[0], bbox[2]))
        ry = grid_axes.find_gridspacing((bbox[1], bbox[3]))

    # we will go through and generate the vectors first
    # since we'll need to scale them
    field_data = []
    max_scale = 0
    x = rx[0]
    while x <= rx[2]:
        y = ry[0]
        while y <= ry[2]:
            f_value = f(x, y)
            try:
                if len(f_value) != 2:
                    log.error("Only two-dimensional vector fields are supported")
                    return;
            except:
                pass
            max_scale = max(max_scale,
                            abs((f_value[0])/rx[1]),
                            abs((f_value[1])/ry[1]))
            field_data.append([np.array([x,y]), f_value])
            y += ry[1]
        x += rx[1]

    scale_factor = min(1, 0.75 / max_scale)
    if element.get('scale') is not None:
        scale = un.valid_eval(element.get('scale'))
        scale_factor = scale

    for datum in field_data:
        p, v = datum
        v = scale_factor * v
        # is this long enough to add?
        tail = p
        tip  = p+v
        p0 = diagram.transform(tail)
        p1 = diagram.transform(tip)
        if math_util.distance(p0, p1) < 2:
            continue

        line_el = copy.deepcopy(line_template)
        line_el.set('p1', utilities.pt2long_str(tail, spacer=','))
        line_el.set('p2', utilities.pt2long_str(tip, spacer=','))
        element.append(line_el)

    group.group(element, diagram, parent, outline_status)

def gradient(element, diagram, parent, outline_status):
    try:
        f = un.valid_eval(element.get("function"))
    except:
        log.error("Error retrieving function in gradient element")
        return

    def grad(a, b):
        f_x_trace = lambda x: f(x, b)
        f_y_trace = lambda y: f(a, y)
        return np.array([calculus.derivative(f_x_trace, a),
                         calculus.derivative(f_y_trace, b)])
    un.enter_namespace("__grad", grad)
    element.set("function", "__grad")
    vector_field(element, diagram, parent, outline_status)
