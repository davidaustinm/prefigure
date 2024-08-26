## Add a graphical element describing a rectangle

import lxml.etree as ET
from . import user_namespace as un
from . import utilities as util

# Process a rectangle tag
def rectangle(element, diagram, parent, outline_status):
    if outline_status == 'finish_outline':
        finish_outline(element, diagram, parent)
        return

    # An author may specifiy either the lower-left corner or the center
    ll = un.valid_eval(element.get('lower-left', '(0,0)'))
    dims = un.valid_eval(element.get('dimensions', '(1,1)'))
    center = element.get('center', None)
    if center is not None:
        center = un.valid_eval(center)
        ll = center - 0.5 * dims
    p0 = diagram.transform(ll)
    p1 = diagram.transform(ll + dims)

    path = ET.SubElement(parent, 'rect')
    diagram.add_id(path, element.get('id'))
    path.set('x', util.float2str(p0[0]))
    path.set('y', util.float2str(p1[1]))
    path.set('width', util.float2str(p1[0]-p0[0]))
    path.set('height', util.float2str(p0[1]-p1[1]))
    if element.get('corner-radius', None) is not None:
        r = un.valid_eval(element.get('corner-radius'))
        path.set('ry', util.float2str(r))

    if diagram.output_format() == 'tactile':
        if element.get('stroke') is not None:
            element.set('stroke', 'black')
        if element.get('fill') is not None:
            element.set('fill', 'lightgray')
    else:
        util.set_attr(element, 'stroke', 'none')
        util.set_attr(element, 'fill', 'none')

    util.set_attr(element, 'thickness', '2')
    util.add_attr(path, util.get_2d_attr(element))
    path.set('type', 'rectangle')
    util.cliptobbox(path, element, diagram)

    if outline_status == 'add_outline':
        diagram.add_outline(element, path, parent)
        return

    if element.get('outline', 'no') == 'yes' or diagram.output_format() == 'tactile':
        diagram.add_outline(element, path, parent)
        finish_outline(element, diagram, parent)
    else:
        parent.append(path)

def finish_outline(element, diagram, parent):
    diagram.finish_outline(element,
                           element.get('stroke'),
                           element.get('thickness'),
                           element.get('fill', 'none'),
                           parent)
