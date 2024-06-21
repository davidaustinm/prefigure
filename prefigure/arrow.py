import lxml.etree as ET
import math
import numpy as np
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

'''
# Construct an arrowhead for a tactile diagram, following BANA guidelines
# TODO:  what happens when the stroke-width is unusually large
def add_tactile_arrowhead_marker(diagram, mid=False):
    angle = 25
    outline_width = 9  # 1/8th of an inch
    t = 1
    s = 9
    id = 'tactile-arrow-head'
    A = angle*math.pi/180
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
    marker.set('orient', 'auto-start-reverse')
    marker.set('refX', util.float2str(abs(x2)))
    marker.set('refY', util.float2str(dims[1]/2))

    ET.SubElement(marker, 'path', attrib=
                  {'d': d,
                   'fill': 'context-stroke',
                   'stroke': 'context-none'
                   }
                  )

    diagram.add_reusable(marker)

    # now we'll make an outline marker too
    p1 = np.array((l, 0))
    p2 = np.array((l - s, y))
    p3 = np.array((l - s, -y))

    # push the top edge outline_width units away
    push_angle = math.radians(90 - angle)
    w = outline_width*np.array((math.cos(push_angle), math.sin(push_angle)))
    q1 = p1 + w
    q2 = p2 + w

    # now the left side
    v = outline_width * np.array((-1,0))
    q3 = p1 + v
    q4 = p3 + v

    # now the bottom edge
    w = np.array((w[0], -w[1]))
    q5 = p3 + w
    q6 = p1 + w

    q1, q2, q3, q4, q5, q6 = [util.pt2str(ctm.transform(p)) for p in [q1, q2, q3, q4, q5, q6]]

    ctm = ctm.translate(outline_width, outline_width)

    cmds = ['M', q1]
    cmds += ['L', q2]
    cmds += ['A', str(outline_width), str(outline_width),'0','0','0',q3]
    cmds += ['L', q4]
    cmds += ['A', str(outline_width), str(outline_width),'0','0','0',q5]
    cmds += ['L', q6]
    cmds += ['A', str(outline_width), str(outline_width),'0','0','0',q1]
    cmds += ['Z']
    d = ' '.join(cmds)                
    
    marker = ET.Element('marker')
    marker.set('id', id+'-outline')
    marker.set('markerWidth', util.float2str(l-x2))
    marker.set('markerHeight', util.float2str(dims[1]))
    marker.set('markerUnits', 'strokeWidth')  # userSpaceOnUse?
    marker.set('orient', 'auto-start-reverse')
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
'''

# improved outlining
def add_tactile_arrowhead_marker(diagram, path, mid=False):
    stroke_width_str = path.get('stroke-width', '1')
    stroke_width = int(stroke_width_str)
    id = 'tactile-arrow-head-'+stroke_width_str

    angle = 25
    outline_width = 9  # 1/8th of an inch
    t = 1
    s = 9

    A = angle*math.pi/180
    l = t/math.tan(A)+0.1

    y = s*math.tan(A)

    ctm = CTM.CTM()
    ctm.scale(stroke_width, stroke_width)
    ctm.translate(s-l, y)
    p1 = ctm.transform((l,0))
    p2 = ctm.transform((l-s,y))
    p3 = ctm.transform((l-s,-y))
    cmds = ['M ' + util.pt2str(p1)]
    cmds.append('L ' + util.pt2str(p2))
    cmds.append('L ' + util.pt2str(p3))
    cmds.append('Z')

    d = ' '.join(cmds)

    x2 = l-s
    dims = [1, 2*y]

    marker = ET.Element('marker')
    marker.set('id', id)
    marker.set('markerWidth', util.float2str(stroke_width*(l-x2)))
    marker.set('markerHeight', util.float2str(stroke_width*dims[1]))
    marker.set('markerUnits', 'userSpaceOnUse')  # userSpaceOnUse?
    marker.set('orient', 'auto-start-reverse')
    marker.set('refX', util.float2str(stroke_width*abs(x2)))
    marker.set('refY', util.float2str(stroke_width*dims[1]/2))

    ET.SubElement(marker, 'path', attrib=
                  {'d': d,
                   'fill': 'context-stroke',
                   'stroke': 'context-none'
                   }
                  )

    diagram.add_reusable(marker)

    # now we'll make an outline marker too
    # push the top edge outline_width units away
    push_angle = math.radians(90 - angle)
    w = outline_width*np.array((math.cos(push_angle), math.sin(push_angle)))
    q1 = p1 + w
    q2 = p2 + w

    # now the left side
    v = outline_width * np.array((-1,0))
    q3 = p2 + v
    q4 = p3 + v

    # now the bottom edge
    w = np.array((w[0], -w[1]))
    q5 = p3 + w
    q6 = p1 + w

    ctm = CTM.CTM()
    ctm.translate(outline_width, outline_width)
    q1, q2, q3, q4, q5, q6 = [util.pt2str(ctm.transform(p)) for p in [q1, q2, q3, q4, q5, q6]]

    cmds = ['M', q1]
    cmds += ['L', q2]
    cmds += ['A', str(outline_width), str(outline_width),'0','0','1',q3]
    cmds += ['L', q4]
    cmds += ['A', str(outline_width), str(outline_width),'0','0','1',q5]
    cmds += ['L', q6]
    cmds += ['A', str(outline_width), str(outline_width),'0','0','1',q1]
    cmds += ['Z']

    d = ' '.join(cmds)                
    
    marker = ET.Element('marker')
    marker.set('id', id+'-outline')
    marker.set('markerWidth', util.float2str(stroke_width*(l-x2)+2*outline_width))
    marker.set('markerHeight', util.float2str(stroke_width*dims[1]+2*outline_width))
    marker.set('markerUnits', 'userSpaceOnUse')  # userSpaceOnUse?
    marker.set('orient', 'auto-start-reverse')
    marker.set('refX', util.float2str(abs(stroke_width*x2)+outline_width))
    marker.set('refY', util.float2str(stroke_width*dims[1]/2+outline_width))

    ET.SubElement(marker, 'path', attrib=
                  {'d': d,
                   'fill': 'context-stroke',
                   'stroke': 'context-none'
                   }
                  )
    diagram.add_reusable(marker)
    return id

# Arrowhead marker for a non-tactile figure
def add_arrowhead_marker(diagram, path, mid=False):
    if diagram.output_format() == 'tactile':
        return add_tactile_arrowhead_marker(diagram, path)

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
                         'orient': 'auto-start-reverse',
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
    id = add_arrowhead_marker(diagram, path, location[-3:] == 'mid')
    if diagram.output_format() == 'tactile':
        style_outline = path.get('style-outline', None)
        style_plain = path.get('style-plain', None)
        if style_outline is None:
            style_outline = ''
            style_plain = ''
        style_outline += ';' + location + r': url(#{})'.format(id+'-outline')
        style_plain += ';' + location + r': url(#{})'.format(id)
        path.set('style-outline', style_outline)
        path.set('style-plain', style_plain)
    else:
        style = path.get('style', None)
        if style is None:
            path.set('style', location + r': url(#{})'.format(id))
        else:
            style += ';' + location + r': url(#{})'.format(id)
            path.set('style', style)
