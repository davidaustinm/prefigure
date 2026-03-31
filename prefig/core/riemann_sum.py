import lxml.etree as ET
import logging
import numpy as np
from . import user_namespace as un
from . import utilities as util
from . import group
from . import label

log = logging.getLogger('prefigure')

# Add a graphical element describing a Riemann sum
# left, right, midpoint, trapezoid, simpsons, upper, lower, samples
def riemann_sum(element, diagram, parent, outline_status):
    diagram.add_id(element, element.get('id'))
    element_id = element.get('id')

    # Author may give a partition and samples so we'll grab them first
    partition = None
    samples = None
    if element.get('partition', None) is not None:
        partition = un.valid_eval(element.get('partition'))
        N = len(partition) - 1
    if element.get('samples', None) is not None:
        samples = un.valid_eval(element.get('samples'))

    # If a parition is not defined, we'll create one
    if partition is None:
        bbox = diagram.bbox()
        domain = element.get('domain')
        if domain == None:
            domain = [bbox[0], bbox[2]]
        else:
            domain = un.valid_eval(domain)
        try:
            N = int(element.get('N'))
        except:
            log.error(f"Error in <riemann-sum> setting N={element.get('N')}")
            return
        partition = np.linspace(domain[0], domain[1], N+1)

    # some rules will be constant on each interval
    constant_rules = {'left', 'right', 'midpoint', 'user-defined', 'upper', 'lower'}
    rule = element.get('rule', None)
    if rule is None:
        if samples is None:
            rule = 'left'
        else:
            rule = 'user-defined'

    if rule is not None:
        if rule == 'left':
            samples = partition[:-1]
        if rule == 'right':
            samples = partition[1:]
        if rule == 'midpoint':
            samples = (partition[:-1] + partition[1:])/2
    try:
        f = un.valid_eval(element.get('function'))
    except:
        log.error(f"Error in <riemann-sum> retrieving function={element.get('function')}")
        return

    annotation = None
    interval_text = None
    if element.get('annotate', 'no') == 'yes':
        annotation = ET.Element('annotation')
        for attrib in ['id', 'text', 'circular', 'sonify', 'speech']:
            if element.get(attrib, None) is not None:
                annotation.set(attrib, element.get(attrib))
        if annotation.get('id', None) is not None:
            annotation.set('ref', annotation.get('id'))
        if annotation.get('text', None) is not None:
            annotation.set('text', label.evaluate_text(annotation.get('text')))
        if annotation.get('speech', None) is not None:
            annotation.set('speech', label.evaluate_text(annotation.get('speech')))
        diagram.push_to_annotation_branch(annotation)
        interval_text = element.get('subinterval-text', None)

    # We will change this element to a group and add area elements below it
    element.tag = 'group'
    outline = element.get('outline', None)
    if outline is None:
        element.set('outline', 'tactile')
    else:
        if outline == 'yes':
            element.set('outline', 'always')
    stroke = element.get('stroke', 'black')
    fill = element.get('fill', 'none')
    thickness = element.get('thickness', '2')
    miterlimit = element.get('miterlimit', None)
    if diagram.output_format() == 'tactile':
        if fill != 'none':
            fill = 'lightgray'

    for interval_num in range(N):
        left = partition[interval_num]
        right = partition[interval_num+1]
        un.enter_namespace('_interval', interval_num)
        un.enter_namespace('_left', f"{left:g}")
        un.enter_namespace('_right', f"{right:g}")
        area = ET.SubElement(element, 'area-under-curve')
        area.set('id', f"{element_id}_{interval_num}")
        area.set('domain', f"({left},{right})")
        area.set('stroke', stroke)
        area.set('fill', fill)
        area.set('thickness', thickness)
        if miterlimit is not None:
            area.set('miterlimit', miterlimit)
        if rule in constant_rules:
            if rule == 'left' or rule == 'right' or rule == 'midpoint' or rule == 'user-defined':
                y_value = f(samples[interval_num])
            if rule == 'upper':
                x_values = np.linspace(left, right, 101)
                y_value = max([f(x) for x in x_values])
            if rule == 'lower':
                x_values = np.linspace(left, right, 101)
                y_value = min([f(x) for x in x_values])
            un.enter_namespace('_height', f"{y_value:g}")
            constant = lambda x, y=y_value: y
            function_name = f"__constant_{interval_num}"
            un.enter_function(function_name, constant)
            area.set('function', function_name)
            area.set('N', '1')
        else:
            if rule == 'trapezoidal':
                area.set('function', element.get('function'))
                area.set('N', '1')
            if rule == 'simpsons':
                h = (right-left)/2
                mid = left+h
                y0 = f(left)
                y1 = f(mid)
                y2 = f(right)
                c = y1
                a = (y0+y2-2*y1)/(2*h*h)
                b = (y2-y0)/(2*h)
                parabola = lambda x, a=a, b=b, c=c, mid=mid: a*(x-mid)**2 + b*(x-mid) + c
                function_name = f"__parabola_{interval_num}"
                un.enter_function(function_name, parabola)
                area.set('function', function_name)
                area.set('N', '100')
        if interval_text is not None:
            interval_annotation = ET.SubElement(annotation, 'annotation')
            interval_annotation.set('ref', area.get('id'))
            interval_annotation.set('text', label.evaluate_text(interval_text))

    group.group(element, diagram, parent, outline_status)
    if annotation is not None:
        diagram.pop_from_annotation_branch()
