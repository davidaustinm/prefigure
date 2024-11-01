## Some functions to handle shapes and associated constructions

import lxml.etree as ET
import shapely
import numpy as np
import re
from . import user_namespace as un
from . import utilities as util
from . import math_utilities as math_util
from . import tags

# these are the tags that can define shapes
allowed_shapes = {
    'arc',
    'area-between-curves',
    'area-under-curve',
    'circle',
    'ellipse',
    'graph',
    'parametric-curve',
    'path',
    'polygon',
    'rectangle',
    'shape'
}

def define(element, diagram, parent, outline_status):
    if outline_status == 'finish_outline':
        return
    for child in element:
        if child.tag not in allowed_shapes:
            print(f"{child.tag} does not define a shape")
            continue
        if child.get('at', None) is not None:
            child.set('id', child.get('at'))
        dummy_parent = ET.Element('group')
        tags.parse_element(
            child,
            diagram,
            dummy_parent
        )
        shape = dummy_parent.getchildren()[0]
        shape.attrib.pop('stroke', None)
        shape.attrib.pop('fill', None)
        diagram.add_shape(shape)
    

# Process a shape tag
def shape(element, diagram, parent, outline_status):
    if outline_status == 'finish_outline':
        finish_outline(element, diagram, parent)
        return

    reference = element.get('shapes', None)
    if reference is None:
        reference = element.get('shape', None)
        if reference is None:
            print('A <shape> tag needs a @shape or @shapes attribute')
            return

    shape_refs = [r.strip() for r in reference.split(',')]
    shapes = []
    for ref in shape_refs:
        shapes.append(diagram.recall_shape(ref))
        if shapes[-1] is None:
            print(f"{ref} is not a reference to a shape")
            shapes.pop(-1)

    operation = element.get('operation', None)
    if operation is None: 
        if len(shapes) > 1:
            operation = 'union'
        else:
            path = ET.SubElement(parent, 'use')
            path.set('href', r'#' + reference)

    if operation is not None:
        paths = [shape.get('d') for shape in shapes]
        style = None
        if operation == 'convex-hull':
            style = 'linestring'

        geometries = [build_shapely_geom(path,style=style) for path in paths]
        for geom, ref in zip(geometries, shape_refs):
            if not geom.is_valid:
                print(f"The shape {ref} is not a valid shapely geometry")
                print(f"  Perhaps it is not defined by a simple curve")
                print(f"  See the shapely documentation for the operation: {operation}")
                return
        if operation == 'intersection':
            if len(paths) < 2:
                print('Intersections require more than one shape')
                return
            result = shapely.intersection_all(geometries)
        if operation == 'union':
            if len(paths) < 2:
                print('Unions require more than one shape')
                return
            result = shapely.union_all(geometries)
        if operation == 'difference':
            if len(paths) != 2:
                print('Differences require exactly two shapes')
                return
            result = shapely.difference(geometries[0],
                                        geometries[1])
        if (operation == 'symmetric-difference' or
            operation == 'sym-diff'):
            if len(paths) < 2:
                print('Symmetric differences require more than one shape')
                return
            result = shapely.symmetric_difference_all(geometries)
            operation = 'symmetric difference'
        if operation == 'convex-hull':
            if len(paths) > 1:
                geometries = shapely.union_all(geometries)
            result = shapely.convex_hull(geometries)
            operation = 'convex hull'
            
        if shapely.is_empty(result):
            print(f"The {operation} defined by {reference} is empty")
            return

        if isinstance(result, shapely.MultiPolygon):
            d = ''
            for polygon in list(result.geoms):
                if len(d) > 0: d+= ' '
                d += cleanup_str(ET.fromstring(polygon.svg()).get('d'))
        else:
            if isinstance(result, np.ndarray):
                result=result[0]
            d = cleanup_str(ET.fromstring(result.svg()).get('d'))
    
        path = ET.SubElement(parent, 'path')
        path.set('d', d)

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
#    path.set('type', 'rectangle')
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

    
def build_shapely_geom(path, N=30, style=None):
    polygons = []
    points = []
    tokens = path.split()
    while len(tokens) > 0:
        token = tokens.pop(0)
        if token.upper() == 'M' or token.upper() == 'L':
            points.append(build_point(tokens.pop(0), tokens.pop(0)))
            continue
        if token.upper() == 'Q':
            p0 = points[-1]
            p1 = build_point(tokens.pop(0), tokens.pop(0))
            p2 = build_point(tokens.pop(0), tokens.pop(0))
            points += quad_bezier(p0, p1, p2, N)
            continue
        if token.upper() == 'C':
            p0 = points[-1]
            p1 = build_point(tokens.pop(0), tokens.pop(0))
            p2 = build_point(tokens.pop(0), tokens.pop(0))
            p3 = build_point(tokens.pop(0), tokens.pop(0))
            points += cubic_bezier(p0, p1, p2, p3, N)
            continue
        if token.upper() == 'Z':
            if style is None:
                polygons.append(shapely.Polygon(points))
            else:
                polygons.append(shapely.LineString(points))
            points = []
            continue
        print(f"PreFigure did not recognize the token {token} when building shapely geometry")
    if len(points) > 0:
        if style is None:
            polygons.append(shapely.Polygon(points))
        else:
            polygons.append(shapely.LineString(points))
    if style is None:
        return shapely.MultiPolygon(polygons)
    return shapely.MultiLineString(polygons)

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

def cleanup_str(string):
    tokens = []
    for t in string.split():
        for s in t.split(','):
            if bool(re.search('\d', s)):
                x = float(s)
                tokens.append(util.float2str(x))
                continue
            else:
                tokens.append(s)
    return ' '.join(tokens)
                
