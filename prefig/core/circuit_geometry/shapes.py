import lxml.etree as ET
import logging
import numpy as np
from .. import user_namespace as un
from .. import utilities as util
from .. import math_utilities as math_util
from .. import CTM
from .. import label as label_module

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

def op_amp(element, diagram, parent, data):
    scale = data['scale']

    half_h  = 0.9  * scale   # half the total height (~1.29x ctz default)
    half_w  = 1.1  * scale   # half the total width / terminal reach
    body_x  = 0.7  * half_w  # x of triangle left edge and right tip
    input_h = 0.5  * half_h  # y of input terminals

    location_str = element.get('location')
    if location_str is None:
        log.error('op_amp element needs a location attribute')
        return
    origin = un.valid_eval(location_str)
    origin = diagram.transform(origin)

    ctm = CTM.CTM()
    ctm.translate(*origin)
    rotation_str = element.get('rotate', None)
    if rotation_str is not None:
        ctm.rotate(-un.valid_eval(rotation_str))

    # Triangle (left edge vertical, tip pointing right)
    top_left  = ctm.transform(np.array((-body_x, -half_h)))
    bot_left  = ctm.transform(np.array((-body_x,  half_h)))
    right_tip = ctm.transform(np.array(( body_x,  0.0  )))

    poly = ET.SubElement(parent, 'polygon')
    poly.set('points',
             f"{top_left[0]},{top_left[1]} "
             f"{bot_left[0]},{bot_left[1]} "
             f"{right_tip[0]},{right_tip[1]}")
    poly.set('stroke', 'black')
    poly.set('fill', 'none')

    # +/- labels inside the triangle, just right of the left edge
    font_size = str(round(0.7 * scale, 1))
    label_x = -body_x + 0.15 * scale
    for sign, y in [('-', -input_h), ('+', input_h)]:
        pt = ctm.transform(np.array((label_x, y)))
        el = ET.Element('label')
        el.text = sign
        el.set('alignment', 'center')
        el.set('p', f"({pt[0]}, {pt[1]})")
        el.set('user-coords', 'no')
        el.set('font-size', font_size)
        label_module.label(el, diagram, parent, None)

    at_name = element.get('at', None)
    if at_name is not None:
        center    = ctm.transform(np.array(( 0.0,  0.0)))
        left_dir  = ctm.transform(np.array((-1.0,  0.0))) - center
        right_dir = ctm.transform(np.array(( 1.0,  0.0))) - center
        up_dir    = ctm.transform(np.array(( 0.0, -1.0))) - center
        down_dir  = ctm.transform(np.array(( 0.0,  1.0))) - center

        def user_pt(local):
            return diagram.inverse_transform(ctm.transform(np.array(local, dtype=float)))

        terminals = {
            'minus': [user_pt((-body_x, -input_h)), left_dir],
            'plus':  [user_pt((-body_x,  input_h)), left_dir],
            'out':   [user_pt(( body_x,  0.0    )), right_dir],
            'up':    [user_pt(( 0.0,    -half_h / 2)), up_dir],
            'down':  [user_pt(( 0.0,     half_h / 2)), down_dir],
        }
        un.enter_namespace(at_name, terminals)
