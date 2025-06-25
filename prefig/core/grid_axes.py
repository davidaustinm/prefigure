import lxml.etree as ET
import math
import re
import logging
import numpy as np
import copy
from . import math_utilities as math_util
from . import utilities as util
from . import user_namespace as un
from . import label
from . import line
from . import arrow
from . import CTM
from . import axes

log = logging.getLogger('prefigure')

# These tags can appear in an <axes> or <grid-axes>
axes_tags = {'xlabel', 'ylabel'}

def is_axes_tag(tag):
    return tag in axes_tags

# Add graphical elements for grids and axes

# Determine the grid spacing when it's not given
grid_delta = {2: 0.1, 3: 0.25, 4: 0.25, 5: 0.5,
              6: 0.5, 7: 0.5, 8: 0.5, 9: 0.5, 10: 0.5,
              11: 0.5, 12: 1, 13: 1, 14: 1, 15: 1, 16: 1, 
              17: 1, 18: 1, 19: 1, 20: 1 }

def find_gridspacing(coordinate_range, pi_format=False):
    if pi_format:
        coordinate_range = [c/math.pi for c in coordinate_range]
    dx = 1
    distance = abs(coordinate_range[1]-coordinate_range[0])
    while distance > 10:
        distance /= 10
        dx *= 10
    while distance <= 1:
        distance *= 10
        dx /= 10
    dx *= grid_delta[round(2*distance)]
    if coordinate_range[1] < coordinate_range[0]:
        dx *= -1
        x0 = dx * math.floor(coordinate_range[0]/dx+1e-10)
        x1 = dx * math.ceil(coordinate_range[1]/dx-1e-10)
    else:
        x0 = dx * math.ceil(coordinate_range[0]/dx-1e-10)
        x1 = dx * math.floor(coordinate_range[1]/dx+1e-10)
    if pi_format:
        return (x0*math.pi, dx*math.pi, x1*math.pi)
    return [x0, dx, x1]

def find_log_positions(r):
    # argument r could have
    #   three arguments if user supplied
    #   two arguments if not
    # each range 10^j -> 10^j+1 could have 1, 2, 5, 10, or 1/n lines
    x0 = np.log10(r[0])
    x1 = np.log10(r[-1])
    if len(r) == 3:
        if r[1] < 1:
            spacing = r[1]
        elif r[1] < 2:
            spacing = 1
        elif r[1] < 4:
            spacing = 2
        elif r[1] < 7:
            spacing = 5
        else:
            spacing = 10
    else:
        width = abs(x1 - x0)
        if width < 1.5:
            spacing = 10
        elif width < 3:
            spacing = 5
        elif width < 5:
            spacing = 2
        elif width <= 10:
            spacing = 1
        else:
            spacing = 10/width

    x0 = math.floor(x0)
    x1 = math.ceil(x1)
    positions = []
    if spacing <= 1:
        gap = round(1/spacing)
        x = x0
        while x <= x1:
            positions.append(10**x)
            x += gap
    else:
        if spacing == 2:
            intermediate = [1,5]
        elif spacing == 5:
            intermediate = [1,2,4,6,8]
        elif spacing == 10:
            intermediate = [1,2,3,4,5,6,7,8,9]
        else:
            intermediate = [1]
        x = x0
        while x <= x1:
            positions += [10**x*c for c in intermediate]
            x += 1
    return positions

def find_linear_positions(r):
    N = round((r[2] - r[0]) / r[1])
    return np.linspace(r[0], r[2], N+1)

# Add a graphical element for a grid.  All the grid lines sit inside a group
def grid(element, diagram, parent, outline_status):
    basis = element.get('basis')
    if basis is not None:
        grid_with_basis(element, diagram, parent, basis, outline_status)
        return

    thickness = element.get('thickness', '1')
    stroke = element.get('stroke', r'#ccc')
    grid = ET.SubElement(parent, 'g',
                         attrib={
                             'id': element.get('id', 'grid'),
                             'stroke': stroke,
                             'stroke-width': thickness
                         }
                         )

    util.cliptobbox(grid, element, diagram)

    bbox = diagram.bbox()
    spacings = element.get('spacings', None)
    h_pi_format = element.get('h-pi-format', 'no') == 'yes'
    v_pi_format = element.get('v-pi-format', 'no') == 'yes'
    
    coordinates = element.get('coordinates', 'cartesian')
    scales = diagram.get_scales()
    hspacings_set = False
    if spacings is not None:
        try:
            rx, ry = un.valid_eval(spacings)
            if scales[0] == 'log':
                x_positions = find_log_positions(rx)
            else:
                x_positions = find_linear_positions(rx)
            if scales[1] == 'log':
                y_positions = find_log_positions(ry)
            else:
                y_positions = find_linear_positions(ry)

            hspacings_set = True
        except:
            log.error(f"Error in <grid> parsing spacings={element.get('spacings')}")
            return
    else:
        rx = element.get('hspacing')
        if rx is None:
            if scales[0] == 'log':
                x_positions = find_log_positions((bbox[0], bbox[2]))
            else:
                rx = find_gridspacing((bbox[0], bbox[2]), h_pi_format)
                x_positions = find_linear_positions(rx)
        else:
            rx = un.valid_eval(rx)
            if scales[0] == 'log':
                x_positions = find_log_positions(rx)
            else:
                x_positions = find_linear_positions(rx)
            hspacings_set = True

        if coordinates == 'polar':
            ry = [0, math.pi/6, 2*math.pi]
        else:
            ry = element.get('vspacing')
            if ry is None:
                if scales[1] == 'log':
                    y_positions = find_log_positions((bbox[1], bbox[3]))
                else:
                    ry = find_gridspacing((bbox[1], bbox[3]), v_pi_format)
                    y_positions = find_linear_positions(ry)
            else:
                ry = un.valid_eval(ry)
                if scales[1] == 'log':
                    y_positions = find_log_positions(ry)
                else:
                    y_positions = find_linear_positions(ry)

    if coordinates == 'polar':
        id = diagram.get_clippath()
        grid.set('clip-path', r'url(#{})'.format(id))
        
        bbox = list(diagram.bbox())
        endpoints = []
        for _ in range(4):
            endpoints.append(bbox[:2])
            bbox = bbox[1:] + [bbox[0]]
        R = max([math_util.length(p) for p in endpoints])
        if hspacings_set:
            R = rx[2]
        r = rx[1]
        N = 100
        dt = 2*math.pi/N
        while r <= R:
            circle = ET.SubElement(grid, 'path')
            t = 0
            cmds = ['M']
            point = diagram.transform([r*math.cos(t), r*math.sin(t)])
            cmds.append(util.pt2str(point))
            for _ in range(N):
                t += dt
                cmds.append('L')
                point = diagram.transform([r*math.cos(t), r*math.sin(t)])
                cmds.append(util.pt2str(point))
            cmds.append('Z')
            circle.set('d', ' '.join(cmds))
            circle.set('fill', 'none')
            r += rx[1]

        if element.get('spacing-degrees', 'no') == 'yes':
            ry = [math.radians(t) for t in ry]
        t = ry[0]
        while t <= ry[2]:
            direction = np.array([math.cos(t), math.sin(t)])
            intersection_times = []
            vert, horiz =  np.isclose(direction, np.array([0,0]))
            if not vert:
                intersection_times.append(bbox[0]/direction[0])
                intersection_times.append(bbox[2]/direction[0])
            if not horiz:
                intersection_times.append(bbox[1]/direction[1])
                intersection_times.append(bbox[3]/direction[1])
            intersection_time = max(intersection_times)
            if hspacings_set:
                intersection_time = R
            if intersection_time > 0:
                line_el = ET.SubElement(grid, 'line')
                start = diagram.transform((0,0))
                end = diagram.transform(intersection_time*direction)
                line_el.set('x1', util.float2str(start[0]))
                line_el.set('y1', util.float2str(start[1]))
                line_el.set('x2', util.float2str(end[0]))
                line_el.set('y2', util.float2str(end[1]))

            t += ry[1]
        return

    # now we'll just build a plain rectangular grid
    for x in x_positions:
        if x < bbox[0] or x > bbox[2]:
            continue
        line_el = line.mk_line((x,bbox[1]), (x,bbox[3]), diagram)
        grid.append(line_el)

    for y in y_positions:
        if y < bbox[1] or y > bbox[3]:
            continue
        line_el = line.mk_line((bbox[0], y), (bbox[2], y), diagram)
        grid.append(line_el)


# Adds both a grid and axes with spacings found automatically

def grid_axes(element, diagram, parent, outline_status):
    group = ET.SubElement(parent, 'g',
                          attrib=
                          {
                              'id': 'grid-axes'
                          }
    )

    group_annotation = ET.Element('annotation')
    group_annotation.set('ref', 'grid-axes')
    group_annotation.set('text', 'The coordinate grid and axes')
    diagram. add_default_annotation(group_annotation)

    grid(element, diagram, group, outline_status)

    annotation = ET.Element('annotation')
    annotation.set('ref', 'grid')
    annotation.set('text', 'The coordinate grid')
    group_annotation.append(annotation)

    element.set('id', 'axes')
    axes.axes(element, diagram, group, outline_status)

    annotation = ET.Element('annotation')
    annotation.set('ref', 'axes')
    annotation.set('text', 'The coordinate axes')
    group_annotation.append(annotation)

# construct a grid with a given basis
def grid_with_basis(element, diagram, parent, basis, outline_status):
    if outline_status == 'finish_outline':
        finish_outline(element, diagram, parent)
        return
    try:
        v1, v2 = un.valid_eval(basis)
    except:
        log.error(f"Error in <grid> parsing basis={basis}")
        return

    if diagram.output_format() == 'tactile':
        element.set('stroke', 'black')
    else:
        element.set('stroke', element.get('stroke', 'black'))
    element.set('thickness', element.get('thickness', '2'))

    cmds = []
    for i in range(100):
        sv = i * v1
        p1, p2 = line.infinite_line(sv, sv + v2, diagram)
        if p1 is None:
            break
        p1 = diagram.transform(p1)
        p2 = diagram.transform(p2)
        cmds.append('M ' + util.pt2str(p1))
        cmds.append('L ' + util.pt2str(p2))

    for i in range(-1, -100, -1):
        sv = i * v1
        p1, p2 = line.infinite_line(sv, sv + v2, diagram)
        if p1 is None:
            break
        p1 = diagram.transform(p1)
        p2 = diagram.transform(p2)
        cmds.append('M ' + util.pt2str(p1))
        cmds.append('L ' + util.pt2str(p2))

    for i in range(100):
        sv = i * v2
        p1, p2 = line.infinite_line(sv, sv + v1, diagram)
        if p1 is None:
            break
        p1 = diagram.transform(p1)
        p2 = diagram.transform(p2)
        cmds.append('M ' + util.pt2str(p1))
        cmds.append('L ' + util.pt2str(p2))

    for i in range(-1, -100, -1):
        sv = i * v2
        p1, p2 = line.infinite_line(sv, sv + v1, diagram)
        if p1 is None:
            break
        p1 = diagram.transform(p1)
        p2 = diagram.transform(p2)
        cmds.append('M ' + util.pt2str(p1))
        cmds.append('L ' + util.pt2str(p2))

    coords = ET.Element('path')
    diagram.add_id(coords, element.get('id'))
    util.add_attr(coords, util.get_1d_attr(element))
    coords.set('d', ' '.join(cmds))
#    coords.set('type', 'grid with basis')

    if outline_status == 'add_outline':
        diagram.add_outline(element, coords, parent)
        return

    if element.get('outline', 'no') == 'yes' or diagram.output_format() == 'tactile':
        diagram.add_outline(element, coords, parent)
        finish_outline(element, diagram, parent)
    else:
        parent.append(coords)

def finish_outline(element, diagram, parent):
    diagram.finish_outline(element,
                           element.get('stroke'),
                           element.get('thickness'),
                           'none',
                           parent
                           )
