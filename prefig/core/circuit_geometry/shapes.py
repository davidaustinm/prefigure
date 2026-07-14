import lxml.etree as ET
import logging
import numpy as np
from .. import user_namespace as un
from .. import utilities as util
from .. import math_utilities as math_util
from .. import CTM

log = logging.getLogger('prefigure')

def battery(element, diagram, parent, data):
    convention = data['convention']

    location_str = element.get('location')
    if location_str is None:
        log.error('battery element needs a location attribute')
        return
    origin = un.valid_eval(location_str)
    origin = diagram.transform(origin)

    scale = data['scale']
    W = 0.3 * scale
    H = 0.6 * scale

    ctm = CTM.CTM()
    ctm.translate(*origin)
    rotation_str = element.get('rotate', None)
    if rotation_str is not None:
        rotation = un.valid_eval(rotation_str)
        ctm.rotate(-rotation)

    positive = np.array((0, -W))
    negative = np.array((0,  W))
    plate_positions = [
        np.array((0, -W)),      # positive terminal, long
        np.array((0, -W/3)),    # inner near positive, short
        np.array((0,  W/3)),    # inner near negative, long
        np.array((0,  W)),      # negative terminal, short
    ]

    for i, pos in enumerate(plate_positions):
        line = ET.SubElement(parent, 'line')
        offset = np.array((H, 0)) if i % 2 == 0 else np.array((H/2, 0))
        p1 = ctm.transform(pos + offset)
        p2 = ctm.transform(pos - offset)
        line.set('x1', f"{p1[0]}")
        line.set('y1', f"{p1[1]}")
        line.set('x2', f"{p2[0]}")
        line.set('y2', f"{p2[1]}")
        line.set('stroke', 'black')

    center = ctm.transform((0,0))
    positive_direction = ctm.transform((0,-1)) - center
    negative_direction = ctm.transform((0, 1)) - center

    id = element.get('at', None)
    if id is not None:
        p1 = diagram.inverse_transform(ctm.transform(positive))
        p2 = diagram.inverse_transform(ctm.transform(negative))
        terminals = {'positive': [p1, positive_direction],
                     'negative': [p2, negative_direction]}
        un.enter_namespace(id, terminals)
