import lxml.etree as ET
import logging
from . import user_namespace as un
from . import math_utilities as math_util
from . import utilities as util
import copy
from . import label

log = logging.getLogger('prefigure')

# Add a graphical element describing a point
def point(element, diagram, parent, outline_status = None):
    # if we're outlining the shape and have already added the outline,
    # we'll just add the point to complete the task
    if outline_status == 'finish_outline':
        finish_outline(element, diagram, parent)
        return

    # determine the location and size of the point from the XML element
    try:
        p = diagram.transform(un.valid_eval(element.get('p')))
    except:
        log.error(f"Error in <point> defining p={element.get('p')}")
        return

    if diagram.output_format() == 'tactile':
        element.set('size', element.get('size', '9'))
    else:
        element.set('size', element.get('size', '4'))
    size = util.get_attr(element, 'size', '1')
    size_str = element.get('size', '1')

    # by default, we'll assume it's a circle but we can change that later
    shape = ET.Element('circle')
    diagram.add_id(shape, element.get('id'))

    # now we'll work out the actual shape of the point
    style = util.get_attr(element, 'style', 'circle')
    if style == 'circle':
        shape.set('cx', util.float2str(p[0]))
        shape.set('cy', util.float2str(p[1]))
        shape.set('r', size)
    size = float(size)
    if style == 'box':
        shape.tag = 'rect'
        shape.set('x', util.float2str(p[0]-size))
        shape.set('y', util.float2str(p[1]-size))
        shape.set('width', util.float2str(2*size))
        shape.set('height', util.float2str(2*size))
    if style == 'diamond':
        shape.tag = 'polygon'
        size *= 1.4
        points = util.pt2str((p[0], p[1]-size), spacer=',')
        points += ' ' + util.pt2str((p[0]+size, p[1]), spacer=',')
        points += ' ' + util.pt2str((p[0], p[1]+size), spacer=',')
        points += ' ' + util.pt2str((p[0]-size, p[1]), spacer=',')
        shape.set('points', points)
    if style == 'cross':
        shape.tag = 'path'
        size *= 1.4
        d = 'M ' + util.pt2str((p[0]-size, p[1]+size))
        d += 'L ' + util.pt2str((p[0]+size, p[1]-size)) 
        d += 'M ' + util.pt2str((p[0]+size, p[1]+size)) 
        d += 'L ' + util.pt2str((p[0]-size, p[1]-size))
        shape.set('d', d)
    if style == 'plus':
        shape.tag = 'path'
        size *= 1.4
        d = 'M ' + util.pt2str((p[0]-size, p[1]))
        d += 'L ' + util.pt2str((p[0]+size, p[1]))
        d += 'M ' + util.pt2str((p[0], p[1]+size))
        d += 'L ' + util.pt2str((p[0], p[1]-size))
        shape.set('d', d)
    if style == 'double-circle':
        shape.tag = 'path'
        r1 = size
        indent = min(size/4, 9)
        r2 = size - indent
        if diagram.output_format() == 'tactile':
            r2 = size - 9
        size_str_2 = str(r2)
        d = 'M ' + util.pt2str((p[0]-r1, p[1]))
        d += 'A ' + size_str + ' ' + size_str + ' 0 0 0 '
        d += util.pt2str((p[0]+r1,p[1])) + ' '
        d += 'A ' + size_str + ' ' + size_str + ' 0 0 0 '
        d += util.pt2str((p[0]-r1,p[1])) + ' Z '
        d += 'M ' + util.pt2str((p[0]-r2, p[1]))
        d += 'A ' + size_str_2 + ' ' + size_str_2 + ' 0 0 0 '
        d += util.pt2str((p[0]+r2,p[1])) + ' '
        d += 'A ' + size_str_2 + ' ' + size_str_2 + ' 0 0 0 '
        d += util.pt2str((p[0]-r2,p[1])) + ' Z '
        shape.set('d', d)

    if diagram.output_format() == 'tactile':
        fill = element.get('fill', None)
        if fill != 'none' and fill != 'white':
            element.set('fill', 'lightgray')
        element.set('stroke', 'black')
    else:
        element.set('fill', util.get_attr(element, 'fill', 'red'))
        element.set('stroke', util.get_attr(element, 'stroke', 'black'))
    element.set('thickness', util.get_attr(element, 'thickness', '2'))
    util.add_attr(shape, util.get_2d_attr(element))
#    shape.set('type', 'point')
    util.cliptobbox(shape, element, diagram)

    if outline_status == 'add_outline':
        diagram.add_outline(element, shape, parent)
        return

    if element.get('outline', 'no') == 'yes' or diagram.output_format() == 'tactile':
        diagram.add_outline(element, shape, parent)
        finish_outline(element, diagram, parent)
    else:
        original_parent = parent
        parent = add_label(element, diagram, parent)
        parent.append(shape)

        # no label has been added if the parent hasn't changed
        if original_parent == parent:
            return

        # if there is a label, then the id is on the outer <g> element
        # so we need to remove it from the children
        if element.get('id', 'none') == parent.get('id'):
            element.attrib.pop('id')
        for child in parent:
            if child.get('id', None) is not None:
                child.attrib.pop('id')

def inside(p, center, size, style, ctm, buffer=0):
    p = ctm.transform(p)
    center = ctm.transform(center)
    p = p - center
    if style == 'circle' or style == 'double-circle':
        return math_util.length(p) < size + buffer
    if style == 'box' or style == 'cross' or style == 'plus':
        if style == 'cross' or style == 'plus':
            size *= 1.4
        return abs(p[0]) < size + buffer and abs(p[1]) < size + buffer
    if style == 'diamond':
        size *= 1.4
        return abs(p[0]+p[1]) < size + buffer and abs(p[0]-p[1]) < size + buffer
    return False

def finish_outline(element, diagram, parent):
    original_parent = parent
    parent = add_label(element, diagram, parent)

    # if we've added a label, remove the id's from element under the parent <g>
    if original_parent != parent:
        for child in parent:
            if child.get('id', None) is not None:
                child.attrib.pop('id')
    diagram.finish_outline(element,
                           element.get('stroke'),
                           element.get('thickness'),
                           element.get('fill', 'none'),
                           parent)

def add_label(element, diagram, parent):
    # Is there a label associated with point?
    text = element.text

    # is there a label here?
    has_text = text is not None and len(text.strip()) > 0
    all_comments = all([subel.tag is ET.Comment for subel in element])
    if has_text or not all_comments:    
        # If there's a label, we'll bundle the label and point in a group
        group = ET.SubElement(parent, 'g')
        diagram.add_id(group, element.get('id'))
#        group.set('type', 'labeled-point')

        # Now we'll create a new XML element describing the label
        el = copy.deepcopy(element)
        el.tag = 'label'

        if element.get('alignment', '').strip() == 'e':
            element.set('alignment', 'east')
        alignment = util.get_attr(element, 'alignment', 'ne')
        el.set('alignment', alignment)
        size = element.get('size', '4')
        displacement = label.alignment_displacement[alignment]
        el.set('anchor', util.get_attr(element, 'p', '(0,0)'))

        # Determine how far to offset the label
        # TODO:  improve tactile offsets
        o = float(size) + 1
        offset = [2*o*(displacement[0]+0.5),
                  2*o*(displacement[1]-0.5)]
        if diagram.output_format() == 'tactile':
            if offset[0] < 0:
                offset[0] -= 6
        else:  # push regular labels a bit more in cardinal directions
            cardinal_push = 3
            if abs(offset[0]) < 1e-14:
                if offset[1] > 0:
                    offset[1] += cardinal_push
                if offset[1] < 0:
                    offset[1] -= cardinal_push
            if abs(offset[1]) < 1e-14:
                if offset[0] > 0:
                    offset[0] += cardinal_push
                if offset[0] < 0:
                    offset[0] -= cardinal_push
        
        relative_offset = element.get('offset', None)
        if relative_offset is not None:
            offset += un.valid_eval(relative_offset)
        el.set('abs-offset', util.np2str(offset))

        # add the label graphical element to the group
        label.label(el, diagram, group)
        return group
    else:
        return parent
