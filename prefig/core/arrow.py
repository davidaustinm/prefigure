import lxml.etree as ET
import logging
import math
import numpy as np
from . import utilities as util
from . import CTM
from . import user_namespace as un

log = logging.getLogger('prefigure')

# Form arrows to be used with a variety of graphical components
# Arrowheads are created as markers and then added to paths

# We'll make arrow heads for tactile diagrams.  Our arrow heads
# will be created in default SVG coordinates since the outlines
# are expressed in those coordinates.  Otherwise, the outlines 
# grow extremely large and blot out elements in the background.
# Because of this, we need to create an arrow head for each stroke
# width.
def add_tactile_arrowhead_marker(diagram, path, mid=False):
    # get the stroke width from the graphical component
    stroke_width_str = path.get('stroke-width', '1')
    stroke_width = int(stroke_width_str)
    id = 'arrow-head-'+stroke_width_str

    # if we've seen this already, there's no need to create it again
    if diagram.has_reusable(id):
        return id

    # Now we'll construct the regular (un-outlined) arrow head.
    # "angle" below is half of the angle at the tip of the head
    # BANA guidelines say this should be between 15 and 22.5 so
    # our angle is a bit too big, but this angle creates a bit
    # more differentiation between the arrow head and the path
    # it's attached to
    angle = 25
    A = math.radians(angle)  # angle in radians
    # We'll construct the arrow head as if the stroke-width is t=1
    # and then scale it below.  We need to think of the path as a
    # rectangle whose height is 2t.  So the tip of the arrow head
    # needs to extend between the endpoint of the path.  
    # The variable l measures the extent the tip extends beyond
    # this endpoint.  The variable s measures the horizontal extent
    # of the arrow head and y is the vertical extent above the
    # center line of the path
    t = 1
    s = 9
    l = t/math.tan(A)+0.1
    y = s*math.tan(A)

    # Scale and translate to fit the stroke-width
    # p1, p2, and p3 are the vertices of the triangle
    # with p1 as the tip
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

    # Now we're done constructing the shape of the arrow head
    # so we'll add it to a marker and save as a reusable
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

    outline_width = 9  # 1/8th of an inch
    # We've finished creating the visible arrow head.  Now we need to create
    # an outline which extends 1/8th of an inch beyond the arrow head.
    # First we push the top edge outline_width units away
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

    # Now that we have the vertices of the outlined arrow head, we will 
    # translate the coordinate system to use as a marker
    ctm = CTM.CTM()
    ctm.translate(outline_width, outline_width)
    q1, q2, q3, q4, q5, q6 = [util.pt2str(ctm.transform(p)) for p in [q1, q2, q3, q4, q5, q6]]

    # Now construct the path, put it in a marker, and add as a reusable
    # The id appends "-outline"
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
# This will follow the same logic as the tactile arrow head created
# above, only the shape of a regular arrow head is slightly different
# than a tactile arrow head.
def add_arrowhead_marker(diagram,
                         path,
                         mid=False,
                         arrow_width=None,
                         arrow_angles=None):
    if arrow_width is not None:
        try:
            arrow_width = un.valid_eval(arrow_width)
        except:
            log.error(f"Error parsing arrow-width={arrow_width}")
            return
    if arrow_angles is not None:
        try:
            arrow_angles = un.valid_eval(arrow_angles)
        except:
            log.error(f"Error parsing arrow-angles={arrow_angles}")
            return
    else:
        arrow_angles = (24, 60)

    if diagram.output_format() == 'tactile':
        return add_tactile_arrowhead_marker(diagram, path)
    
    # get the stroke width from the graphical component
    stroke_width_str = path.get('stroke-width', '1')
    stroke_width = int(stroke_width_str)
    stroke_color = path.get('stroke')

    # Dimensions are a bit different if the arrow head is at an
    # end or in the middle of a path
    id_data = f"_{arrow_width}_{arrow_angles[0]}_{arrow_angles[1]}"
    if not mid:
        id = 'arrow-head-end-'+stroke_width_str+id_data+'-'+stroke_color
        if arrow_width is None:
            arrow_width = 4
        dims = (1, arrow_width)
    else:
        id = 'arrow-head-mid-'+stroke_width_str+id_data+'-'+stroke_color
        if arrow_width is None:
            arrow_width = 13/3
        dims = (1, arrow_width) #11/3)

    # If we've already created this one, we'll just move on
    if diagram.has_reusable(id):
        return id

    # Next we'll construct the path defining the arrow head
    t, s = [d/2 for d in dims]
    A, B = [math.radians(angle) for angle in arrow_angles]
    l = t/math.tan(A) + 0.1
    x2 = l - s/math.tan(A)
    x1 = x2 + (s-t)/math.tan(B)

    ctm = CTM.CTM()
    ctm.scale(stroke_width, stroke_width)
    ctm.translate(-x2, s)
    p1 = ctm.transform((l,0))
    p2 = ctm.transform((x2,s))
    p3 = ctm.transform((x1,t))
    p4 = ctm.transform((x1,-t))
    p5 = ctm.transform((x2, -s))

    d = 'M ' + util.pt2str(p1)
    d += 'L ' + util.pt2str(p2)
    d += 'L ' + util.pt2str(p3)
    d += 'L ' + util.pt2str(p4)
    d += 'L ' + util.pt2str(p5)
    d += 'Z'

    # Now put the arrow head in a marker and add as a reusable
    marker = ET.Element('marker', attrib=
                        {'id': id,
                         'markerWidth': util.float2str(stroke_width*(l-x2)),
                         'markerHeight': util.float2str(stroke_width*2*s),
                         'markerUnits': 'userSpaceOnUse',
                         'orient': 'auto-start-reverse',
                         'refX': util.float2str(stroke_width*abs(x2)),
                         'refY': util.float2str(stroke_width*s)
                         }
                        )

    ET.SubElement(marker, 'path', attrib=
                  {'d': d,
                   'fill': stroke_color, #'context-stroke',
                   'stroke': 'none'
                   #'stroke': 'context-none'
                   }
                  )

    diagram.add_reusable(marker)

    outline_width = 2
    # We've finished creating the visible arrow head.  Now we need to create
    # an outline which extends 1/8th of an inch beyond the arrow head.
    # First we push the top edge outline_width units away
    push_angle = math.pi/2 - A
    w = outline_width*np.array((math.cos(push_angle), math.sin(push_angle)))
    q1 = p1 + w
    q2 = p2 + w

    # now the left side
    v = outline_width * np.array((-1,0))
    q3 = p2 + v
    q4 = p5 + v

    # now the bottom edge
    w = np.array((w[0], -w[1]))
    q5 = p5 + w
    q6 = p1 + w

    # Now that we have the vertices of the outlined arrow head, we will 
    # translate the coordinate system to use as a marker
    ctm = CTM.CTM()
    ctm.translate(outline_width, outline_width)
    q1, q2, q3, q4, q5, q6 = [util.pt2str(ctm.transform(p)) for p in [q1, q2, q3, q4, q5, q6]]

    # Now construct the path, put it in a marker, and add as a reusable
    # The id appends "-outline"
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
    marker.set('markerHeight', util.float2str(stroke_width*2*s+2*outline_width))
    marker.set('markerUnits', 'userSpaceOnUse')  # userSpaceOnUse?
    marker.set('orient', 'auto-start-reverse')
    marker.set('refX', util.float2str(abs(stroke_width*x2)+outline_width))
    marker.set('refY', util.float2str(stroke_width*s+outline_width))

    ET.SubElement(marker, 'path', attrib=
                  {'d': d,
                   'fill': 'white', #'context-stroke',
                   'stroke': 'none'
                   #'stroke': 'context-none'
                   }
                  )

    diagram.add_reusable(marker)

    return id

def add_arrowhead_to_path(diagram,
                          location,
                          path,
                          arrow_width=None,
                          arrow_angles=None):
    id = add_arrowhead_marker(diagram,
                              path,
                              location[-3:] == 'mid',
                              arrow_width,
                              arrow_angles)
    path.set(location, r'url(#{})'.format(id))

