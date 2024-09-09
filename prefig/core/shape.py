## Some functions to handle shapes and associated constructions

import lxml.etree as ET
import shapely
import numpy as np
from . import user_namespace as un
from . import utilities as util
from . import math_utilities as math_util

# Process a shape tag
def shape(element, diagram, parent, outline_status):
    if outline_status == 'finish_outline':
        finish_outline(element, diagram, parent)
        return

    reference = element.get('ref', None)
    if reference is None:
        print('A <shape> tag needs a @ref attribute')
        return

    shape = diagram.recall_shape(reference)
    path = shape.get('d', None)
    if path is None:
        print(f"{reference} is not a reference to a shape")
        return

    path = ET.SubElement(parent, 'use')
    path.set('href', r'#' + reference)
    diagram.add_id(path, element.get('id'))

    if diagram.output_format() == 'tactile':
        if element.get('stroke') is not None:
            element.set('stroke', 'black')
        if element.get('fill') is not None:
            element.set('fill', 'lightgray')
    else:
        util.set_attr(element, 'stroke', 'none')
        util.set_attr(element, 'fill', 'none')

    util.set_attr(element, 'thickness', '2')
    util.add_attr(path, util.get_2d_attr(element))
    path.set('type', 'rectangle')
    util.cliptobbox(path, element, diagram)

    if outline_status == 'add_outline':
        diagram.add_outline(element, path, parent)
        return

    if (
            element.get('outline', 'no') == 'yes' or
            diagram.output_format() == 'tactile'
    ):
        diagram.add_outline(element, path, parent)
        finish_outline(element, diagram, parent)
    else:
        parent.append(path)

def intersection(element, diagram, parent, outline_status):
    if outline_status == 'finish_outline':
        finish_outline(element, diagram, parent)
        return

    reference = element.get('ref', None)
    if reference is None:
        print('A <shape> tag needs a @ref attribute')
        return

    shape_refs = [r.strip() for r in reference.split(',')]
    if len(shape_refs) < 2:
        print('Intersections require more than one shape')
        return
    shapes = []
    for ref in shape_refs:
        shapes.append(diagram.recall_shape(ref))
        if shapes[-1] is None:
            print(f"{ref} is not a reference to a shape")
            shapes.pop(-1)
    paths = [shape.get('d') for shape in shapes]
    geometries = [build_shapely_geom(path) for path in paths]
    intersection = shapely.intersection_all(geometries)
    if shapely.is_empty(intersection):
        print(f"Intersection defined by {reference} is empty")
        return
    if isinstance(intersection, shapely.MultiPolygon):
        d = ''
        for polygon in list(intersection.geoms):
            if len(d) > 0: d+= ' '
            d += ET.fromstring(polygon.svg()).get('d')
    else:
        d = ET.fromstring(intersection.svg()).get('d')
    
    path = ET.SubElement(parent, 'path')
    diagram.add_id(path, element.get('id'))
    path.set('d', d)

    if diagram.output_format() == 'tactile':
        if element.get('stroke') is not None:
            element.set('stroke', 'black')
        if element.get('fill') is not None:
            element.set('fill', 'lightgray')
    else:
        util.set_attr(element, 'stroke', 'none')
        util.set_attr(element, 'fill', 'none')

    util.set_attr(element, 'thickness', '2')
    util.add_attr(path, util.get_2d_attr(element))
    path.set('type', 'rectangle')
    util.cliptobbox(path, element, diagram)

    if outline_status == 'add_outline':
        diagram.add_outline(element, path, parent)
        return

    if (
            element.get('outline', 'no') == 'yes' or
            diagram.output_format() == 'tactile'
    ):
        diagram.add_outline(element, path, parent)
        finish_outline(element, diagram, parent)
    else:
        parent.append(path)

def union(element, diagram, parent, outline_status):
    if outline_status == 'finish_outline':
        finish_outline(element, diagram, parent)
        return

    reference = element.get('ref', None)
    if reference is None:
        print('A <shape> tag needs a @ref attribute')
        return

    shape_refs = [r.strip() for r in reference.split(',')]
    if len(shape_refs) < 2:
        print('Unions require more than one shape')
        return
    shapes = []
    for ref in shape_refs:
        shapes.append(diagram.recall_shape(ref))
        if shapes[-1] is None:
            print(f"{ref} is not a reference to a shape")
            shapes.pop(-1)
    paths = [shape.get('d') for shape in shapes]
    geometries = [build_shapely_geom(path) for path in paths]
    union = shapely.union_all(geometries)
    if shapely.is_empty(union):
        print(f"Union defined by {reference} is empty")
        return
    if isinstance(union, shapely.MultiPolygon):
        d = ''
        for polygon in list(union.geoms):
            if len(d) > 0: d+= ' '
            d += ET.fromstring(polygon.svg()).get('d')
    else:
        d = ET.fromstring(union.svg()).get('d')
    
    path = ET.SubElement(parent, 'path')
    diagram.add_id(path, element.get('id'))
    path.set('d', d)

    if diagram.output_format() == 'tactile':
        if element.get('stroke') is not None:
            element.set('stroke', 'black')
        if element.get('fill') is not None:
            element.set('fill', 'lightgray')
    else:
        util.set_attr(element, 'stroke', 'none')
        util.set_attr(element, 'fill', 'none')

    util.set_attr(element, 'thickness', '2')
    util.add_attr(path, util.get_2d_attr(element))
    path.set('type', 'rectangle')
    util.cliptobbox(path, element, diagram)

    if outline_status == 'add_outline':
        diagram.add_outline(element, path, parent)
        return

    if (
            element.get('outline', 'no') == 'yes' or
            diagram.output_format() == 'tactile'
    ):
        diagram.add_outline(element, path, parent)
        finish_outline(element, diagram, parent)
    else:
        parent.append(path)

def difference(element, diagram, parent, outline_status):
    if outline_status == 'finish_outline':
        finish_outline(element, diagram, parent)
        return

    reference = element.get('ref', None)
    if reference is None:
        print('A <shape> tag needs a @ref attribute')
        return

    shape_refs = [r.strip() for r in reference.split(',')]
    if len(shape_refs) != 2:
        print('Differences require two shapes')
        return
    shapes = []
    for ref in shape_refs:
        shapes.append(diagram.recall_shape(ref))
        if shapes[-1] is None:
            print(f"{ref} is not a reference to a shape")
            shapes.pop(-1)
    paths = [shape.get('d') for shape in shapes]
    geometries = [build_shapely_geom(path) for path in paths]
    diff = shapely.difference(geometries[0], geometries[1])
    if shapely.is_empty(diff):
        print(f"Difference defined by {reference} is empty")
        return
    if isinstance(diff, shapely.MultiPolygon):
        d = ''
        for polygon in list(diff.geoms):
            if len(d) > 0: d+= ' '
            d += ET.fromstring(polygon.svg()).get('d')
    else:
        d = ET.fromstring(diff.svg()).get('d')
    
    path = ET.SubElement(parent, 'path')
    diagram.add_id(path, element.get('id'))
    path.set('d', d)

    if diagram.output_format() == 'tactile':
        if element.get('stroke') is not None:
            element.set('stroke', 'black')
        if element.get('fill') is not None:
            element.set('fill', 'lightgray')
    else:
        util.set_attr(element, 'stroke', 'none')
        util.set_attr(element, 'fill', 'none')

    util.set_attr(element, 'thickness', '2')
    util.add_attr(path, util.get_2d_attr(element))
    path.set('type', 'rectangle')
    util.cliptobbox(path, element, diagram)

    if outline_status == 'add_outline':
        diagram.add_outline(element, path, parent)
        return

    if (
            element.get('outline', 'no') == 'yes' or
            diagram.output_format() == 'tactile'
    ):
        diagram.add_outline(element, path, parent)
        finish_outline(element, diagram, parent)
    else:
        parent.append(path)

def xor(element, diagram, parent, outline_status):
    if outline_status == 'finish_outline':
        finish_outline(element, diagram, parent)
        return

    reference = element.get('ref', None)
    if reference is None:
        print('A <shape> tag needs a @ref attribute')
        return

    shape_refs = [r.strip() for r in reference.split(',')]
    if len(shape_refs) < 2:
        print('Symmetric differences require more than one shape')
        return
    shapes = []
    for ref in shape_refs:
        shapes.append(diagram.recall_shape(ref))
        if shapes[-1] is None:
            print(f"{ref} is not a reference to a shape")
            shapes.pop(-1)
    paths = [shape.get('d') for shape in shapes]
    geometries = [build_shapely_geom(path) for path in paths]
    xor = shapely.symmetric_difference_all(geometries)
    if shapely.is_empty(xor):
        print(f"Symmetric difference defined by {reference} is empty")
        return
    if isinstance(xor, shapely.MultiPolygon):
        d = ''
        for polygon in list(xor.geoms):
            if len(d) > 0: d+= ' '
            d += ET.fromstring(polygon.svg()).get('d')
    else:
        d = ET.fromstring(xor.svg()).get('d')
    
    path = ET.SubElement(parent, 'path')
    diagram.add_id(path, element.get('id'))
    path.set('d', d)

    if diagram.output_format() == 'tactile':
        if element.get('stroke') is not None:
            element.set('stroke', 'black')
        if element.get('fill') is not None:
            element.set('fill', 'lightgray')
    else:
        util.set_attr(element, 'stroke', 'none')
        util.set_attr(element, 'fill', 'none')

    util.set_attr(element, 'thickness', '2')
    util.add_attr(path, util.get_2d_attr(element))
    path.set('type', 'rectangle')
    util.cliptobbox(path, element, diagram)

    if outline_status == 'add_outline':
        diagram.add_outline(element, path, parent)
        return

    if (
            element.get('outline', 'no') == 'yes' or
            diagram.output_format() == 'tactile'
    ):
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

def build_shapely_geom(path, N=30):
    points = []
    tokens = path.split()
    while len(tokens) > 0:
        token = tokens.pop(0)
        if token == 'M' or token == 'L':
            points.append(build_point(tokens.pop(0), tokens.pop(0)))
            continue
        if token == 'Q':
            p0 = points[-1]
            p1 = build_point(tokens.pop(0), tokens.pop(0))
            p2 = build_point(tokens.pop(0), tokens.pop(0))
            points += quad_bezier(p0, p1, p2, N)
            continue
        if token == 'C':
            p0 = points[-1]
            p1 = build_point(tokens.pop(0), tokens.pop(0))
            p2 = build_point(tokens.pop(0), tokens.pop(0))
            p3 = build_point(tokens.pop(0), tokens.pop(0))
            points += cubic_bezier(p0, p1, p2, p3, N)
            continue
        if token == 'Z':
            continue
        print(f"PreFigure doesn't recognize the token {token} when building shapely geometry")
    return shapely.Polygon(points)

def build_point(s0, s1):
    return [float(s) for s in [s0,s1]]

def quad_bezier(p0, p1, p2, N):
    p0, p1, p2 = [np.array(p) for p in [p0,p1,p2]]
    points = []
    t = 0
    dt = 1/N
    for _ in range(N+1):
        points.append((1-t)**2*p0 + 2*t*(1-t)*p1 + t**2*p2)
        t += dt
    return points

def cubic_bezier(p0, p1, p2, p3, N):
    p0, p1, p2, p3 = [np.array(p) for p in [p0,p1,p2,p3]]
    points = []
    t = 0
    dt = 1/N
    for _ in range(N+1):
        points.append((1-t)**3*p0 + 3*t*(1-t)**2*p1 + 3*t**2*(1-t)*p2+t**3*p3)
        t += dt
    return points


