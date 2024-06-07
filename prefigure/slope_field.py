import lxml.etree as ET
import user_namespace as un
import utilities
import math
import grid_axes

# Add a graphical element for slope fields
def slope_field(element, diagram, parent, outline_status):
    if outline_status == 'finish_outline':
        finish_outline(element, diagram, parent)
        return

    f = un.valid_eval(element.get('function'))
    bbox = diagram.bbox()

    cmds = []

    rx = element.get('rx', None)
    ry = element.get('ry', None)

    if rx is None:
        rx = grid_axes.find_gridspacing((bbox[0], bbox[2]))
    else:
        rx = un.valid_eval(rx)

    if ry is None:
        ry = grid_axes.find_gridspacing((bbox[1], bbox[3]))
    else:
        ry = un.valid_eval(ry)

    x = rx[0]
    while x <= rx[2]:
        y = ry[0]
        while y <= ry[2]:
            slope = f(x,y)
            dx = rx[1]/(4*math.sqrt(1+slope**2))
            x0 = x - dx
            x1 = x + dx
            y0 = y - slope*dx
            y1 = y + slope*dx
            p0 = diagram.transform((x0, y0))
            p1 = diagram.transform((x1, y1))
            cmds.append('M ' + utilities.pt2str(p0))
            cmds.append('L ' + utilities.pt2str(p1))
            y += ry[1]
        x += rx[1]
    d = ' '.join(cmds)

    if diagram.output_format() == 'tactile':
        element.set('stroke', 'black')
    else:
        element.set('stroke', element.get('stroke', 'blue'))
    element.set('thickness', element.get('thickness', '2'))

    path = ET.Element('path')
    diagram.add_id(path, element.get('id'))
    utilities.add_attr(path, utilities.get_1d_attr(element))
    path.set('d', d)
    path.set('type', 'slope field')

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
                           'none',
                           parent)
