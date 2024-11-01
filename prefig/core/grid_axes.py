import lxml.etree as ET
import math
import re
import numpy as np
from . import utilities as util
from . import user_namespace as un
from . import label
from . import line
from . import arrow
from . import CTM


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
    return (x0, dx, x1)

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
    
    if spacings is not None:
        rx, ry = un.valid_eval(spacings)
    else:
        rx = element.get('hspacing')
        if rx is None:
            rx = find_gridspacing((bbox[0], bbox[2]), h_pi_format) 
        else:
            rx = un.valid_eval(rx)

        ry = element.get('vspacing')
        if ry is None:
            ry = find_gridspacing((bbox[1], bbox[3]), v_pi_format)
        else:
            ry = un.valid_eval(ry)

    x = rx[0]
    while x <= rx[2]:
        line_el = line.mk_line((x,bbox[1]), (x,bbox[3]), diagram)
#        line_el.set('type', 'vertical grid')
        grid.append(line_el)
        x += rx[1]

    y = ry[0]
    while y <= ry[2]:
        line_el = line.mk_line((bbox[0], y), (bbox[2], y), diagram)
#        line_el.set('type', 'horizontal grid')
        grid.append(line_el)
        y += ry[1]

# Automate finding the positions where ticks and labels go
label_delta = {2: 0.2, 3: 0.5, 4: 0.5, 5: 1,
               6: 1, 7: 1, 8: 1, 9: 1, 10: 1, 11: 1,
               12: 2, 13: 2, 14: 2, 15: 2, 16: 2, 17: 2,
               18: 2, 19: 2, 20: 2}

def find_label_positions(coordinate_range, pi_format = False):
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
    if dx > 1:
        dx *= label_delta[round(2*distance)]
        dx = int(dx)
    else:
        dx *= label_delta[round(2*distance)]
    if coordinate_range[1] < coordinate_range[0]:
        dx *= -1
        x0 = dx * math.floor(coordinate_range[0]/dx+1e-10)
        x1 = dx * math.ceil(coordinate_range[1]/dx-1e-10)
    else:
        x0 = dx * math.ceil(coordinate_range[0]/dx-1e-10)
        x1 = dx * math.floor(coordinate_range[1]/dx+1e-10)
    return (x0, dx, x1)

# find a string representation of x*pi
def get_pi_text(x):
    if abs(abs(x) - 1) < 1e-10:
        if x < 0:
            return '-\pi'
        return '\pi'

    if abs(x - round(x)) < 1e-10:
        return str(round(x))+'\pi'
    if abs(4*x - round(4*x)) < 1e-10:
        num = round(4*x)
        if num == -1:
            return '-\pi/4'
        if num == 1:
            return '\pi/4'
        if num % 2 == 1:
            return str(num)+'\pi/4'
    if abs(2*x - round(2*x)) < 1e-10:
        num = round(2*x)
        if num == -1:
            return '-\pi/2'
        if num == 1:
            return '\pi/2'
        return str(num)+'\pi/2'
    if abs(3*x - round(3*x)) < 1e-10:
        num = round(3*x)
        if num == -1:
            return '-\pi/3'
        if num == 1:
            return '\pi/3'
        return str(num)+'\pi/3'
    return '{0:g}\pi'.format(x)
    

# Add a graphical element for axes.  All the axes sit inside a group
# There are a number of options to add: labels, tick marks, etc
# TODO:  rethink logic of options
# TODO:  add labels
# TODO:  handle cases where axis is outside bbox
# TODO:  Matt's request for boundaries
position_tolerance = 1e-10
def axes(element, diagram, parent, outline_status):
    stroke = element.get('stroke', 'black')
    thickness = element.get('thickness', '2')

    axes = ET.SubElement(parent, 'g',
                         attrib={
                             'id': element.get('id', 'axes'),
                             'stroke': stroke,
                             'stroke-width': thickness
                             }
    )

    util.cliptobbox(axes, element, diagram)
    ctm, bbox = diagram.ctm_bbox()

    top_labels = False
    y_axis_location = 0
    y_axis_offsets = (0,0)
    if bbox[1] * bbox[3] >= 0:
        if bbox[3] <= 0:
            top_labels = True
            y_axis_location = bbox[3]
            if bbox[3] < 0:
                y_axis_offsets = (0,-5)
        else:
            if abs(bbox[1]) > 1e-10:
                y_axis_location = bbox[1]
                y_axis_offsets = (5,0)
    y_axis_offsets = np.array(y_axis_offsets)

    right_labels = False
    x_axis_location = 0
    x_axis_offsets = (0,0)
    if bbox[0] * bbox[2] >= 0:
        if bbox[2] <= 0:
            right_labels = True
            x_axis_location = bbox[2]
            if bbox[2] < 0:
                x_axis_offsets = (0,-10)
        else:
            if abs(bbox[0]) > 1e-10:
                x_axis_location = bbox[0]
                x_axis_offsets = (10,0)
    x_axis_offsets = np.array(x_axis_offsets)
    
    decorations = element.get('decorations', 'yes')

    left_axis = diagram.transform((bbox[0], y_axis_location))
    right_axis = diagram.transform((bbox[2], y_axis_location))

    h_line_el = line.mk_line(left_axis,
                             right_axis,
                             diagram,
                             endpoint_offsets = x_axis_offsets,
                             user_coords = False)
    h_line_el.set('stroke', stroke)
#    h_line_el.set('type', 'horizontal axis')
    h_line_el.set('stroke-width', thickness)
    axes.append(h_line_el)

    bottom_axis = diagram.transform((x_axis_location, bbox[1]))
    top_axis = diagram.transform((x_axis_location, bbox[3]))

    v_line_el = line.mk_line(bottom_axis,
                             top_axis,
                             diagram,
                             endpoint_offsets = y_axis_offsets,
                             user_coords = False)
    v_line_el.set('stroke', stroke)
#    v_line_el.set('type', 'vertical axis')
    v_line_el.set('stroke-width', thickness)
    axes.append(v_line_el)

    arrows = int(element.get('arrows', '0'))
    if arrows > 0:
        arrow.add_arrowhead_to_path(diagram, 'marker-end', h_line_el)
        arrow.add_arrowhead_to_path(diagram, 'marker-end', v_line_el)
    if arrows > 1:
        arrow.add_arrowhead_to_path(diagram, 'marker-start', h_line_el)
        arrow.add_arrowhead_to_path(diagram, 'marker-start', v_line_el)

    if element.get('labels') == 'no':
        return

    hticks = element.get('hticks', None)
    vticks = element.get('vticks', None)

    h_pi_format = element.get('h-pi-format', 'no') == 'yes'
    v_pi_format = element.get('v-pi-format', 'no') == 'yes'
    
    hlabels = element.get('hlabels')
    if hlabels is None:
        hlabels = find_label_positions((bbox[0], bbox[2]),
                                       pi_format = h_pi_format)
    else:
        hlabels = un.valid_eval(hlabels)
        if h_pi_format:
            hlabels = 1/math.pi * hlabels

    g_hticks = ET.SubElement(axes, 'g',
                             attrib={
#                                 'type': 'horizontal ticks'
                             }
                             )
    diagram.add_id(g_hticks)

    if diagram.output_format() == 'tactile':
        ticksize = (18, 0)
    else:
        ticksize = (3, 3)
    if hticks is not None:
        hticks = un.valid_eval(hticks)
        x = hticks[0]
        tick_direction = 1
        if top_labels:
            tick_direction = -1
        while x <= hticks[2]:
            if any([abs(x-p) < position_tolerance for p in [bbox[0], bbox[2]]]):
                x += hticks[1]
                continue
            p = diagram.transform((x,y_axis_location))
            line_el = line.mk_line((p[0], p[1]+tick_direction*ticksize[0]),
                                   (p[0], p[1]-tick_direction*ticksize[1]),
                                   diagram,
                                   user_coords=False)
#            line_el.set('type', 'tick on horizontal axis')
            g_hticks.append(line_el)
            x += hticks[1]

    h_scale = 1
    if h_pi_format:
        h_scale = math.pi
    if decorations == 'yes' or element.get('hlabels', None) is not None:
        x = hlabels[0]
        tick_direction = 1
        if top_labels:
            tick_direction = -1
        while x <= hlabels[2]:
            if any([abs(x*h_scale-p) < position_tolerance for p in [bbox[0], bbox[2],0]]):
                x += hlabels[1]
                continue

            xlabel = ET.Element('label')
            math_element = ET.SubElement(xlabel, 'm')
            math_element.text = r'\text{'+'{0:g}'.format(x)+'}'
            if h_pi_format:
                math_element.text = get_pi_text(x)

            xlabel.set('p', '({},{})'.format(x*h_scale, y_axis_location))
            if diagram.output_format() == 'tactile':
                if top_labels:
                    xlabel.set('alignment', 'hat')
                    xlabel.set('offset', '(0,0)')
                else:
                    xlabel.set('alignment', 'ha')
                    xlabel.set('offset', '(0,0)')
            else:
                if top_labels:
                    xlabel.set('alignment', 'north')
                    xlabel.set('offset', '(0,7)')
                else:
                    xlabel.set('alignment', 'south')
                    xlabel.set('offset', '(0,-7)')
            xlabel.set('clear-background', 'no')
            label.label(xlabel, diagram, parent, outline_status)

            p = diagram.transform((x*h_scale,y_axis_location))
            line_el = line.mk_line((p[0], p[1]+tick_direction*ticksize[0]),
                                   (p[0], p[1]-tick_direction*ticksize[1]),
                                   diagram,
                                   user_coords=False)
#            line_el.set('type', 'tick on horizontal axis')
            g_hticks.append(line_el)

            x += hlabels[1]

    vlabels = element.get('vlabels')
    if vlabels is None:
        vlabels = find_label_positions((bbox[1], bbox[3]),
                                       pi_format = v_pi_format)
    else:
        vlabels = un.valid_eval(vlabels)
        if v_pi_format:
            vlabels = 1/math.pi * vlabels
            
#    g_vticks = ET.SubElement(axes, 'g', attrib={
#                              'type': 'vertical ticks'
#                              }
#    )
    g_vticks = ET.SubElement(axes, 'g')
    diagram.add_id(g_vticks)

    if vticks is not None:
        vticks = un.valid_eval(vticks)
        y = vticks[0]
        tick_direction = 1
        if right_labels:
            tick_direction = -1
        while y <= vticks[2]:
            if any([abs(y-p) < position_tolerance for p in [bbox[1], bbox[3]]]):
                y += vticks[1]
                continue
            p = diagram.transform((x_axis_location, y))
            line_el = line.mk_line((p[0]-tick_direction*ticksize[0], p[1]),
                                   (p[0]+tick_direction*ticksize[1], p[1]),
                                   diagram,
                                   user_coords=False)
#            line_el.set('type', 'tick on vertical axis')
            g_vticks.append(line_el)
            y += vticks[1]

    v_scale = 1
    if v_pi_format:
        v_scale = math.pi
    if decorations == 'yes' or element.get('vlabels', None) is not None:
        y = vlabels[0]
        tick_direction = 1
        if right_labels:
            tick_direction = -1
        while y <= vlabels[2]:
            if any([abs(y*v_scale-p) < position_tolerance for p in [bbox[1], bbox[3], 0]]):
                y += vlabels[1]
                continue

            ylabel = ET.Element('label')
            math_element = ET.SubElement(ylabel, 'm')
            math_element.text = r'\text{'+'{0:g}'.format(y)+'}'
            if v_pi_format:
                math_element.text = get_pi_text(y)
            # process as a math number
            ylabel.set('p', '({},{})'.format(x_axis_location, y*v_scale))

            if diagram.output_format() == 'tactile':
                if right_labels:
                    ylabel.set('alignment', 'east')
                    ylabel.set('offset', '(25, 0)')
                else:
                    ylabel.set('alignment', 'va')
                    ylabel.set('offset', '(-25, 0)')
            else:
                if right_labels:
                    ylabel.set('alignment', 'east')
                    ylabel.set('offset', '(7,0)')
                else:
                    ylabel.set('alignment', 'west')
                    ylabel.set('offset', '(-7,0)')

            ylabel.set('clear-background', 'no')
            label.label(ylabel, diagram, parent, outline_status)
            p = diagram.transform((x_axis_location, y*v_scale))
            line_el = line.mk_line((p[0]-tick_direction*ticksize[0], p[1]),
                                   (p[0]+tick_direction*ticksize[1], p[1]),
                                   diagram,
                                   user_coords=False)
#            line_el.set('type', 'tick on vertical axis')
            g_vticks.append(line_el)
            y += vlabels[1]

    xlabel = element.get('xlabel')
    if xlabel is not None:
        el = ET.Element('label')
        math_element = ET.SubElement(el, 'm')
        math_element.text = xlabel
        el.set('clear-background', 'no')
        el.set('p', '({},{})'.format(bbox[2], y_axis_location))
        el.set('alignment', 'xl')
        if arrows > 0:
            if diagram.output_format() == 'tactile':
                el.set('offset', '(-6,6)')
            else:
                el.set('offset', '(-2,2)')
        label.label(el, diagram, parent, outline_status)

    ylabel = element.get('ylabel')
    if ylabel is not None:
        el = ET.Element('label')
        math_element = ET.SubElement(el, 'm')
        math_element.text = ylabel
        el.set('clear-background', 'no')
        el.set('p', '({},{})'.format(x_axis_location, bbox[3]))
        el.set('alignment', 'se')
        if arrows > 0:
            el.set('offset', '(2,-2)')
        label.label(el, diagram, parent, outline_status)


# Adds both a grid and axes with spacings found automatically

def grid_axes(element, diagram, parent, outline_status):
    group = ET.SubElement(parent, 'g',
                          attrib=
                          {
                              'id': 'grid-axes'
                          }
    )

    group_annotation = ET.Element('annotation')
    group_annotation.set('id', 'grid-axes')
    group_annotation.set('text', 'The coordinate grid and axes')
    diagram. add_default_annotation(group_annotation)

    grid(element, diagram, group, outline_status)

    annotation = ET.Element('annotation')
    annotation.set('id', 'grid')
    annotation.set('text', 'The coordinate grid')
    group_annotation.append(annotation)

    el = ET.Element('axes')
    el.set('id', 'axes')
    if element.get('xlabel') is not None:
        el.set('xlabel', element.get('xlabel'))
    if element.get('ylabel') is not None:
        el.set('ylabel', element.get('ylabel'))
    if element.get('decorations') is not None:
        el.set('decorations', element.get('decorations'))
    if element.get('hlabels') is not None:
        el.set('hlabels', element.get('hlabels'))
    if element.get('vlabels') is not None:
        el.set('vlabels', element.get('vlabels'))
    if element.get('h-pi-format') is not None:
        el.set('h-pi-format', element.get('h-pi-format'))
    if element.get('v-pi-format') is not None:
        el.set('v-pi-format', element.get('v-pi-format'))
        
    axes(el, diagram, group, outline_status)

    annotation = ET.Element('annotation')
    annotation.set('id', 'axes')
    annotation.set('text', 'The coordinate axes')
    group_annotation.append(annotation)

# construct a grid with a given basis
def grid_with_basis(element, diagram, parent, basis, outline_status):
    if outline_status == 'finish_outline':
        finish_outline(element, diagram, parent)
        return
    v1, v2 = un.valid_eval(basis)

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
