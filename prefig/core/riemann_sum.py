import lxml.etree as ET
from . import user_namespace as un
from . import utilities as util

# Add a graphical element describing a Riemann sum
def riemann_sum(element, diagram, parent, outline_status):
    if outline_status == 'finish_outline':
        finish_outline(element, diagram, parent)
        return

    bbox = diagram.bbox()
    domain = element.get('domain')
    if domain == None:
        domain = [bbox[0], bbox[2]]
    else:
        domain = un.valid_eval(domain)

    N = int(element.get('N'))
    f = un.valid_eval(element.get('function'))
    dx = (domain[1]-domain[0])/N
    rule = element.get('rule', 'left')
    rules = {'left': 0, 'right': 1, 'midpoint': 0.5}
    offset = rules[rule] * dx
    x = domain[0]

    cmds = []

    for i in range(N):
        h = f(x + offset)
        p0 = diagram.transform((x, 0))
        p1 = diagram.transform((x, h))
        p2 = diagram.transform((x+dx, h))
        p3 = diagram.transform((x+dx, 0))

        cmds.append('M ' + util.pt2str(p0))
        for p in [p1, p2, p3]:
            cmds.append('L ' + util.pt2str(p))
        cmds.append('Z')
        x += dx

    if diagram.output_format() == 'tactile':
        element.set('stroke', 'black')
        element.set('fill', 'lightgray')
    else:
        element.set('stroke', element.get('stroke', 'black'))
        element.set('fill', element.get('fill', 'lightgray'))
    element.set('thickness', element.get('thickness', '2'))
    # id = diagram.find_id(element, element.get('id'))+'-'+str(i)
    id = diagram.find_id(element, element.get('id'))
    path = ET.Element('path', attrib=
                      {
                          'id': id,
                          'd': ' '.join(cmds),
                          'fill': element.get('fill', 'none'),
                          'stroke': element.get('stroke', 'none'),
                          'stroke-width': element.get('thickness'),
#                          'type': 'riemann-sum'
                      }
    )

    diagram.add_id(path, element.get('id'))
    util.add_attr(path, util.get_2d_attr(element))
#    path.set('type', 'riemann-sum')

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
                           element.get('fill'),
                           parent)
