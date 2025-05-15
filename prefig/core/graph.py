import lxml.etree as ET
import logging
from . import user_namespace as un
from . import utilities as util

log = logging.getLogger('prefigure')

# Graph of a 1-variable function
# We'll set up an SVG path element by sampling the graph
# on an equally spaced mesh
def graph(element, diagram, parent, outline_status = None):
    # if we've already added an outline, just plot the graph
    if outline_status == 'finish_outline':
        finish_outline(element, diagram, parent)
        return

    # by default, the domain is the width of the bounding box
    bbox = diagram.bbox()
    domain = element.get('domain')
    if domain is None:
        domain = [bbox[0], bbox[2]]
    else:
        domain = un.valid_eval(domain)

    # retrieve the function from the namespace and generate points
    try:
        f = un.valid_eval(element.get('function'))
    except SyntaxError as e:
        log.error(f"Error retrieving function in graph: {str(e)}")
        return

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

    N = int(element.get('N', 100))
    dx = (domain[1] - domain[0])/N
    x = domain[0]
    cmds = []
    next_cmd = 'M'
    height = (bbox[3] - bbox[1])
    upper = bbox[3] + height
    lower = bbox[1] - height
    last_visible = False
    for _ in range(N+1):
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

    path = ET.Element('path', attrib = attrib)

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


