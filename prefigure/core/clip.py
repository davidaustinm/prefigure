import lxml.etree as ET
from . import utilities as util
from . import user_namespace as un

def clip(element, diagram, parent, outline_status):
    dims = un.valid_eval(element.get('dimensions', '(1,1)'))
    if element.get('center') is not None:
        center = un.valid_eval(element.get('center'))
        ll = center - 0.5*dims
    else:
        ll = un.valid_eval(element.get('lower-left', '(0,0)'))
    p0 = diagram.transform(ll)
    p1 = diagram.transform(ll + dims)

    clip = ET.Element('clipPath')
    path = ET.SubElement(clip, 'rect')
    diagram.add_id(clip, element.get('id'))
    path.set('x', util.float2str(p0[0]))
    path.set('y', util.float2str(p1[1]))
    path.set('width', util.float2str(p1[0]-p0[0]))
    path.set('height', util.float2str(p0[1]-p1[1]))

    diagram.add_reusable(clip)

    group = ET.SubElement(parent, 'g')
    group.set('clip-path', r'url(#{})'.format(clip.get('id')))

    diagram.parse(element, group, outline_status)
