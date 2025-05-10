import lxml.etree as ET
import math
import numpy as np
from . import CTM
from . import user_namespace as un
from . import math_utilities as math_util

import logging
log = logging.getLogger('prefigure')

# add a group to the diagram for one of two possible reasons:
# to group graphical components to be annotated as a group or 
# to place outlines behind all of the grouped components before 
# stroking them
def group(element, diagram, parent, outline_status):
    # determine whether we will outline the grouped components together
    outline = element.get('outline', None)
    if outline == 'always' or outline == diagram.output_format():
        # we'll pass through the grouped components twice first adding
        # the outline
        group = ET.SubElement(parent, 'g')
        diagram.add_id(group, element.get('id'))
        diagram.parse(element, group, outline_status = 'add_outline')

        # then stroking the grouped components
        group = ET.SubElement(parent, 'g')
        diagram.add_id(group, element.get('id'))
        diagram.parse(element, group, outline_status = 'finish_outline')

        return

    group = ET.SubElement(parent, 'g')
    diagram.add_id(group, element.get('id'))

    transform = element.get('transform', None)
    if transform is not None:
        transform = transform.strip()
        if transform.startswith('translate'):
            index = transform.find('(')
            vec = un.valid_eval(transform[index:])
            diff = diagram.transform(vec) - diagram.transform((0,0))
            t_string = CTM.translatestr(*diff)
            group.set('transform', t_string)
        if transform.startswith('reflect'):
            index = transform.find('(')
            data = un.valid_eval(transform[index:])
            if len(data) == 2:
                p1, p2 = data
            if len(data) == 3:
                # is the line vertical
                if np.isclose(B, 0):
                    p1 = np.array((C/A, 0))
                    p2 = np.array((C/A, 1))
                else:
                    p1 = np.array((0,C/B))
                    p2 = np.array((1,(C-A)/B))
            p1 = diagram.transform(p1)
            p2 = diagram.transform(p2)
            diff = p1 - p2
            angle = math.degrees(math.atan2(diff[1],diff[0]))
            t_string = CTM.translatestr(*p1)
            t_string += ' ' + CTM.rotatestr(-angle)
            t_string += ' ' + CTM.scalestr(1,-1)
            t_string += ' ' + CTM.rotatestr(angle)
            t_string += ' ' + CTM.translatestr(*(-p1))
            group.set('transform', t_string)
        if transform.startswith('rotate'):
            index = transform.find('(')
            data = un.valid_eval(transform[index:])
            if isinstance(data, tuple):
                angle = data[0]
                center = diagram.transform(data[1])
            else:
                angle = data
                center = diagram.transform((0,0))
            t_string = CTM.translatestr(*center)
            t_string += ' ' + CTM.rotatestr(angle)
            t_string += ' ' + CTM.translatestr(*(-center))
            group.set('transform', t_string)
        if transform.startswith('scale'):
            index = transform.find('(')
            data = list(un.valid_eval(transform[index:]))
            center = diagram.transform(data.pop(-1))
            if len(data) == 2:
                sx, sy = data
            else:
                sx = data[0]
                sy = data[0]
            t_string = CTM.translatestr(*center)
            t_string += CTM.scalestr(sx, sy)
            t_string += CTM.translatestr(*-center)
            group.set('transform', t_string)
        if transform.startswith('matrix'):
            index = transform.find('(')
            data = un.valid_eval(transform[index:])
            matrix = data[0]
            center = diagram.transform(data[1])
            t_string = CTM.translatestr(*center)
            t_string += CTM.matrixstr(matrix)
            t_string += CTM.translatestr(*-center)
            group.set('transform', t_string)

    diagram.parse(element, group, outline_status)
