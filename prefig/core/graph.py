import lxml.etree as ET
import logging
import math
import numpy as np
from . import math_utilities as math_util
from . import user_namespace as un
from . import utilities as util
from . import arrow

log = logging.getLogger('prefigure')

# Graph of a 1-variable function
# We'll set up an SVG path element by sampling the graph
# on an equally spaced mesh
def graph(element, diagram, parent, outline_status = None):
    # if we've already added an outline, just plot the graph
    if outline_status == 'finish_outline':
        finish_outline(element, diagram, parent)
        return

    polar = element.get('coordinates', 'cartesian') == 'polar'
    # by default, the domain is the width of the bounding box
    bbox = diagram.bbox()
    domain = element.get('domain')
    if domain is None:
        if polar:
            domain = [0, 2*math.pi]
        else:
            domain = [bbox[0], bbox[2]]
    else:
        domain = un.valid_eval(domain)
        if domain[0] == -np.inf:
            domain[0] = bbox[0]
        if domain[1] == np.inf:
            domain[1] = bbox[2]

    # if there are arrows, we need to pull the domain in by two pixels
    # so that the arrows don't go outside the domain
    arrows = int(element.get('arrows', '0'))
    if arrows > 0 and not polar:
        end = diagram.transform((domain[1], 0))
        end[0] -= 2
        new_domain = diagram.inverse_transform(end)
        domain[1] = new_domain[0]
    if arrows == 2 and not polar:
        begin = diagram.transform((domain[0],0))
        begin[0] += 2
        new_domain = diagram.inverse_transform(begin)
        domain[0] = new_domain[0]

    # retrieve the function from the namespace and generate points
    try:
        f = un.valid_eval(element.get('function'))
    except SyntaxError as e:
        log.error(f"Error retrieving function in graph: {str(e)}")
        return

    N = int(element.get('N', '100'))
    if polar:
        cmds = polar_path(element, diagram, f, domain, N)
    else:
        cmds = cartesian_path(element, diagram, f, domain, N)

    # now set up the attributes
    util.set_attr(element, 'thickness', '2')
    util.set_attr(element, 'stroke', 'blue')
    if diagram.output_format() == 'tactile':
        element.set('stroke', 'black')

    attrib = {'id': diagram.find_id(element, element.get('id'))}
    attrib.update(util.get_1d_attr(element))
    attrib.update(
        {
            'd': ' '.join(cmds),
            'fill': 'none'
        }
    )
    if polar and element.get('fill', None) is not None:
        attrib['fill'] = element.get('fill')

    path = ET.Element('path', attrib = attrib)

    arrows = int(element.get('arrows', '0'))
    forward = 'marker-end'
    backward = 'marker-start'
    if element.get('reverse', 'no') == 'yes':
        forward, backward = backward, forward
    if arrows > 0:
        arrow.add_arrowhead_to_path(
            diagram,
            forward,
            path,
            arrow_width=element.get('arrow-width', None),
            arrow_angles=element.get('arrow-angles', None)
        )
    if arrows > 1:
        arrow.add_arrowhead_to_path(
            diagram,
            backward,
            path,
            arrow_width=element.get('arrow-width', None),
            arrow_angles=element.get('arrow-angles', None)
        )

    # By default, we clip the graph to the bounding box
    if element.get('cliptobbox') is None:
        element.set('cliptobbox', 'yes')
    util.cliptobbox(path, element, diagram)

    # Finish up handling any requested outlines
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

def cartesian_path(element, diagram, f, domain, N):
    # Sometimes we encounter a divide by zero when building a graph
    # These are safe to ignore since they return an nan
    np.seterr(divide="ignore")

    # The graphing routine is relatively straightforward.
    # We just walk across the horizontal axis and connect points with lines
    # We try to detect if the function is not defined or if we have passed
    # a vertical asymptote.  One complication is that we try to get close
    # to the singularity or vertical asymptote by subdividing the
    # interval based on the last good point we've seen
    #
    # In the vertical direction, we imagine a buffer (lower, upper)
    # where upper - lower = 3*height and with the viewing box centered inside.
    # We plot anything in the buffer.
    #
    # We maintain a history of sorts using next_cmd, which is either 'M' or 'L'
    # and last_visible, which tells us whether the last point plotted is
    # in the viewing window

    scales = diagram.get_scales()
    if scales[0] == 'log':
        x_positions = np.logspace(np.log10(domain[0]),
                                  np.log10(domain[1]),
                                  N+1)
    else:
        x_positions = np.linspace(domain[0], domain[1], N+1)

    bbox = diagram.bbox()
    dx = (domain[1] - domain[0])/N
    x = domain[0]
    cmds = []
    next_cmd = 'M'
    if scales[1] == 'log':
        bottom = np.log10(bbox[1])
        top = np.log10(bbox[3])
        lower = 10**(bottom - 3)
        upper = 10**(top + 3)
    else:
        height = (bbox[3] - bbox[1])
        upper = bbox[3] + height
        lower = bbox[1] - height
    last_visible = False
    for i, x in enumerate(x_positions):
        if i > 0:
            dx = x - x_positions[i-1]
        else:
            dx = 0
        try:
            y = f(x)
        except:
            if last_visible:
                # we plotted the last point so let's find the singularity,
                # which is in the interval (x-dx, x).  We subdivide 8 times
                # keeping the last valid value in last_good_x
                ddx = dx/2
                xx = x - ddx
                last_good_x = x - dx
                for _ in range(8):
                    ddx /= 2
                    try:
                        y = f(xx)
                    except:
                        xx -= ddx
                        continue
                    last_good_x = xx
                    xx += ddx
                p = diagram.transform((last_good_x, f(last_good_x)))
                cmds += ['L', util.pt2str(p)]

            last_visible = False
            next_cmd = 'M'
            x += dx
            continue
        if y > upper or y < lower:
            if last_visible:
                # the last point was visible so this could be a vertical
                # asymptote.  We will subdivide until we're in the plotting
                # range
                ddx = dx/2
                xx = x - ddx
                last_good_x = x - dx
                for _ in range(8):
                    ddx /= 2
                    yy = f(xx)
                    if yy > upper or yy < lower:
                        xx -= ddx
                    else:
                        last_good_x = xx
                        xx += ddx
                p = diagram.transform((last_good_x, f(last_good_x)))
                cmds += ['L', util.pt2str(p)]

            last_visible = False
            next_cmd = 'M'
            x += dx
            continue
        if next_cmd == 'M' and x > domain[0]:
            # let's see if we need to back up a bit to find the asymptote
            # or edge of the domain
            ddx = dx/2
            xx = x - ddx
            last_good_x = x
            for _ in range(8):
                ddx /= 2
                try:
                    yy = f(xx)
                except:
                    xx += ddx
                    continue
                if yy > upper or yy < lower:
                    xx += ddx
                    continue
                last_good_x = xx
                xx -= ddx

            if last_good_x < x:
                p = diagram.transform((last_good_x, f(last_good_x)))
                cmds += ['M', util.pt2str(p)]
                next_cmd = 'L'

        p = diagram.transform((x, y))
        cmds += [next_cmd, util.pt2str(p)]
        next_cmd = 'L'
        x += dx
        if y < bbox[3] and y > bbox[1]:
            last_visible = True
        else:
            last_visible = False

    return cmds

def log_path(element, diagram, f, domain, N):
    log_y = diagram.get_scales()[1] == 'log'
    x0 = np.log10(domain[0])
    x1 = np.log10(domain[1])
    x_values = np.logspace(x0, x1, N+1)
    cmds = []
    next_cmd = 'M'
    for x in x_values:
        y = f(x)
        if y < 0 and log_y:
            next_cmd = 'M'
            continue
        p = diagram.transform((x, f(x)))
        cmds += [next_cmd, util.pt2str(p)]
        next_cmd = 'L'
    return cmds

def polar_path(element, diagram, f, domain, N):
    bbox = diagram.bbox()
    center = math_util.midpoint(bbox[:2], bbox[2:])
    R = math_util.distance(center, bbox[2:])
    
    if element.get('domain-degrees', 'no') == 'yes':
        domain = [math.radians(d) for d in domain]
    t = domain[0]
    dt = (domain[1] - domain[0])/N
    polar_cmds = []
    next_cmd = 'M'
    for _ in range(N+1):
        try:
            r = f(t)
        except:
            next_cmd = 'M'
            t += dt
            continue

        p = (r*math.cos(t), r*math.sin(t))
        if math_util.distance(p, center) > 2*R:
            next_cmd = 'M'
            t += dt
            continue
        polar_cmds.append(next_cmd)
        polar_cmds.append(util.pt2str(diagram.transform(p)))
        next_cmd = 'L'
        t += dt
    if element.get('closed', 'no') == 'yes':
        polar_cmds.append('Z')

    return polar_cmds
