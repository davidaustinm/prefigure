import user_namespace as un

# process a coordinates element defining a coordinate
#    system.  destination, which will usually be None,
#    indicates the region in the current coordinate system
#    into which the new bounding box maps.

def coordinates(element, diagram, root, outline_status):
    ctm, current_bbox = diagram.ctm_bbox()
    bbox = un.valid_eval(element.get('bbox'))
    destination = element.get('destination', None)
    if destination is None:
        destination = current_bbox
    else:
        destination = un.valid_eval(destination)

    ctm = ctm.copy()
    ctm.translate(destination[0], destination[1])
    ctm.scale( (destination[2]-destination[0])/float(bbox[2]-bbox[0]),
               (destination[3]-destination[1])/float(bbox[3]-bbox[1]) )
    ctm.translate(-bbox[0], -bbox[1])
    un.valid_eval(str(destination), 'bbox')

    diagram.push_ctm([ctm, bbox])
    diagram.parse(element, root, outline_status)
    diagram.pop_ctm()
