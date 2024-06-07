## Add a graphical element describing a polygon

import lxml.etree as ET
import user_namespace as un
import utilities as util

# Process a polygon tag into a graphical component
def polygon(element, diagram, parent, outline_status):
    if outline_status == 'finish_outline':
        finish_outline(element, diagram, parent)
        return

    if diagram.output_format() == 'tactile':
        if element.get('stroke') is not None:
            element.set('stroke', 'black')
        if element.get('fill') is not None:
            element.set('fill', 'lightgray')
    util.set_attr(element, 'stroke', 'none')
    util.set_attr(element, 'fill', 'none')
    util.set_attr(element, 'thickness', '2')

    # We allow the vertices to be generated programmatically
    parameter = element.get('parameter')
    points = element.get('points')
    if parameter is None:
        points = un.valid_eval(points)
    else:
        var, expr = parameter.split('=')
        param_0, param_1 = map(un.valid_eval, expr.split('..'))
        plot_points = []
        for k in range(param_0, param_1+1):
            un.valid_eval(str(k), var)
            plot_points.append(un.valid_eval(points))
        points = plot_points

    # Form an SVG path now that we have the vertices
    p = diagram.transform(points[0])
    d = ['M ' + util.pt2str(p)]
    for p in points[1:]:
        p = diagram.transform(p)
        d.append('L ' + util.pt2str(p))
    if element.get('closed', 'no') == 'yes':
        d.append('Z')
    d = ' '.join(d)

    path = ET.Element('path')
    diagram.add_id(path, element.get('id'))
    path.set('d', d)
    util.add_attr(path, util.get_2d_attr(element))
    path.set('type', 'polygon')
    element.set('cliptobbox', element.get('cliptobbox', 'yes'))
    util.cliptobbox(path, element)

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
