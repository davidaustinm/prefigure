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

log = logging.getLogger('prefigure')

# These tags can appear in an <axes> or <grid-axes>
axes_tags = {'xlabel', 'ylabel'}

def is_axes_tag(tag):
    return tag in axes_tags

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
            spacing = 2
        elif width <= 10:
            spacing = 1
        else:
            spacing = 5/width

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

# find a string representation of x*pi
def get_pi_text(x):
    if abs(abs(x) - 1) < 1e-10:
        if x < 0:
            return r'-\pi'
        return r'\pi'

    if abs(x - round(x)) < 1e-10:
        return str(round(x))+r'\pi'
    if abs(4*x - round(4*x)) < 1e-10:
        num = round(4*x)
        if num == -1:
            return r'-\pi/4'
        if num == 1:
            return r'\pi/4'
        if num % 2 == 1:
            return str(num)+r'\pi/4'
    if abs(2*x - round(2*x)) < 1e-10:
        num = round(2*x)
        if num == -1:
            return r'-\pi/2'
        if num == 1:
            return r'\pi/2'
        return str(num)+r'\pi/2'
    if abs(3*x - round(3*x)) < 1e-10:
        num = round(3*x)
        if num == -1:
            return r'-\pi/3'
        if num == 1:
            return r'\pi/3'
        return str(num)+r'\pi/3'
    return r'{0:g}\pi'.format(x)
    

# Add a graphical element for axes.  All the axes sit inside a group
# There are a number of options to add: labels, tick marks, etc

class Axes():
    def __init__(self, element, diagram, parent):
        self.tactile = diagram.output_format() == "tactile"
        self.stroke = element.get('stroke', 'black')
        self.thickness = element.get('thickness', '2')

        self.axes = ET.SubElement(parent, 'g',
                                  attrib={
                                      'id': element.get('id', 'axes'),
                                      'stroke': self.stroke,
                                      'stroke-width': self.thickness
                                  }
                                  )
        util.cliptobbox(self.axes, element, diagram)
        
        # which axes are we asked to build
        self.axes_attribute = element.get("axes", None)
        if self.axes_attribute == "all":
            self.axes_attribute = None
            element.attrib.pop("axes")
        self.horizontal_axis = element.get("axes", "horizontal") == "horizontal"
        self.vertical_axis = element.get("axes", "vertical") == "vertical"

        self.clear_background = element.get('clear-background', 'no')
        self.decorations = element.get('decorations', 'yes')
        self.h_pi_format = element.get('h-pi-format', 'no') == 'yes'
        self.v_pi_format = element.get('v-pi-format', 'no') == 'yes'
        if self.tactile:
            self.ticksize = (18, 0)
        else:
            self.ticksize = (3, 3)
            if element.get('tick-size', None) is not None:
                self.ticksize = un.valid_eval(element.get('tick-size'))
                if not isinstance(self.ticksize, np.ndarray):
                    self.ticksize = [self.ticksize, self.ticksize]
        
        self.bbox = diagram.bbox()
        self.position_tolerance = 1e-10

        try:
            self.arrows = int(element.get('arrows', '0'))
        except:
            log.error(f"Error in <axes> parsing arrows={element.get('arrows')}")
            self.arrows = 0

        self.position_axes(element, diagram)
        self.apply_axis_labels(element, diagram, parent)

        if element.get('bounding-box', 'no') == 'yes':
            rect = ET.SubElement(self.axes, 'rect')
            ul = diagram.transform([self.bbox[0], self.bbox[3]])
            lr = diagram.transform([self.bbox[2], self.bbox[1]])
            w = lr[0] - ul[0]
            h = lr[1] - ul[1]
            rect.set('x', util.float2str(ul[0]))
            rect.set('y', util.float2str(ul[1]))
            rect.set('width', util.float2str(w))
            rect.set('height', util.float2str(h))
            rect.set('fill', 'none')

        if self.horizontal_axis:
            self.add_h_axis(element, diagram, self.arrows)
            self.h_tick_group = ET.Element('g')
            self.horizontal_ticks(element, diagram)
            self.h_labels(element, diagram, parent)
        if self.vertical_axis:
            self.add_v_axis(element, diagram, self.arrows)
            self.v_tick_group = ET.Element('g')
            self.vertical_ticks(element, diagram)
            self.v_labels(element, diagram, parent)


    def position_axes(self, element, diagram):
        scales = diagram.get_scales()
        self.y_axis_location = 0
        self.y_axis_offsets = (0,0)
        self.h_zero_include = False
        self.top_labels = False
        if float(self.bbox[1]) * float(self.bbox[3]) >= 0:
            if self.bbox[3] <= 0:
                self.top_labels = True
                self.y_axis_location = self.bbox[3]
                if self.bbox[3] < 0:
                    self.y_axis_offsets = (0,-5)
            else:
                if abs(self.bbox[1]) > 1e-10:
                    self.y_axis_location = self.bbox[1]
                    self.y_axis_offsets = (5,0)

        h_frame = element.get('h-frame', None)
        if h_frame == 'bottom':
            self.y_axis_location = self.bbox[1]
            self.y_axis_offsets = (0,0)
            self.h_zero_include = True
        if h_frame == 'top':
            self.y_axis_location = self.bbox[3]
            self.y_axis_offsets = (0,0)
            self.h_zero_include = True
            self.top_labels = True

        if scales[1] == 'log':
            self.y_axis_offsets = (0,0)
            self.h_zero_include = True
        self.y_axis_offsets = np.array(self.y_axis_offsets)

        # which locations will not get ticks or labels 
        self.h_exclude = []
        self.h_zero_label = element.get("h-zero-label", "no") == "yes"
        if (
                not self.h_zero_include and
                self.axes_attribute != 'horizontal' and
                not self.h_zero_label
        ):
            self.h_exclude.append(0)

        # ticks move up when the horizontal axis is on top
        self.h_tick_direction = 1
        if self.top_labels:
            self.h_tick_direction = -1
        

        self.x_axis_location = 0
        self.x_axis_offsets = (0,0)
        self.v_zero_include = False
        self.right_labels = False
        if float(self.bbox[0]) * float(self.bbox[2]) >= 0:
            if self.bbox[2] <= 0:
                self.right_labels = True
                self.x_axis_location = self.bbox[2]
                if self.bbox[2] < 0:
                    self.x_axis_offsets = (0,-10)
            else:
                if abs(self.bbox[0]) > 1e-10:
                    self.x_axis_location = self.bbox[0]
                    self.x_axis_offsets = (10,0)

        v_frame = element.get('v-frame', None)
        if v_frame == 'left':
            self.x_axis_location = self.bbox[0]
            self.x_axis_offsets = (0,0)
            self.v_zero_include = True
        if v_frame == 'right':
            self.x_axis_location = self.bbox[2]
            self.x_axis_offsets = (0,0)
            self.v_zero_include = True
            self.right_labels = True

        if scales[1] == 'log':
            self.x_axis_offsets = (0,0)
            self.v_zero_include = True

        self.x_axis_offsets = np.array(self.x_axis_offsets)

        # which locations will not get ticks or labels 
        self.v_exclude = []
        self.v_zero_label = element.get("v-zero-label", "no") == "yes"
        if (
                not self.v_zero_include and
                self.axes_attribute != 'vertical' and
                not self.v_zero_label
        ):
            self.v_exclude.append(0)

        # ticks move right when the vertical axis is on the right
        self.v_tick_direction = 1
        if self.right_labels:
            self.v_tick_direction = -1
        
        
    def apply_axis_labels(self, element, diagram, parent):
        # process xlabel and ylabel

        xlabel = element.get('xlabel')
        if xlabel is not None:
            el = ET.Element('label')
            math_element = ET.SubElement(el, 'm')
            math_element.text = xlabel
            el.set('clear-background', 'no')
            el.set('p', '({},{})'.format(self.bbox[2],
                                         self.y_axis_location))
            el.set('alignment', 'xl')
            if self.arrows > 0:
                if self.tactile:
                    el.set('offset', '(-6,6)')
                else:
                    el.set('offset', '(-2,2)')

            el.set('clear-background', self.clear_background)
            label.label(el, diagram, parent, outline_status=None)

        ylabel = element.get('ylabel')
        if ylabel is not None:
            el = ET.Element('label')
            math_element = ET.SubElement(el, 'm')
            math_element.text = ylabel
            el.set('clear-background', 'no')
            el.set('p', '({},{})'.format(self.x_axis_location,
                                         self.bbox[3]))
            el.set('alignment', 'se')
            if self.arrows > 0:
                el.set('offset', '(2,-2)')

            el.set('clear-background', self.clear_background)
            label.label(el, diagram, parent, outline_status=None)

        
        for child in element:
            if child.tag == "xlabel":
                child.tag = "label"
                child.set("user-coords", "no")
                anchor = diagram.transform((self.bbox[2], self.y_axis_location))
                child.set("anchor", util.pt2str(anchor, spacer=","))
                if child.get("alignment", None) is None:
                    child.set("alignment", "east")
                if child.get("offset", None) is None:
                    if self.arrows > 0:
                        child.set("offset", "(2,0)")
                    else:
                        child.set("offset", "(1,0)")

                label.label(child, diagram, parent)
                continue
            
            if child.tag == "ylabel":
                child.tag = "label"
                child.set("user-coords", "no")
                anchor = diagram.transform((self.x_axis_location, self.bbox[3]))
                child.set("anchor", util.pt2str(anchor, spacer=","))
                if child.get("alignment", None) is None:
                    child.set("alignment", "north")
                    if child.get("offset", None) is None:
                        if self.arrows > 0:
                            child.set("offset", "(0,2)")
                        else:
                            child.set("offset", "(0,1)")

                label.label(child, diagram, parent)
                continue
            log.info(f"{child.tag} element is not allowed inside a <label>")
            continue

    def add_h_axis(self, element, diagram, arrows):
        left_axis = diagram.transform((self.bbox[0], self.y_axis_location))
        right_axis = diagram.transform((self.bbox[2], self.y_axis_location))

        h_line_el = line.mk_line(left_axis,
                                 right_axis,
                                 diagram,
                                 endpoint_offsets = self.x_axis_offsets,
                                 user_coords = False)
        h_line_el.set('stroke', self.stroke)
        h_line_el.set('stroke-width', self.thickness)
        if arrows > 0:
            arrow.add_arrowhead_to_path(diagram, 'marker-end', h_line_el)
        if arrows > 1:
            arrow.add_arrowhead_to_path(diagram, 'marker-start', h_line_el)
        self.axes.append(h_line_el)

    def add_v_axis(self, element, diagram, arrows):
        bottom_axis = diagram.transform((self.x_axis_location, self.bbox[1]))
        top_axis = diagram.transform((self.x_axis_location, self.bbox[3]))

        v_line_el = line.mk_line(bottom_axis,
                                 top_axis,
                                 diagram,
                                 endpoint_offsets = self.y_axis_offsets,
                                 user_coords = False)
        v_line_el.set('stroke', self.stroke)
        v_line_el.set('stroke-width', self.thickness)
        if arrows > 0:
            arrow.add_arrowhead_to_path(diagram, 'marker-end', v_line_el)
        if arrows > 1:
            arrow.add_arrowhead_to_path(diagram, 'marker-start', v_line_el)
        self.axes.append(v_line_el)


    def horizontal_ticks(self, element, diagram):
        hticks = element.get('hticks', None)
        if hticks is None:
            return

        self.axes.append(self.h_tick_group)
        diagram.add_id(self.h_tick_group)

        try:
            hticks = un.valid_eval(hticks)
        except:
            log.error(f"Error in <axes> parsing hticks={hticks}")
            return

        scale = diagram.get_scales()[0]
        if scale == 'log':
            x_positions = find_log_positions(hticks)
        else:
            N = round( (hticks[2] - hticks[0]) / hticks[1])
            x_positions = np.linspace(hticks[0], hticks[2], N+1)

        for x in x_positions:
            if x < self.bbox[0] or x > self.bbox[2]:
                continue
            if scale == 'log':
                avoid = [abs(np.log10(x) - np.log10(p)) for p in self.h_exclude]
            else:
                avoid = [abs(x - p) for p in self.h_exclude]
            if any([dist < self.position_tolerance for dist in avoid]):
                continue

            p = diagram.transform((x,self.y_axis_location))
            line_el = line.mk_line((p[0],
                                    p[1]+self.h_tick_direction*self.ticksize[0]),
                                   (p[0],
                                    p[1]-self.h_tick_direction*self.ticksize[1]),
                                   diagram,
                                   user_coords=False)
            self.h_tick_group.append(line_el)
                    

    def vertical_ticks(self, element, diagram):
        vticks = element.get('vticks', None)
        if vticks is None:
            return

        self.axes.append(self.v_tick_group)
        diagram.add_id(self.v_tick_group)

        try:
            vticks = un.valid_eval(vticks)
        except:
            log.error(f"Error in <axes> parsing vticks={vticks}")
            return

        scale = diagram.get_scales()[1]
        if scale == 'log':
            y_positions = find_log_positions(vticks)
        else:
            N = round( (vticks[2] - vticks[0]) / vticks[1])
            y_positions = np.linspace(vticks[0], vticks[2], N+1)

        for y in y_positions:
            if y < self.bbox[1] or y > self.bbox[3]:
                continue
            if scale == 'log':
                avoid = [abs(np.log10(y) - np.log10(p)) for p in self.v_exclude]
            else:
                avoid = [abs(y - p) for p in self.v_exclude]
            if any([dist < self.position_tolerance for dist in avoid]):
                continue

            p = diagram.transform((self.x_axis_location, y))
            line_el = line.mk_line((p[0]-self.v_tick_direction*self.ticksize[0],
                                    p[1]),
                                   (p[0]+self.v_tick_direction*self.ticksize[1],
                                    p[1]),
                                   diagram,
                                   user_coords=False)
            self.v_tick_group.append(line_el)
            y += vticks[1]
            

    def h_labels(self, element, diagram, parent):
        hlabels = element.get('hlabels')
        if self.decorations == 'no' and hlabels is None:
            return

        h_exclude = self.h_exclude[:]

        scale = diagram.get_scales()[0]
        if hlabels is None:
            if scale == 'log':
                h_positions = find_log_positions((self.bbox[0],
                                                       self.bbox[2]))
            else:
                hlabels = find_label_positions((self.bbox[0], self.bbox[2]),
                                               pi_format = self.h_pi_format)
                N = round( (hlabels[2] - hlabels[0]) / hlabels[1])
                h_positions = np.linspace(hlabels[0], hlabels[2], N+1)
            h_exclude += [self.bbox[0], self.bbox[2]]
        else:
            try:
                hlabels = un.valid_eval(hlabels)
                if scale == 'log':
                    h_positions = find_log_positions(hlabels)
                else:
                    N = round( (hlabels[2] - hlabels[0]) / hlabels[1])
                    h_positions = np.linspace(hlabels[0], hlabels[2], N+1)
            except:
                log.error(f"Error in <axes> parsing hlabels={hlabels}")
                return
            if self.h_pi_format:
                hlabels = 1/math.pi * hlabels

        h_scale = 1
        if self.h_pi_format:
            h_scale = math.pi

        if self.h_tick_group.getparent() is None:
            self.axes.append(self.h_tick_group)

        if self.h_zero_label:
            try:
                h_exclude.remove(0)
            except:
                pass

        commas = element.get("label-commas", "yes") == "yes"

        for x in h_positions:
            if x < self.bbox[0] or x > self.bbox[2]:
                continue
            if scale == 'log':
                avoid = [abs(np.log10(x*h_scale) - np.log10(p)) for p in h_exclude]
            else:
                avoid = [abs(x*h_scale - p) for p in h_exclude]
            if any([dist < self.position_tolerance for dist in avoid]):
                continue

            xlabel = ET.Element('label')
            math_element = ET.SubElement(xlabel, 'm')
            if scale == 'log':
                x_text = np.log10(x)
                frac = x_text % 1.0
                prefix = round(10**frac)
                if prefix != 1:
                    x_exp = math.floor(x_text)
                    prefix = str(prefix)
                    begin = prefix + r'\cdot10^{'
                else:
                    x_exp = x_text
                    begin = r'10^{'
                math_element.text = begin+'{0:g}'.format(x_exp)+'}'
                xlabel.set('scale', '0.8')
            else:
                #math_element.text = r'\text{'+'{0:g}'.format(x)+'}'
                math_element.text = label_text(x, commas)
            if self.h_pi_format:
                math_element.text = get_pi_text(x)

            xlabel.set('p', '({},{})'.format(x*h_scale, self.y_axis_location))
            if self.tactile:
                if self.top_labels:
                    xlabel.set('alignment', 'hat')
                    xlabel.set('offset', '(0,0)')
                else:
                    xlabel.set('alignment', 'ha')
                    xlabel.set('offset', '(0,0)')
            else:
                if self.top_labels:
                    xlabel.set('alignment', 'north')
                    xlabel.set('offset', '(0,7)')
                else:
                    xlabel.set('alignment', 'south')
                    xlabel.set('offset', '(0,-7)')

            xlabel.set('clear-background', self.clear_background)
            label.label(xlabel, diagram, parent, outline_status=None)

            p = diagram.transform((x*h_scale,self.y_axis_location))
            line_el = line.mk_line((p[0],
                                    p[1]+self.h_tick_direction*self.ticksize[0]),
                                   (p[0],
                                    p[1]-self.h_tick_direction*self.ticksize[1]),
                                   diagram,
                                   user_coords=False)

            self.h_tick_group.append(line_el)

    def v_labels(self, element, diagram, parent):
        vlabels = element.get('vlabels')
        if self.decorations == "no" and vlabels is None:
            return

        v_exclude = self.v_exclude[:]

        scale = diagram.get_scales()[1]
        if vlabels is None:
            if scale == 'log':
                v_positions = find_log_positions((self.bbox[1], self.bbox[3]))
            else:
                vlabels = find_label_positions((self.bbox[1], self.bbox[3]),
                                               pi_format = self.v_pi_format)
                N = round( (vlabels[2] - vlabels[0]) / vlabels[1])
                v_positions = np.linspace(vlabels[0], vlabels[2], N+1)

            v_exclude += [self.bbox[1], self.bbox[3]]
        else:
            try:
                vlabels = un.valid_eval(vlabels)
                if scale == 'log':
                    v_positions = find_log_positions(vlabels)
                else:
                    N = round( (vlabels[2] - vlabels[0]) / vlabels[1])
                    v_positions = np.linspace(vlabels[0], vlabels[2], N+1)
            except:
                log.error(f"Error in <axes> parsing vlabels={vlabels}")
                return

            if self.v_pi_format:
                vlabels = 1/math.pi * vlabels
            
        v_scale = 1
        if self.v_pi_format:
            v_scale = math.pi

        if self.v_tick_group.getparent() is None:
            self.axes.append(self.v_tick_group)

        if element.get("v-zero-label", "no") == "yes":
            try:
                v_exclude.remove(0)
            except:
                pass

        commas = element.get("label-commas", "yes") == "yes"

        for y in v_positions:
            if y < self.bbox[1] or y > self.bbox[3]:
                continue

            if scale == 'log':
                avoid = [abs(np.log10(y*v_scale) - np.log10(p)) for p in v_exclude]
            else:
                avoid = [abs(y*v_scale - p) for p in v_exclude]
            if any([dist < self.position_tolerance for dist in avoid]):
                continue

            ylabel = ET.Element('label')
            math_element = ET.SubElement(ylabel, 'm')
            if scale == 'log':
                y_text = np.log10(y)
                frac = y_text % 1.0
                prefix = round(10**frac)
                if prefix != 1:
                    y_exp = math.floor(y_text)
                    prefix = str(prefix)
                    begin = prefix + r'\cdot10^{'
                else:
                    y_exp = y_text
                    begin = r'10^{'
                math_element.text = begin+'{0:g}'.format(y_exp)+'}'
                ylabel.set('scale', '0.8')
            else:
                #math_element.text = r'\text{'+'{0:g}'.format(y)+'}'
                math_element.text = label_text(y, commas)
            if self.v_pi_format:
                math_element.text = get_pi_text(y)
            # process as a math number
            ylabel.set('p', '({},{})'.format(self.x_axis_location, y*v_scale))

            if self.tactile:
                if self.right_labels:
                    ylabel.set('alignment', 'east')
                    ylabel.set('offset', '(25, 0)')
                else:
                    ylabel.set('alignment', 'va')
                    ylabel.set('offset', '(-25, 0)')
            else:
                if self.right_labels:
                    ylabel.set('alignment', 'east')
                    ylabel.set('offset', '(7,0)')
                else:
                    ylabel.set('alignment', 'west')
                    ylabel.set('offset', '(-7,0)')

            ylabel.set('clear-background', self.clear_background)
            label.label(ylabel, diagram, parent, outline_status=None)
            p = diagram.transform((self.x_axis_location, y*v_scale))
            line_el = line.mk_line((p[0]-self.v_tick_direction*self.ticksize[0],
                                    p[1]),
                                   (p[0]+self.v_tick_direction*self.ticksize[1],
                                    p[1]),
                                   diagram,
                                   user_coords=False)
            self.v_tick_group.append(line_el)

def label_text(x, commas):
    # we'll construct a text representation of x
    # maybe it's simple
    if x < 0:
        prefix = '-'
        x = abs(x)
    else:
        prefix = ''
    text = '{0:g}'.format(x)

    # but it could be in exponential notation
    if text.find('e') >= 0:
        integer = math.floor(x)
        fraction = x - integer
        if fraction > 1e-14:
            suffix = '{0:g}'.format(fraction)[1:]
        else:
            suffix = ''
        int_part = ''
        while integer >= 10:
            int_part = str(integer % 10) + int_part
            integer = int(integer / 10)
        int_part = str(integer) + int_part
        text = int_part + suffix

    if not commas:
        return r'\text{' + prefix + text + r'}'

    period = text.find('.')
    if period < 0:
        suffix = ''
    else:
        suffix = text[period:]
        text = text[:period]
    while len(text) > 3:
        suffix = ',' + text[-3:] + suffix
        text = text[:-3]
    text = text + suffix
    return r'\text{' + prefix + text + r'}'

def tick_mark(element, diagram, parent, outline_status):
    # tick marks are in the background so there's no need to worry
    # about the outline_status
    if outline_status == 'finish_outline':
        return

    axis = element.get('axis', 'horizontal')
    tactile = diagram.output_format() == 'tactile'
    location = un.valid_eval(element.get('location', '0'))
    y_axis_location = 0
    x_axis_location = 0
    top_labels = False
    right_labels = False
    if axes_object is not None:
        y_axis_location = axes_object.y_axis_location
        x_axis_location = axes_object.x_axis_location
        top_labels = axes_object.top_labels
        right_labels = axes_object.right_labels

    if not isinstance(location, np.ndarray):
        if axis == 'horizontal':
            location = (location, y_axis_location)
        else:
            location = (x_axis_location, location)
    p = diagram.transform(location)

    # ticksize is globally defined but we can change it
    if axes_object is not None:
        size = axes_object.ticksize

    if element.get('size', None) is not None:
        size = un.valid_eval(element.get('size'))
        if not isinstance(size, np.ndarray):
            size = (size, size)
        else:
            size = (3,3)

    if tactile:
        size = (18,0)

    tick_direction = 1
    if axis == 'horizontal':
        if axes_object is not None:
            tick_direction = axes_object.h_tick_direction
        line_el = line.mk_line((p[0], p[1]+tick_direction*size[0]),
                               (p[0], p[1]-tick_direction*size[1]),
                               diagram,
                               user_coords=False)
    else:
        if axes_object is not None:
            tick_direction = axes_object.v_tick_direction
        line_el = line.mk_line((p[0]-tick_direction*size[0], p[1]),
                               (p[0]+tick_direction*size[1], p[1]),
                               diagram,
                               user_coords=False)

    thickness = element.get('thickness', None)
    if thickness is None:
        if axes_object is None:
            thickness = '2'
        else:
            thickness = axes_object.thickness

    stroke = element.get('stroke', None)
    if stroke is None:
        if axes_object is None:
            stroke = 'black'
        else:
            stroke = axes_object.stroke
    if tactile:
        thickness = '2'
        stroke = 'black'

    line_el.set('stroke-width', thickness)
    line_el.set('stroke', stroke)
    parent.append(line_el)

    try:
        el_text = element.text.strip()
    except:
        el_text = None
    if el_text is not None and (len(el_text) > 0 or len(element) > 0):
        el_copy = copy.deepcopy(element)
        if axis == 'horizontal':
            if tactile:
                if top_labels:
                    align = 'hat'
                    off = '(0,0)'
                else:
                    align = 'ha'
                    off = '(0,0)'
            else:
                if top_labels:
                    align = 'north'
                    off = '(0,7)'
                else:
                    align = 'south'
                    off = '(0,-7)'
        else:
            if tactile:
                if right_labels:
                    align = 'east'
                    off = '(25,0)'
                else:
                    align = 'va'
                    off = '(-25,0)'
            else:
                if right_labels:
                    align = 'east'
                    off = '(7,0)'
                else:
                    align = 'west'
                    off = '(-7,0)'

        if el_copy.get('alignment', None) is None:
            el_copy.set('alignment', align)
        if el_copy.get('offset', None) is None:
            el_copy.set('offset', off)
        el_copy.set("user-coords", "no")
        el_copy.set("anchor", util.pt2str(p, spacer=","))
        label.label(el_copy, diagram, parent, outline_status)


axes_object = None
def get_axes():
    return axes_object

def axes(element, diagram, parent, outline_status):
    if outline_status == "finish_outline":
        return
    global axes_object
    axes_object = Axes(element, diagram, parent)
    
