## Add a graphical element describing a rectangle

import lxml.etree as ET
import logging
from . import user_namespace as un
from . import utilities as util
from . import math_utilities as math_util
from . import CTM

log = logging.getLogger('prefigure')

# Process a rectangle tag
def rectangle(element, diagram, parent, outline_status):
    if outline_status == 'finish_outline':
        finish_outline(element, diagram, parent)
        return

    # An author may specifiy either the lower-left corner or the center
    try:
        ll = un.valid_eval(element.get('lower-left', '(0,0)'))
        dims = un.valid_eval(element.get('dimensions', '(1,1)'))
        center = element.get('center', None)
        if center is not None:
            center = un.valid_eval(center)
            ll = center - 0.5 * dims
        else:
            center = ll + 0.5*dims
    except:
        log.error(f"Error parsing data in a <rectangle>")
        return
    p0 = ll
    p1 = ll + dims

    # We're going to make a path so that we can use this with shape operations
    path = ET.SubElement(parent, 'path')
    diagram.add_id(path, element.get('id'))

    rotate = un.valid_eval(element.get('rotate', '0'))
    ctm = CTM.CTM()
    ctm.translate(*center)
    ctm.rotate(rotate)
    dx, dy = dims/2
    user_corners = [ctm.transform(p) for p in [
        (-dx, -dy), (dx, -dy), (dx, dy), (-dx, dy)
    ]
                    ]
    
    corners = [diagram.transform(c) for c in user_corners]

    radius = un.valid_eval(element.get('corner-radius', '0'))
    if radius == 0:
        cmds = ['M', util.pt2str(corners[0])]
        for c in corners[1:]:
            cmds += ['L', util.pt2str(c)]
        cmds.append('Z')
    else:
        cmds = []
        corners += corners[:2]
        for i in range(4):
            v1 = math_util.normalize(corners[i+1] - corners[i])
            v2 = math_util.normalize(corners[i+2] - corners[i+1])
            command = 'L'
            if len(cmds) == 0:
                command = 'M'
            cmds += [command, util.pt2str(corners[i+1] - radius*v1)]
            cmds += ['Q',
                     util.pt2str(corners[i+1]),
                     util.pt2str(corners[i+1] + radius*v2)]
        cmds.append('Z')

    path.set('d', ' '.join(cmds))

    if diagram.output_format() == 'tactile':
        if element.get('stroke') is not None:
            element.set('stroke', 'black')
        if element.get('fill') is not None:
            element.set('fill', 'lightgray')
    else:
        util.set_attr(element, 'stroke', 'none')
        util.set_attr(element, 'fill', 'none')

    util.set_attr(element, 'thickness', '2')
    util.add_attr(path, util.get_2d_attr(element))
    util.cliptobbox(path, element, diagram)

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
