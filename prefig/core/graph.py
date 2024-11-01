import lxml.etree as ET
from . import user_namespace as un
from . import utilities as util

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
    f = un.valid_eval(element.get('function'))
    N = int(element.get('N', 100))
    dx = (domain[1] - domain[0])/N
    x = domain[0]
    cmds = []
    for _ in range(N+1):
        p = diagram.transform((x, f(x)))
        if len(cmds) == 0:
            cmds = ['M', util.pt2str(p)]
        else:
            cmds += ['L', util.pt2str(p)]
        x += dx

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
            'fill': 'none',
#            'type': 'function-graph'
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


