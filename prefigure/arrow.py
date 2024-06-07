import lxml.etree as ET
import math
import utilities as util
import CTM

# Form arrows to be used with a variety of graphical components
# Arrowheads are created as markers and then added to paths

# Creates a path describing an arrow head (not currently used)
# Note that point, direction, and width are given in SVG default coordinates
def draw_arrowhead(point, direction, width):
    ctm = CTM.CTM()
    ctm.translate(point[0], point[1])
    ctm.rotate(-math.atan2(-direction[1], direction[0]), units='rad')

    dims = (width, 3*width)
    t = dims[0]/2
    s = dims[1]/2

    A = 24*math.pi/180
    B = 60*math.pi/180
    l = t*math.tan(B)+0.1

    x2 = l-s/math.tan(A)
    x1 = x2 + (s-t)/math.tan(B)

    cmds = ['M ' + util.pt2str(ctm.transform((x1, -t)))]
    cmds.append('L ' + util.pt2str(ctm.transform((x2, -s))))
    cmds.append('L ' + util.pt2str(ctm.transform((l, 0))))
    cmds.append('L ' + util.pt2str(ctm.transform((x2, s))))
    cmds.append('L ' + util.pt2str(ctm.transform((x1, t))))
    cmds.append('Z')    

    d = ' '.join(cmds)

    return d

# Construct an arrowhead for a tactile diagram, following BANA guidelines
# TODO:  what happens when the stroke-width is unusually large
def add_tactile_arrowhead_marker(diagram, mid=False):
    t = 1
    s = 9
    id = 'tactile-arrow-head'
    A = 25*math.pi/180
    l = t/math.tan(A)+0.1

    y = s*math.tan(A)

    ctm = CTM.CTM()
    ctm.translate(s-l, y)
    cmds = ['M ' + util.pt2str(ctm.transform((l,0)))]
    cmds.append('L ' + util.pt2str(ctm.transform((l-s, y))))
    cmds.append('L ' + util.pt2str(ctm.transform((l-s, -y))))
    cmds.append('Z')

    d = ' '.join(cmds)

    x2 = l-s
    dims = [1, 2*y]

    marker = ET.Element('marker')
    marker.set('id', id)
    marker.set('markerWidth', util.float2str(l-x2))
    marker.set('markerHeight', util.float2str(dims[1]))
    marker.set('markerUnits', 'strokeWidth')  # userSpaceOnUse?
    marker.set('orient', 'auto')
    marker.set('refX', util.float2str(abs(x2)))
    marker.set('refY', util.float2str(dims[1]/2))

    ET.SubElement(marker, 'path', attrib=
                  {'d': d,
                   'fill': 'context-stroke',
                   'stroke': 'context-none'
                   }
                  )

    diagram.add_reusable(marker)
    return id

# Arrowhead marker for a non-tactile figure
def add_arrowhead_marker(diagram, mid=False):
    if diagram.output_format() == 'tactile':
        return add_tactile_arrowhead_marker(diagram)

    if not mid:
        id = 'arrow-head-end'
        dims = (1, 3)
    else:
        id = 'arrow-head-mid'
        dims = (1, 11/3)

    if diagram.has_reusable(id):
        return id

    t = dims[0]/2
    s = dims[1]/2

    A = 24*math.pi/180
    B = 60*math.pi/180
    l = t*math.tan(B)+0.1

    x2 = l-s/math.tan(A)
    x1 = x2 + (s-t)/math.tan(B)

    ctm = CTM.CTM()
    ctm.translate(-x2, dims[1]/2)
    cmds = ['M ' + util.pt2str(ctm.transform((x1, -t)))]
    cmds.append('L ' + util.pt2str(ctm.transform((x2, -s))))
    cmds.append('L ' + util.pt2str(ctm.transform((l, 0))))
    cmds.append('L ' + util.pt2str(ctm.transform((x2, s))))
    cmds.append('L ' + util.pt2str(ctm.transform((x1, t))))
    cmds.append('Z')    

    d = ' '.join(cmds)

    marker = ET.Element('marker', attrib=
                        {'id': id,
                         'markerWidth': util.float2str(l-x2),
                         'markerHeight': util.float2str(dims[1]),
                         'markerUnits': 'strokeWidth',
                         'orient': 'auto',
                         'refX': util.float2str(abs(x2)),
                         'refY': util.float2str(dims[1]/2)
                         }
                        )

    ET.SubElement(marker, 'path', attrib=
                  {'d': d,
                   'fill': 'context-stroke',
                   'stroke': 'context-none'
                   }
                  )

    diagram.add_reusable(marker)
    return id

def add_arrowhead_to_path(diagram, location, path):
    id = add_arrowhead_marker(diagram, location[-3:] == 'mid')
    path.set('style', location + r': url(#{})'.format(id))
