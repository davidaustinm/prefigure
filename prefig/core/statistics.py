import copy
import lxml.etree as ET
from scipy.ndimage import histogram
from . import user_namespace as un
from . import math_utilities as math_util
from . import tags

import logging
log = logging.getLogger('prefigure')

def scatter(element, diagram, parent, outline_status):
    data = element.get('data', None)
    if data is None:
        log.error('A <scatter> needs a @data attribute')
        return
    data = un.retrieve(data)

    x_field = element.get('x', None)
    if x_field is None:
        log.error('A <scatter> needs an @x attribute')
        return

    y_field = element.get('y', None)
    if y_field is None:
        log.error('A <scatter> needs a @y attribute')
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
