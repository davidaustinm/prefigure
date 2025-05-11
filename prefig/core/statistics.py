import copy
import lxml.etree as ET
import numpy as np
import scipy.ndimage
from . import user_namespace as un
from . import math_utilities as math_util
from . import tags

import logging
log = logging.getLogger('prefigure')

def scatter(element, diagram, parent, outline_status):
    points = None
    data = element.get('data', None)
    if data is not None:
        data = un.retrieve(data)

        x_field = element.get('x', None)
        if x_field is None:
            log.error('A <scatter> defined from a data source needs an @x attribute')
            return

        y_field = element.get('y', None)
        if y_field is None:
            log.error('A <scatter> defined from a data source needs a @y attribute')
            return

        filter = element.get('filter', None)
        if filter is not None:
            field, value = un.valid_eval(filter)
            x_data = math_util.filter(data, x_field, field, value)
            y_data = math_util.filter(data, y_field, field, value)
        else:
            x_data = data[x_field]
            y_data = data[y_field]

        points = math_util.zip_lists(x_data, y_data)
    else:
        pts = element.get('points', None)
        if pts is None:
            log.error("A <scatter> needs with a @data or @points attribute")
            return
        points = un.valid_eval(pts)
    un.enter_namespace('__scatter_points', points)

    point_element = copy.deepcopy(element)
    point_element.tag = 'point'
    point_element.set('p', 'point')
    handle = element.get('at', None)
    if handle is not None:
        point_element.set('at', handle+'-point')
    point_text = element.get('point-text', None)
    if point_text is not None:
        point_element.set('annotate', 'yes')
        point_element.set('text', point_text)

    element.tag = 'repeat'
    element.set('parameter', 'point in __scatter_points')
    element.append(point_element)

    tags.parse_element(element, diagram, parent, outline_status)

def histogram(element, diagram, parent, outline_status):
    data = element.get('data', None)
    if data is None:
        log.error('A <histogram> needs a @data attribute')
        return
    data = un.valid_eval(data)

    minimum = un.valid_eval(element.get('min', '0'))
    maximum = element.get('max', None)
    if maximum is None:
        maximum = max(data)
    else:
        maximum = un.valid_eval(maximum)
    bin_str = element.get('bins', '20')
    bins = un.valid_eval(bin_str)

    hist = scipy.ndimage.histogram(data, minimum, maximum, bins)
    x_values = np.linspace(minimum, maximum, bins+1)
    delta_x = (maximum - minimum)/bins

    un.enter_namespace('__histogram_x', x_values)
    un.enter_namespace('__histogram_y', hist)
    un.enter_namespace('__delta_x', delta_x)

    bin_element = copy.deepcopy(element)
    bin_element.tag = 'rectangle'
    bin_element.set('lower-left', '(__histogram_x[bin_num],0)')
    bin_element.set('dimensions',
                    '(__delta_x,__histogram_y[bin_num])')

    handle = element.get('at', None)
    if handle is not None:
        bin_element.set('at', handle+'-bin')
    bin_text = element.get('bin-text', None)
    if bin_text is not None:
        bin_element.set('annotate', 'yes')
        bin_element.set('text', bin_text)

    element.tag = 'repeat'
    element.set('parameter', f"bin_num=0..{bins-1}")
    element.append(bin_element)

    tags.parse_element(element, diagram, parent, outline_status)
    
