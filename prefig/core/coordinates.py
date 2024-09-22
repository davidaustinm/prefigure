import lxml.etree as ET
import numpy as np
from . import user_namespace as un
from . import utilities as util

# process a coordinates element defining a coordinate
#    system.  destination, which will usually be None,
#    indicates the region in the current coordinate system
#    into which the new bounding box maps.

def coordinates(element, diagram, root, outline_status):
    ctm, current_bbox = diagram.ctm_bbox()
    destination_str = element.get('destination', None)
    if destination_str is None:
        destination = current_bbox
        destination_str = util.np2str(destination)
    else:
        destination = un.valid_eval(destination_str)

    lower_left_clip = diagram.transform(destination[:2])
    upper_right_clip = diagram.transform(destination[2:])

    dest_dx, dest_dy = upper_right_clip - lower_left_clip
    dest_dy *= -1

    bbox = un.valid_eval(element.get('bbox'))
    if element.get('aspect-ratio', None) is not None:
        ratio = un.valid_eval(element.get('aspect-ratio'))
        if element.get('preserve-y-range', 'no') == 'yes':
            box_dy = bbox[3]-bbox[1]
            y_scale = dest_dy / box_dy
            x_scale = ratio * y_scale
            box_dx = dest_dx / x_scale
            bbox = np.array([bbox[0],
                             bbox[1],
                             bbox[0] + box_dx,
                             bbox[3]])
        else:
            box_dx = bbox[2]-bbox[0]
            x_scale = dest_dx / box_dx
            y_scale = x_scale / ratio
            box_dy = dest_dy / y_scale
            bbox = np.array([bbox[0],
                             bbox[1],
                             bbox[2],
                             bbox[1] + box_dy])

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
