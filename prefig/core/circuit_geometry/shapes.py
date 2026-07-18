import lxml.etree as ET
import logging
import numpy as np
import math
import copy
from .. import user_namespace as un
from .. import utilities as util
from .. import math_utilities as math_util
from .. import CTM
from .. import label as label_module
from .. import line as line_module

log = logging.getLogger('prefigure')

def node(element, diagram, parent, data):
    parent = ET.SubElement(parent, 'g')
    diagram.add_id(parent, element.get('id'))

    location_str = element.get('location')
    if location_str is None:
        log.error('node element needs a location attribute')
        return
    origin = un.valid_eval(location_str)
    pt = diagram.transform(origin)

    filled = element.get('filled', 'yes') == 'yes'

    circle = ET.SubElement(parent, 'circle')
    circle.set('cx', str(pt[0]))
    circle.set('cy', str(pt[1]))
    circle.set('r', '3')
    if filled:
        circle.set('fill', 'black')
        circle.set('stroke', 'none')
    else:
        circle.set('fill', 'white')
        circle.set('stroke', 'black')

    if label_module.has_label(element):
        alignment = element.get('alignment', 'northeast')
        el = ET.SubElement(parent, 'label')
        el.text = element.text
        for child in element:
            el.append(copy.deepcopy(child))
        el.set('anchor', f"({pt[0]}, {pt[1]})")
        el.set('user-coords', 'no')
        el.set('alignment', alignment)
        if element.get('offset', None) is not None:
            offset = un.valid_eval(element.get('offset'))
            el.set('offset', f"({offset[0]}, {offset[1]})")
        label_module.label(el, diagram, parent, None)

    at_name = element.get('at', None)
    if at_name is not None:
        un.enter_namespace(at_name, {'location': origin})

def ground(element, diagram, parent, data):
    scale = data['scale']
    step = 0.5*scale

    parent = ET.SubElement(parent, 'g')
    diagram.add_id(parent, element.get('id'))

    location_str = element.get('location')
    if location_str is None:
        log.error('ground element needs a location attribute')
        return
    origin = un.valid_eval(location_str)
    origin = diagram.transform(origin)

    ctm = CTM.CTM()
    ctm.translate(*origin)

    common = element.get('common', 'no') == 'yes'
    stub_y = step if common else 1.2 * step

    # Vertical stub from connection point downward
    p1 = ctm.transform(np.array((0.0,    0.0  )))
    p2 = ctm.transform(np.array((0.0, stub_y  )))
    stub = ET.SubElement(parent, 'line')
    stub.set('x1', str(p1[0])); stub.set('y1', str(p1[1]))
    stub.set('x2', str(p2[0])); stub.set('y2', str(p2[1]))
    stub.set('stroke', 'black')

    if not common:
        # Standard earth ground: three horizontal bars of decreasing width
        for y_pos, half_w in [(1.2*step, 0.6*step),
                              (1.4*step, 0.4*step),
                              (1.6*step, 0.25*step)]:
            q1 = ctm.transform(np.array((-half_w, y_pos)))
            q2 = ctm.transform(np.array(( half_w, y_pos)))
            bar = ET.SubElement(parent, 'line')
            bar.set('x1', str(q1[0])); bar.set('y1', str(q1[1]))
            bar.set('x2', str(q2[0])); bar.set('y2', str(q2[1]))
            bar.set('stroke', 'black')
    else:
        # Signal/common ground: downward-pointing triangle
        tl  = ctm.transform(np.array((-0.6*step, step    )))
        tr  = ctm.transform(np.array(( 0.6*step, step    )))
        tip = ctm.transform(np.array(( 0.0,      1.8*step)))
        poly = ET.SubElement(parent, 'polygon')
        poly.set('points',
                 f"{tl[0]},{tl[1]} {tr[0]},{tr[1]} {tip[0]},{tip[1]}")
        poly.set('stroke', 'black')
        poly.set('fill', 'none')

    # Label defaults to the right of the top bar
    if label_module.has_label(element):
        alignment = element.get('alignment', 'east')
        anchor = ctm.transform(np.array((0.6*step, 1.2*step)))
        el = ET.SubElement(parent, 'label')
        el.text = element.text
        for child in element:
            el.append(copy.deepcopy(child))
        el.set('anchor', f"({anchor[0]}, {anchor[1]})")
        el.set('user-coords', 'no')
        el.set('alignment', alignment)
        if element.get('offset', None) is not None:
            offset = un.valid_eval(element.get('offset'))
            el.set('offset', f"({offset[0]}, {offset[1]})")
        label_module.label(el, diagram, parent, None)

    # Terminal dictionary — one connection point at the top
    at_name = element.get('at', None)
    if at_name is not None:
        center_pt = diagram.inverse_transform(ctm.transform(np.array((0.0, 0.0))))
        center_svg = ctm.transform(np.array((0.0, 0.0)))
        up_dir = ctm.transform(np.array((0.0, -1.0))) - center_svg
        terminals = {'center': [center_pt, up_dir]}
        un.enter_namespace(at_name, terminals)

def battery(element, diagram, parent, data):
    convention = data['convention']

    parent = ET.SubElement(parent, 'g')
    diagram.add_id(parent, element.get('id'))

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

    if label_module.has_label(element):
        alignment = element.get('alignment', None)
        if alignment is None:
            direction = ctm.transform(negative) - ctm.transform(positive)
            direction[1] *= -1
            direction = math_util.rotate(direction,
                                         math.pi/2)
            alignment = label_module.get_alignment_from_direction(direction)
        anchor = ctm.transform((H, 0))
        el = ET.SubElement(parent, 'label')
        el.text = element.text
        for child in element:
            el.append(copy.deepcopy(child))
        el.set('anchor', f"({anchor[0]}, {anchor[1]})")
        el.set('user-coords', 'no')
        el.set('alignment', alignment)
        if element.get('offset', None) is not None:
            offset = un.valid_eval(element.get('offset'))
            el.set('offset', f"({offset[0]}, {offset[1]})")
        label_module.label(el, diagram, parent, None)

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
    scale = 1.4*data['scale']

    parent = ET.SubElement(parent, 'g')
    diagram.add_id(parent, element.get('id'))

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
    offset_direction = np.array((1,0))
    if rotation_str is not None:
        angle = -un.valid_eval(rotation_str)
        ctm.rotate(angle)
        rad_angle = math.radians(-angle)
        offset_direction = math_util.rotate(offset_direction,
                                            -rad_angle)

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
    offset_scale = {'-': 6, '+': 6}
    for sign, y in [('-', -input_h), ('+', input_h)]:
        label_pos = np.array((-body_x, y))
        pt = ctm.transform(label_pos)
        pt += offset_scale[sign] * offset_direction
        el = ET.Element('label')
        math_el = ET.SubElement(el, 'm')
        math_el.text = sign
        el.set('anchor', f"({pt[0]}, {pt[1]})")
        el.set('alignment', 'center')
        el.set('scale', f"{0.75*data['scale']/20}")
        el.set('user-coords', 'no')
        el.set('font-size', font_size)
        label_module.label(el, diagram, parent, None)

    font_size = str(round(0.4 * scale, 1))
    if label_module.has_label(element):
        el = ET.SubElement(parent, 'label')
        el.text = element.text
        for child in element:
            el.append(copy.deepcopy(child))
        anchor = ctm.transform((0,0))
        el.set('anchor', f"({anchor[0]}, {anchor[1]})")
        el.set('alignment', 'center')
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

def dc_current_source(element, diagram, parent, data):
    scale = 1.2*data['scale']
    radius     = 0.6  * scale
    arrow_len  = 0.15 * scale   # arrowhead length
    arrow_half = 0.08 * scale   # arrowhead half-width

    parent = ET.SubElement(parent, 'g')
    diagram.add_id(parent, element.get('id'))

    location_str = element.get('location')
    if location_str is None:
        log.error('dc_current_source element needs a location attribute')
        return
    origin = un.valid_eval(location_str)
    origin = diagram.transform(origin)

    ctm = CTM.CTM()
    ctm.translate(*origin)
    rotation_str = element.get('rotate', None)
    if rotation_str is not None:
        rotation = un.valid_eval(rotation_str)
        ctm.rotate(-rotation)

    invert = element.get('invert', 'no') == 'yes'
    sign = -1 if invert else 1

    # Circle
    center = ctm.transform(np.array((0.0, 0.0)))
    circle = ET.SubElement(parent, 'circle')
    circle.set('cx', str(center[0]))
    circle.set('cy', str(center[1]))
    circle.set('r',  str(radius))
    circle.set('stroke', 'black')
    circle.set('fill', 'none')

    # Arrow shaft ending at the base of the arrowhead
    arrow_x = sign * 0.5 * radius
    shaft_start = ctm.transform(np.array((-sign * 0.7 * radius, 0.0)))
    shaft_end   = ctm.transform(np.array((arrow_x - sign * arrow_len / 2, 0.0)))
    line = ET.SubElement(parent, 'line')
    line.set('x1', str(shaft_start[0]))
    line.set('y1', str(shaft_start[1]))
    line.set('x2', str(shaft_end[0]))
    line.set('y2', str(shaft_end[1]))
    line.set('stroke', 'black')
    line.set('thickness', '2')

    # Filled arrowhead triangle
    tip   = ctm.transform(np.array((arrow_x + sign * arrow_len / 2,  0.0       )))
    base1 = ctm.transform(np.array((arrow_x - sign * arrow_len / 2, -arrow_half)))
    base2 = ctm.transform(np.array((arrow_x - sign * arrow_len / 2,  arrow_half)))
    poly = ET.SubElement(parent, 'polygon')
    poly.set('points',
             f"{tip[0]},{tip[1]} {base1[0]},{base1[1]} {base2[0]},{base2[1]}")
    poly.set('fill', 'black')
    poly.set('stroke', 'none')

    # Label
    if label_module.has_label(element):
        alignment = element.get('alignment', None)
        if alignment is None:
            direction = ctm.transform(np.array((0.0, -1.0))) - center
            direction[1] *= -1
            alignment = label_module.get_alignment_from_direction(direction)
        anchor = ctm.transform(np.array((0.0, -radius)))
        el = ET.SubElement(parent, 'label')
        el.text = element.text
        for child in element:
            el.append(copy.deepcopy(child))
        el.set('anchor', f"({anchor[0]}, {anchor[1]})")
        el.set('user-coords', 'no')
        el.set('alignment', alignment)
        if element.get('offset', None) is not None:
            offset = un.valid_eval(element.get('offset'))
            el.set('offset', f"({offset[0]}, {offset[1]})")
        label_module.label(el, diagram, parent, None)

    # Terminal dictionary
    at_name = element.get('at', None)
    if at_name is not None:
        center_pt = ctm.transform(np.array((0.0, 0.0)))
        left_dir  = ctm.transform(np.array((-1.0, 0.0))) - center_pt
        right_dir = ctm.transform(np.array(( 1.0, 0.0))) - center_pt
        left_pt   = diagram.inverse_transform(ctm.transform(np.array((-radius, 0.0))))
        right_pt  = diagram.inverse_transform(ctm.transform(np.array(( radius, 0.0))))
        if not invert:
            terminals = {'in':  [left_pt,  left_dir],
                         'out': [right_pt, right_dir]}
        else:
            terminals = {'in':  [right_pt, right_dir],
                         'out': [left_pt,  left_dir]}
        un.enter_namespace(at_name, terminals)

def diode(element, diagram, parent, data):
    scale = data['scale']
    half_w = 0.4 * scale
    half_h = 0.5 * scale

    parent = ET.SubElement(parent, 'g')
    diagram.add_id(parent, element.get('id'))

    location_str = element.get('location')
    if location_str is None:
        log.error('diode element needs a location attribute')
        return
    origin = un.valid_eval(location_str)
    origin = diagram.transform(origin)

    ctm = CTM.CTM()
    ctm.translate(*origin)
    rotation_str = element.get('rotate', None)
    if rotation_str is not None:
        rotation = un.valid_eval(rotation_str)
        ctm.rotate(-rotation)

    invert = element.get('invert', 'no') == 'yes'
    sign = -1 if invert else 1

    # Triangle: base on left, tip pointing right (or flipped if invert)
    apex  = ctm.transform(np.array(( sign * half_w,  0.0   )))
    base1 = ctm.transform(np.array((-sign * half_w, -half_h)))
    base2 = ctm.transform(np.array((-sign * half_w,  half_h)))
    poly = ET.SubElement(parent, 'polygon')
    poly.set('points',
             f"{apex[0]},{apex[1]} {base1[0]},{base1[1]} {base2[0]},{base2[1]}")
    poly.set('fill', 'none')
    poly.set('stroke', 'black')

    # Cathode bar at the tip (right side, or left if inverted)
    bar_top = ctm.transform(np.array(( sign * half_w, -half_h)))
    bar_bot = ctm.transform(np.array(( sign * half_w,  half_h)))
    bar = ET.SubElement(parent, 'line')
    bar.set('x1', str(bar_top[0]))
    bar.set('y1', str(bar_top[1]))
    bar.set('x2', str(bar_bot[0]))
    bar.set('y2', str(bar_bot[1]))
    bar.set('stroke', 'black')

    # Label
    center = ctm.transform(np.array((0.0, 0.0)))
    if label_module.has_label(element):
        alignment = element.get('alignment', None)
        if alignment is None:
            direction = ctm.transform(np.array((0.0, -1.0))) - center
            direction[1] *= -1
            alignment = label_module.get_alignment_from_direction(direction)
        anchor = ctm.transform(np.array((0.0, -half_h)))
        el = ET.SubElement(parent, 'label')
        el.text = element.text
        for child in element:
            el.append(copy.deepcopy(child))
        el.set('anchor', f"({anchor[0]}, {anchor[1]})")
        el.set('user-coords', 'no')
        el.set('alignment', alignment)
        if element.get('offset', None) is not None:
            offset = un.valid_eval(element.get('offset'))
            el.set('offset', f"({offset[0]}, {offset[1]})")
        label_module.label(el, diagram, parent, None)

    # Terminal dictionary
    at_name = element.get('at', None)
    if at_name is not None:
        left_dir  = ctm.transform(np.array((-1.0, 0.0))) - center
        right_dir = ctm.transform(np.array(( 1.0, 0.0))) - center
        anode_pt   = diagram.inverse_transform(ctm.transform(np.array((-half_w, 0.0))))
        cathode_pt = diagram.inverse_transform(ctm.transform(np.array(( half_w, 0.0))))
        if not invert:
            terminals = {'anode':   [anode_pt,   left_dir],
                         'cathode': [cathode_pt, right_dir]}
        else:
            terminals = {'anode':   [cathode_pt, right_dir],
                         'cathode': [anode_pt,   left_dir]}
        un.enter_namespace(at_name, terminals)

def transistor(element, diagram, parent, data):
    scale = data['scale']
    W   = 0.6  * scale
    H   = 1.1  * scale
    bh  = 0.4          # fraction of H where diagonal meets base bar
    bh2 = 0.15         # fraction of H where diagonal meets right edge

    parent = ET.SubElement(parent, 'g')
    diagram.add_id(parent, element.get('id'))

    location_str = element.get('location')
    if location_str is None:
        log.error('bjt element needs a location attribute')
        return
    origin = un.valid_eval(location_str)
    origin = diagram.transform(origin)

    ctm = CTM.CTM()
    ctm.translate(*origin)
    rotation_str = element.get('rotate', None)
    if rotation_str is not None:
        rotation = un.valid_eval(rotation_str)
        ctm.rotate(-rotation)

    pnp = element.get('type', 'npn') == 'pnp'

    # x of vertical base bar in local SVG coords
    # bar_x = -W keeps diagonal angles proportional to ctz (base_width=0.5, width=0.6, height=1.1)
    bar_x = -W

    def p(x, y):
        return ctm.transform(np.array((x, y)))

    # Collector:  (0,-H) → (0,-bh*H) → (bar_x, -bh2*H)
    # Base bar:   (bar_x, -bh*H) → (bar_x, +bh*H)
    # Base lead:  (bar_x, 0) → (-2W, 0)
    # Emitter:    (bar_x, +bh2*H) → (0, +bh*H) → (0, +H)
    pts = {
        'coll_end':   p(0,     -H      ),
        'coll_elbow': p(0,     -bh*H   ),
        'coll_joint': p(bar_x, -bh2*H  ),
        'bar_top':    p(bar_x, -bh*H   ),
        'bar_bot':    p(bar_x,  bh*H   ),
        'base_arm':   p(bar_x,  0      ),
        'base_end':   p(-2*W,   0      ),
        'emit_joint': p(bar_x,  bh2*H  ),
        'emit_elbow': p(0,      bh*H   ),
        'emit_end':   p(0,      H      ),
    }

    def pt(name):
        return util.pt2str(pts[name])

    path = ET.SubElement(parent, 'path')
    d = (f"M {pt('coll_end')} L {pt('coll_elbow')} L {pt('coll_joint')} "
         f"M {pt('base_arm')} L {pt('base_end')} "
         f"M {pt('emit_joint')} L {pt('emit_elbow')} L {pt('emit_end')}")
    path.set('d', d)
    path.set('stroke', 'black')
    path.set('fill', 'none')

    bar = ET.SubElement(parent, 'path')
    bar.set('d', f"M {pt('bar_top')} L {pt('bar_bot')}")
    bar.set('stroke', 'black')
    bar.set('stroke-width', '1.5')
    bar.set('fill', 'none')

    # Arrow on emitter diagonal: lower for npn, upper (collector side) for pnp
    if pnp:
        # pnp emitter is the upper terminal; arrow goes inward (elbow → bar joint)
        A = np.array((bar_x, -bh2 * H))
        B = np.array((0.0,   -bh  * H))
        arrow_p1 = B + 0.25 * (A - B)
        arrow_p2 = B + 0.75 * (A - B)
    else:
        # npn emitter is the lower terminal; arrow goes outward (bar joint → elbow)
        A = np.array((bar_x, bh2 * H))
        B = np.array((0.0,   bh  * H))
        arrow_p1 = A + 0.25 * (B - A)
        arrow_p2 = A + 0.75 * (B - A)

    p1_user = diagram.inverse_transform(ctm.transform(arrow_p1))
    p2_user = diagram.inverse_transform(ctm.transform(arrow_p2))
    line_el = ET.Element('line')
    line_el.set('endpoints',
                f"(({p1_user[0]}, {p1_user[1]}), ({p2_user[0]}, {p2_user[1]}))")
    line_el.set('stroke', 'black')
    line_el.set('thickness', '1')
    line_el.set('arrows', '1')
    line_el.set('arrow-width', '5')
    line_el.set('arrow-angles', '(25, 90)')
    line_module.line(line_el, diagram, parent, None)

    # Label
    center = ctm.transform(np.array((0.0, 0.0)))
    if label_module.has_label(element):
        alignment = element.get('alignment', 'east')
        el = ET.SubElement(parent, 'label')
        el.text = element.text
        for child in element:
            el.append(copy.deepcopy(child))
        el.set('anchor', f"({center[0]}, {center[1]})")
        el.set('user-coords', 'no')
        el.set('alignment', alignment)
        if element.get('offset', None) is not None:
            offset = un.valid_eval(element.get('offset'))
            el.set('offset', f"({offset[0]}, {offset[1]})")
        label_module.label(el, diagram, parent, None)

    # Terminals
    at_name = element.get('at', None)
    if at_name is not None:
        up_dir   = ctm.transform(np.array((0.0, -1.0))) - center
        down_dir = ctm.transform(np.array((0.0,  1.0))) - center
        left_dir = ctm.transform(np.array((-1.0, 0.0))) - center

        def user_pt(x, y):
            return diagram.inverse_transform(ctm.transform(np.array((x, y))))

        terminals = {
            'collector': [user_pt(0,    -H), up_dir  ],
            'emitter':   [user_pt(0,     H), down_dir],
            'base':      [user_pt(-2*W, 0), left_dir],
        }
        un.enter_namespace(at_name, terminals)
