import lxml.etree as ET
import logging
import numpy as np
import math
import copy
from .. import user_namespace as un
from .. import utilities as util
from .. import math_utilities as math_util
from .. import CTM
from .. import label

log = logging.getLogger('prefigure')

def find_terminal(terminal):
    if isinstance(terminal[0], np.ndarray):
        return terminal
    return terminal, None

def connection(element, diagram, parent, data):
    convention = data['convention']

    start = element.get('start', None)
    if start is None:
        log.error('A connection element needs a start attribute')
        return

    start = un.valid_eval(start)
    start, current_direction = find_terminal(start)

    start = diagram.transform(start)
    path = ET.SubElement(parent, 'path')
    cmds = ['M ', util.pt2str(start)]
    current_pt = start

    for child in element:
        if child.tag == "wire":
            next_cmds, current_pt = wire(child, diagram, parent, current_pt,
                                         current_direction, data)
            cmds += next_cmds
            continue
        if child.tag == "resistor":
            next_cmds, current_pt = resistor(child, diagram, parent, current_pt,
                                             current_direction, data)
            cmds += next_cmds
            continue
        if child.tag == "inductor":
            next_cmds, current_pt = inductor(child, diagram, parent, current_pt,
                                             current_direction, data)
            cmds += next_cmds
            continue
        if child.tag == "capacitor":
            next_cmds, current_pt = capacitor(child, diagram, parent, current_pt,
                                              current_direction, data)
            cmds += next_cmds
            continue
    path.set('stroke', 'black')
    path.set('d', ''.join(cmds))
    path.set('fill', 'none')

def log_pt(pt):
    log.error((pt[0], pt[1]))

def plot_path(current_pt, current_direction, end_pt, end_direction):
    waypts = [current_pt]
    if current_direction is None and end_direction is None:
        diff = end_pt - current_pt
        if np.isclose(diff[0], 0) or np.isclose(diff[1], 0):
            waypts = [current_pt, end_pt]
            return waypts
        waypts.append(((current_pt[0]+end_pt[0])/2, current_pt[1]))
        waypts.append(((current_pt[0]+end_pt[0])/2, end_pt[1]))
        waypts.append(end_pt)
        return waypts

    reversed = False
    if current_direction is None and end_direction is not None:
        current_pt, end_pt = end_pt, current_pt
        current_direction, end_direction = end_direction, current_direction
        reversed = True

    ctm = CTM.CTM()
    ctm.translate(*current_pt)
    angle = math.atan2(current_direction[1], current_direction[0])
    ctm.rotate(angle, units="radians")

    def_step = 30
    end = ctm.inverse_transform(end_pt)
    if current_direction is not None and end_direction is None:
        if end[0] > 0:
            waypts.append(ctm.transform((end[0],0)))
            waypts.append(end_pt)
        elif end[0] == 0:
            waypts.append(ctm.transform((def_step,0)))
            waypts.append(ctm.transform((def_step,end[1])))
            waypts.append(end_pt)
        else:
            waypts.append(ctm.transform((def_step,0)))
            if np.isclose(end[1], 0):
                waypts.append(ctm.transform((def_step,0)))
                waypts.append(ctm.transform((def_step,def_step)))
                waypts.append(ctm.transform((end[0],def_step)))
                waypts.append(end_pt)
            else:
                waypts.append(ctm.transform((def_step,end[1])))
                waypts.append(end_pt)
        if reversed:
            waypts.reverse()
        return waypts
    
    # now we have directions on both ends of the wire
    end_direction = math_util.rotate(end_direction, -angle)
    end_direction *= -1
    if np.isclose(end[1], 0):  # end point is on axis
        if end[0] > 0:  # end point on positive axis
            if np.isclose(end_direction[1], 0):
                if end_direction[0] > 0:
                    waypts.append(end_pt)
                    return waypts
                else:
                    waypts.append(ctm.transform((end[0]-def_step,0)))
                    waypts.append(ctm.transform((end[0]-def_step,def_step)))
                    waypts.append(ctm.transform((end[0]+def_step,def_step)))
                    waypts.append(ctm.transform((end[0]+def_step,0)))
                    waypts.append(end_pt)
                    return waypts
            else:
                if end_direction[1] < 0:
                    end_direction[1] *= -1
                    ctm.scale(1,-1)
                waypts.append(ctm.transform((end[0]-def_step,0)))
                waypts.append(ctm.transform((end[0]-def_step,-def_step)))
                waypts.append(ctm.transform((end[0],-def_step)))
                waypts.append(end_pt)
                return waypts
        # end point on negative axis
        if np.isclose(end_direction[1], 0):
            if end_direction[0] > 0:
                waypts.append(ctm.transform((def_step,0)))
                waypts.append(ctm.transform((def_step,def_step)))
                waypts.append(ctm.transform((end[0]-def_step, def_step)))
                waypts.append(ctm.transform((end[0]-def_step, 0)))
                waypts.append(end_pt)
                return waypts
            else:
                waypts.append(ctm.transform((def_step,0)))
                waypts.append(ctm.transform((def_step,def_step)))
                waypts.append(ctm.transform((end[0]+def_step, def_step)))
                waypts.append(ctm.transform((end[0]+def_step, 0)))
                waypts.append(end_pt)
                return waypts
        if end_direction[1] < 0:
            ctm.scale(1,-1)
            end_direction[1] *= -1
        waypts.append(ctm.transform((def_step,0)))
        waypts.append(ctm.transform((def_step,-def_step)))
        waypts.append(ctm.transform((end[0], -def_step)))
        waypts.append(end_pt)
        return waypts
                
    if end[0] > 0: # in quadrant 1 or 4
        if end[1] < 0:
            ctm.scale(1, -1)
            end[1] *= -1
            end_direction[1] *= -1
        if np.isclose(end_direction[0], 0): # end direction is vertical
            if end_direction[1] > 0:
                waypts.append(ctm.transform((end[0],0)))
                waypts.append(end_pt)
                return waypts
            else:
                waypts.append(ctm.transform((end[0]/2,0)))
                waypts.append(ctm.transform((end[0]/2,end[1] + def_step)))
                waypts.append(ctm.transform((end[0],end[1] + def_step)))
                waypts.append(end_pt)
                return waypts
        else: # end direction is horizontal
            if end_direction[0] > 0:
                waypts.append(ctm.transform((end[0]/2,0)))
                waypts.append(ctm.transform((end[0]/2,end[1])))
                waypts.append(end_pt)
                return waypts
            else:
                waypts.append(ctm.transform((end[0]+def_step,0)))
                waypts.append(ctm.transform((end[0]+def_step,end[1])))
                waypts.append(end_pt)
                return waypts

    # we're in quadrant 2 or 3
    if end[1] < 0:
        ctm.scale(1, -1)
        end[1] *= -1
        end_direction[1] *= -1
        
    if np.isclose(end_direction[0], 0):  # vertical final direction
        if end_direction[1] > 0:
            waypts.append(ctm.transform((def_step,0)))
            waypts.append(ctm.transform((def_step,end[1]/2)))
            waypts.append(ctm.transform((end[0],end[1]/2)))
            waypts.append(end_pt)
            return waypts
        else:
            waypts.append(ctm.transform((def_step,0)))
            waypts.append(ctm.transform((def_step,end[1] + def_step)))
            waypts.append(ctm.transform((end[0],end[1] + def_step)))
            waypts.append(end_pt)
            return waypts
    if end_direction[0] < 0:
        waypts.append(ctm.transform((def_step,0)))
        waypts.append(ctm.transform((def_step,end[1])))
        waypts.append(end_pt)
        return waypts
    else: 
        waypts.append(ctm.transform((def_step,0)))
        waypts.append(ctm.transform((def_step,end[1]/2)))
        waypts.append(ctm.transform((end[0]-def_step,end[1]/2)))
        waypts.append(ctm.transform((end[0]-def_step,end[1])))
        waypts.append(end_pt)
        return waypts

def mk_path(waypts):
    p0 = waypts.pop(0)
    cmds = ['M ', util.pt2str(p0)]
    for p in waypts:
        cmds += ['L ', util.pt2str(p)]
    return cmds

def wire(child, diagram, parent, current_pt, current_direction, data):
    p = child.get("to", None)
    if p is None:
        log.error(f"A {child.tag} element needs an attribute to")
        return
    p = un.valid_eval(p)
    end_pt, end_direction = find_terminal(p)
    end_pt = diagram.transform(end_pt)

    waypts = plot_path(current_pt, current_direction,
                       end_pt, end_direction)
    return mk_path(waypts), end_pt

def inductor(child, diagram, parent, current_pt,
             current_direction, data):
    has_label = label.has_label(child)
    if has_label:
        parent = ET.SubElement(parent, 'g')
        id_element = parent
    path = ET.SubElement(parent, 'path')
    if not has_label:
        id_element = path
    diagram.add_id(id_element, child.get('id'))

    scale = data['scale']
    N = int(un.valid_eval(child.get('coils', '4')))
    other = 0.1 * scale        # x-radius of small return arc (fixed per-coil geometry)
    step = 0.275 * scale       # x-radius of large arc (fixed per-coil geometry, from N=4 default)
    body_half = N * step - (N - 1) * other  # widens with more coils
    b_up = 0.3 * scale        # y-radius of large arcs (above wire)
    b_down = 0.15 * scale     # y-radius of small return arcs (below wire)
    k = 4 * (math.sqrt(2) - 1) / 3  # quarter-ellipse Bezier constant ≈ 0.5523

    p = child.get("to", None)
    if p is None:
        log.error(f"A {child.tag} element needs an attribute to")
        return
    p = un.valid_eval(p)
    end_pt, end_direction = find_terminal(p)
    end_pt = diagram.transform(end_pt)
    waypts = plot_path(current_pt, current_direction,
                       end_pt, end_direction)

    segments = zip(waypts[:-1], waypts[1:])
    longest = np.argmax([math_util.length(p-q) for p,q in segments])
    
    diff = waypts[longest+1] - waypts[longest]
    mid_pt = 1/2*(waypts[longest] + waypts[longest+1])
    angle = math.degrees(math.atan2(diff[1], diff[0]))
    ctm = CTM.CTM()
    ctm.translate(*mid_pt)
    ctm.rotate(angle)

    body_start = ctm.transform(np.array((-body_half, 0)))
    body_end   = ctm.transform(np.array(( body_half, 0)))
    first_path = waypts[:longest+1] + [body_start]
    second_path = [body_end] + waypts[longest+1:]
    cmds = mk_path(first_path) + mk_path(second_path)

    x = -body_half
    d = 'M ' + util.pt2str(ctm.transform(np.array((x, 0))))
    for i in range(N):
        # large arc above: (x,0) → (x+2*step,0) through peak (x+step, -b_up)
        xc, x1 = x + step, x + 2 * step
        cp1 = ctm.transform(np.array((x,          -k * b_up)))
        cp2 = ctm.transform(np.array((xc - k*step, -b_up   )))
        p3  = ctm.transform(np.array((xc,          -b_up   )))
        d += f' C {util.pt2str(cp1)} {util.pt2str(cp2)} {util.pt2str(p3)}'
        cp1 = ctm.transform(np.array((xc + k*step, -b_up   )))
        cp2 = ctm.transform(np.array((x1,          -k*b_up )))
        p3  = ctm.transform(np.array((x1,          0       )))
        d += f' C {util.pt2str(cp1)} {util.pt2str(cp2)} {util.pt2str(p3)}'
        x = x1
        if i < N - 1:
            # small return arc below: (x,0) → (x-2*other,0) through (x-other, +b_down)
            xc, x1 = x - other, x - 2 * other
            cp1 = ctm.transform(np.array((x,              k * b_down)))
            cp2 = ctm.transform(np.array((xc + k*other,   b_down    )))
            p3  = ctm.transform(np.array((xc,             b_down    )))
            d += f' C {util.pt2str(cp1)} {util.pt2str(cp2)} {util.pt2str(p3)}'
            cp1 = ctm.transform(np.array((xc - k*other,   b_down    )))
            cp2 = ctm.transform(np.array((x1,             k * b_down)))
            p3  = ctm.transform(np.array((x1,             0         )))
            d += f' C {util.pt2str(cp1)} {util.pt2str(cp2)} {util.pt2str(p3)}'
            x = x1
    path.set('d', d)
    path.set('stroke', 'black')
    path.set('fill', 'none')

    if has_label:
        add_label(child, diagram, parent, mid_pt, b_up, diff)

    return cmds, end_pt

def resistor(child, diagram, parent, current_pt,
             current_direction, data):
    has_label = label.has_label(child)
    if has_label:
        parent = ET.SubElement(parent, 'g')
        id_element = parent
    path = ET.SubElement(parent, 'path')
    if not has_label:
        id_element = path
    diagram.add_id(id_element, child.get('id'))

    scale = data['scale']
    T = int(un.valid_eval(child.get('teeth', '3')))
    H = 0.3 * scale
    step = 0.8 * scale / 6    # per-tooth geometry fixed at teeth=3 default
    body_half = 2 * T * step  # widens with more teeth

    p = child.get("to", None)
    if p is None:
        log.error(f"A {child.tag} element needs an attribute to")
        return
    p = un.valid_eval(p)
    end_pt, end_direction = find_terminal(p)
    end_pt = diagram.transform(end_pt)
    waypts = plot_path(current_pt, current_direction,
                       end_pt, end_direction)

    segments = zip(waypts[:-1], waypts[1:])
    longest = np.argmax([math_util.length(p-q) for p,q in segments])
    
    diff = waypts[longest+1] - waypts[longest]
    mid_pt = 1/2*(waypts[longest] + waypts[longest+1])
    angle = math.degrees(math.atan2(diff[1], diff[0]))
    ctm = CTM.CTM()
    ctm.translate(*mid_pt)
    ctm.rotate(angle)

    body_start = ctm.transform(np.array((-body_half, 0)))
    body_end   = ctm.transform(np.array(( body_half, 0)))
    first_path = waypts[:longest+1] + [body_start]
    second_path = [body_end] + waypts[longest+1:]
    cmds = mk_path(first_path) + mk_path(second_path)

    vert = -1
    pt = np.array((-body_half, 0))
    d = 'M ' + util.pt2str(ctm.transform(pt))
    pt += np.array((step, vert * H))
    d += ' L ' + util.pt2str(ctm.transform(pt))
    for _ in range(2 * T - 1):
        vert *= -1
        pt += np.array((2*step, vert * 2*H))
        d += ' L ' + util.pt2str(ctm.transform(pt))
    vert *= -1
    pt += np.array((step, vert * H))
    d += ' L ' + util.pt2str(ctm.transform(pt))
    path.set('d', d)
    path.set('stroke', 'black')
    path.set('fill', 'none')

    if has_label:
        add_label(child, diagram, parent, mid_pt, H, diff)

    return cmds, end_pt

def capacitor(child, diagram, parent, current_pt,
              current_direction, data):
    has_label = label.has_label(child)
    if has_label:
        parent = ET.SubElement(parent, 'g')
        id_element = parent
    path = ET.SubElement(parent, 'path')
    if not has_label:
        id_element = path
    diagram.add_id(id_element, child.get('id'))
    
    scale = data['scale']
    gap_half = 0.2 * scale    # half the gap between plates
    plate_half = 0.6 * scale  # half the plate length

    p = child.get("to", None)
    if p is None:
        log.error(f"A {child.tag} element needs an attribute to")
        return
    to_pt = diagram.transform(un.valid_eval(p))
    diff = to_pt - current_pt
    mid_pt = 1/2*(to_pt + current_pt)
    angle = math.degrees(math.atan2(diff[1], diff[0]))
    ctm = CTM.CTM()
    ctm.translate(*mid_pt)
    ctm.rotate(angle)

    # wire to left plate, then jump path to right plate and wire to destination
    left_pt  = ctm.transform(np.array((-gap_half, 0)))
    right_pt = ctm.transform(np.array(( gap_half, 0)))
    cmds = ['L ', util.pt2str(left_pt),
            'M ', util.pt2str(right_pt),
            'L ', util.pt2str(to_pt)]

    # left plate
    p1 = ctm.transform(np.array((-gap_half, -plate_half)))
    p2 = ctm.transform(np.array((-gap_half,  plate_half)))
    d = f"M {p1[0]} {p1[1]} L {p2[0]} {p2[1]} "

    # right plate
    p1 = ctm.transform(np.array(( gap_half, -plate_half)))
    p2 = ctm.transform(np.array(( gap_half,  plate_half)))
    d += f"M {p1[0]} {p1[1]} L {p2[0]} {p2[1]}"
    path.set('d', d)
    path.set('stroke', 'black')

    if has_label:
        add_label(child, diagram, parent, mid_pt, plate_half, diff)

    return cmds, to_pt

def add_label(element, diagram, parent, anchor, initial_offset, direction):
    element = copy.deepcopy(element)
    alignment = element.get('alignment')
    if alignment is not None:
        offset_direction = label.alignment_directions.get(alignment, (0,0))
        if math_util.length(offset_direction) > 0:
            offset_direction = math_util.normalize(offset_direction)
    else:
        offset_direction = math_util.rotate(direction, -math.pi/2)
        if offset_direction[1] > 0:
            offset_direction[1] *= -1
        offset_direction = math_util.normalize(offset_direction)

    anchor = np.array(anchor) + initial_offset * offset_direction
    offset = un.valid_eval(element.get('offset', '(0,0)'))

    if alignment is None:
        offset_direction[1] *= -1
        alignment = label.get_alignment_from_direction(offset_direction)
        element.set('alignment', alignment)

    element.attrib.pop('at', None)
    element.attrib.pop('id', None)
    element.set('anchor', f"({anchor[0]}, {anchor[1]})")
    element.set('offset', f"({offset[0]}, {offset[1]})")
    element.set('user-coords', 'no')
    element.tag = 'label'
    label.label(element, diagram, parent, None)
