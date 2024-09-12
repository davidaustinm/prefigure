import lxml.etree as ET
from . import utilities as util
from . import user_namespace as un

def clip(element, diagram, parent, outline_status):
    shape_ref = element.get('shape', None)
    if shape_ref is None:
        print("A <clip> tag needs a @shape attribute")
        return

    shape = diagram.recall_shape(shape_ref)

    clip = ET.Element('clipPath')
    clip.append(shape)
    clip_id = 'clip-'+shape_ref
    clip.set('id', clip_id)

    diagram.add_reusable(clip)

    group = ET.SubElement(parent, 'g')
    group.set('clip-path', r'url(#{})'.format(clip.get('id')))

    diagram.parse(element, group, outline_status)
