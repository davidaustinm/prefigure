import lxml.etree as ET
from . import user_namespace as un
from . import utilities as util

# process a coordinates element defining a coordinate
#    system.  destination, which will usually be None,
#    indicates the region in the current coordinate system
#    into which the new bounding box maps.

def coordinates(element, diagram, root, outline_status):
    ctm, current_bbox = diagram.ctm_bbox()
    bbox = un.valid_eval(element.get('bbox'))
    destination_str = element.get('destination', None)
    if destination_str is None:
        destination = current_bbox
        destination_str = util.np2str(destination)
    else:
        destination = un.valid_eval(destination_str)

    lower_left_clip = diagram.transform(destination[:2])
    upper_right_clip = diagram.transform(destination[2:])

    clippath = ET.Element('clipPath')
    clip_box = ET.SubElement(clippath, 'rect')
    clip_box.set('x', util.float2str(lower_left_clip[0]))
    clip_box.set('y', util.float2str(upper_right_clip[1]))
    width = upper_right_clip[0] - lower_left_clip[0]
    height = lower_left_clip[1] - upper_right_clip[1]
    clip_box.set('width', util.float2str(width))
    clip_box.set('height', util.float2str(height))
    diagram.push_clippath(clippath)

    ctm = ctm.copy()
    ctm.translate(destination[0], destination[1])
    ctm.scale( (destination[2]-destination[0])/float(bbox[2]-bbox[0]),
               (destination[3]-destination[1])/float(bbox[3]-bbox[1]) )
    ctm.translate(-bbox[0], -bbox[1])
    bbox_str = '['+','.join([str(b) for b in bbox])+']'
    un.valid_eval(bbox_str, 'bbox')

    diagram.push_ctm([ctm, bbox])
    diagram.parse(element, root, outline_status)
    diagram.pop_ctm()
    diagram.pop_clippath()
