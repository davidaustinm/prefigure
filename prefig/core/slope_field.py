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
                slope = f(x,y)
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

