import lxml.etree as ET
try:
    import networkx as nx
except:
    from .compat import ErrorOnAccess
    nx = ErrorOnAccess("networkx")
import numpy as np
import math
import copy
import logging
from . import user_namespace as un
from . import math_utilities as math_util
from . import utilities as util
from . import coordinates
from . import label
from . import CTM
from . import group
from . import point

log = logging.getLogger('prefigure')

# Add a graphical element describing a network
def network(element, diagram, parent, outline_status):
    # shouldn't go into this
    if outline_status == 'finish_outline':
        coords = diagram.get_network_coordinates(element)
        coordinates.coordinates(coords, diagram, parent, outline_status)
        return

    # Is the network directed?
    directed = element.get('directed', 'no') == 'yes'
    global_loop_scale = element.get('loop-scale', None)
    if global_loop_scale is not None:
        global_loop_scale = un.valid_eval(global_loop_scale)

    # retrieve the label dictionary
    label_predictionary = un.valid_eval(element.get('label-dictionary', '{}'))
    label_dictionary = {}
    for k, v in label_predictionary.items():
        label_dictionary[str(k)] = v
    
    # Let's see if there is a dictionary defining the graph
    graph = element.get('graph', None)
    graph_dict = {}
    if graph is not None:
        graph = un.valid_eval(graph)
        if not isinstance(graph, dict):
            log.error("@graph attribute of a <network> element should be a dictionary")
            return
        for key, value in graph.items():
            value = [str(v) for v in value]
            graph_dict[str(key)] = value
        
    # We'll keep track of the number of edges in a multigraph
    loops = {}           # dictionary to count loops
    directed_edges = {}  # dictionary to count directed edges
    all_edges = {}       # dictionary to count undirected edges

    # The graph could be defined using a dictionary but nodes can be
    # added as subelements of the <network> element.  This allows
    # a node to be drawn differently from the rest

    # Let's find all the subelement nodes and store them in a dictionary
    nodes = {}
    positions = {}
    for node in element.findall('node'):
        handle = node.get('at', None)
        diagram.add_id(node, handle)
        handle = node.get('id')
        nodes[handle] = node

        # check for the position of this node 
        position = node.get('p', None)
        if position is not None:
            positions[handle] = un.valid_eval(position)

        # now check for edges
        edges = node.get('edges', None)
        if edges is not None:
            edges = un.valid_eval(edges)

            # tuples with a single element aren't returned as a list
            # this will be fixed but here's a workaround
            if isinstance(edges, np.ndarray):
                edges = list(edges)
            else:
                edges = [edges]

            # record each given edge
            for destination in edges:
                destination = str(destination)
                if destination == handle:
                    loop_record = loops.get(handle, [])
                    loop_record.append(None)
                    loops[handle] = loop_record
                    continue
                vertices = [handle, destination]

                leaving_edges = directed_edges.get(tuple(vertices), [])
                leaving_edges.append(None)
                directed_edges[tuple(vertices)] = leaving_edges

                vertices.sort()
                all_edges[tuple(vertices)] = all_edges.get(tuple(vertices), 0) + 1

    # Now we'll go through the structure given by the graph dictionary
    for node, edges in graph_dict.items():
        # have we seen this node already?
        if nodes.get(node, None) is None:
            nodes[node] = None
        for destination in edges:
            if destination == node:
                loop_record = loops.get(node, [])
                loop_record.append(None)
                loops[node] = loop_record
                continue
            vertices = [node, destination]

            leaving_edges = directed_edges.get(tuple(vertices), [])
            leaving_edges.append(None)
            directed_edges[tuple(vertices)] = leaving_edges
            
            vertices.sort()
            all_edges[tuple(vertices)] = all_edges.get(tuple(vertices), 0) + 1

    # finally, there may be <edge> subelements of <network> with decorations
    for edge in element.findall('edge'):
        try:
            endpoints = un.valid_eval(edge.get('vertices'))
        except:
            log.error(f"Error in <edge> evaluating vertices={element.get('vertices')}")
            return
        p = str(endpoints[0])
        q = str(endpoints[1])
        if p == q:
            placed = False
            loop_record = loops.get(p, [])
            for i, loop in enumerate(loop_record):
                if loop is None:
                    loop_record[i] = edge
                    placed = True
                    break
            if not placed:
                loop_record.append(edge)
                loops[p] = loop_record
            continue
        vertices = [p, q]

        placed = False
        leaving_edges = directed_edges.get(tuple(vertices), [])
        for i, e in enumerate(leaving_edges):
            if e is None:
                leaving_edges[i] = edge
                directed_edges[tuple(vertices)] = leaving_edges
                placed = True
                break
        if not placed and not directed:
            vertices = [q, p]
            leaving_edges = directed_edges.get(tuple(vertices), [])
            for i, e in enumerate(leaving_edges):
                if e is None:
                    leaving_edges[i] = edge
                    directed_edges[tuple(vertices)] = leaving_edges
                    placed = True
                    break
        if not placed:
            vertices = [p, q]
            leaving_edges = directed_edges.get(tuple(vertices), [])
            leaving_edges.append(edge)
            directed_edges[tuple(vertices)] = leaving_edges
            vertices.sort()
            all_edges[tuple(vertices)] = all_edges.get(tuple(vertices), 0) + 1

    # Use networkx to layout the graph if we need to
    auto_layout = False
    if len(positions) != len(nodes):
        auto_layout = True

        # may need to use MultiGraph or MultiDiGraph
        # layout seems best with a plain Graph though
        G = nx.Graph()

        for key in nodes.keys():
            G.add_node(key)
        for edge, edges in directed_edges.items():
            for _ in range(len(edges)):
                G.add_edge(edge[0], edge[1])

        layout = element.get('layout', None)

        seed = int(element.get('seed', '1'))
        if layout is None or layout == 'spring':
            positions = nx.spring_layout(G, seed=seed)
        elif layout == 'bfs':
            start = element.get('start', None)
            if start is None:
                log.error('bfs network layout needs a starting node')
                return
            positions = nx.bfs_layout(G, start=start)
        elif layout == 'spectral':
            positions = nx.spectral_layout(G)
        elif layout == 'circular':
            positions = nx.circular_layout(G)
        elif layout == 'random':
            positions = nx.random_layout(G, seed=seed)
        elif layout == 'planar':
            positions = nx.planar_layout(G)
        elif layout == 'bipartite':
            alignment = element.get('alignment', 'horizontal')
            bipartite_set = element.get('bipartite-set', None)
            if bipartite_set is None:
                log.error('A bipartite network needs a @bipartite-set attribute')
                return
            bipartite_set = un.valid_eval(bipartite_set)
            bipartite_set = [str(n) for n in bipartite_set]
            positions = nx.bipartite_layout(G,
                                            bipartite_set,
                                            align=alignment)

        # Now that we have the positions of the nodes, we will form the
        # graphical components.  First find the bounding box
        xvals = [p[0] for p in positions.values()]
        yvals = [p[1] for p in positions.values()]
        ll = np.array([min(xvals), min(yvals)])
        ur = np.array([max(xvals), max(yvals)])
        center = 0.5*(ll + ur)
    
        scale = float(element.get('scale', '0.8'))
        rotate = float(element.get('rotate', '0'))
        ctm = CTM.CTM()
        ctm.translate(*(-center))
        ctm.rotate(rotate)
        for key in positions.keys():
            positions[key] = ctm.transform(positions[key])

        xvals = [p[0] for p in positions.values()]
        yvals = [p[1] for p in positions.values()]
        ll = np.array([min(xvals), min(yvals)])
        ur = np.array([max(xvals), max(yvals)])
        center = 0.5*(ll+ur)

        ctm = CTM.CTM()
        ctm.translate(*(-center))
        for key in positions.keys():
            positions[key] = ctm.transform(positions[key])

        xvals = [p[0] for p in positions.values()]
        yvals = [p[1] for p in positions.values()]
        ll = np.array([min(xvals), min(yvals)])
        ur = np.array([max(xvals), max(yvals)])
        center = 0.5*(ll+ur)

        ll = 1/scale * ll
        ur = 1/scale * ur
    
        bbox_str = f"({ll[0]},{ll[1]},{ur[0]},{ur[1]})"

        # We're going to turn the <network> element into <coordinates>
        # This means that the coordinate system we will use for rendering
        # components may be different than the current coordinate system.
        # We need that coordinate system for directed graphs so we'll find
        # it here

        diagram_ctm, diagram_bbox = diagram.ctm_bbox()
        future_ctm = diagram_ctm.copy()
        future_ctm.translate(diagram_bbox[0], diagram_bbox[1])
        future_ctm.scale( (diagram_bbox[2]-diagram_bbox[0])/float(ur[0]-ll[0]),
                          (diagram_bbox[3]-diagram_bbox[1])/float(ur[1]-ll[1]) )
        future_ctm.translate(-ll[0], -ll[1])

    
    edge_stroke = element.get('edge-stroke', 'black')
    edge_thickness = element.get('edge-thickness', '2')
    edge_dash = element.get('edge-dash', 'none')
    node_fill = element.get('node-fill', 'darkorange')
    node_stroke = element.get('node-stroke', 'black')
    node_thickness = element.get('node-thickness', '1')
    node_style = element.get('node-style', 'circle')
    labels = element.get('labels', 'no') == 'yes'
    default_node_size = '10'
    if labels:
        default_node_size = '12'
    node_size = element.get('node-size', default_node_size)
    mid_arrows = element.get('arrows', 'end') == 'middle'

    if diagram.output_format() == 'tactile':
        node_fill = 'white'
        node_stroke = 'black'

    element.clear()
    if auto_layout:
        element.tag = 'coordinates'
        element.set('bbox', bbox_str)
    else:
        element.tag= 'group'
        future_ctm = diagram.ctm().copy()

    # We'll add groups for edges and then nodes. first the edges
    edge_group = ET.SubElement(element, 'group')
    edge_group.set('outline', 'tactile')

    # set up a dictionary to get the directions entering/leaving each node
    # key = node
    # value = list of points the edges move toward
    edge_directions = {}
    
    # controls the spread of edges in a multi-graph

    arrow_buffer = 3
    spread = 15
    if diagram.output_format() == 'tactile':
        arrow_buffer = 12
        spread = 20
        
    # remember that directed_edges is a dictionary:
    #   key = (p, q)
    #   value = all the edges from p to q

    for edge, edges in directed_edges.items():
        handle_0, handle_1 = edge
        endpoints = [handle_0, handle_1]
        endpoints.sort()
        edge = tuple(endpoints)
        y = (all_edges[edge] - 1)/2 * spread
        for num, edge in enumerate(edges):
            ctm = CTM.CTM()
            user_p0 = positions[handle_0]
            user_p1 = positions[handle_1]
            p0 = future_ctm.transform(user_p0)
            p1 = future_ctm.transform(user_p1)
            u = p1 - p0
            angle = math.atan2(u[1], u[0])
            length = math_util.length(u)
            ctm.translate(*p0)
            ctm.rotate(angle, units="rad")
            center = ctm.transform((length/2, y))
            c1 = ctm.transform((length/4, y))
            c2 = ctm.transform((3*length/4, y))

            center = future_ctm.inverse_transform(center)
            c1 = future_ctm.inverse_transform(c1)
            c2 = future_ctm.inverse_transform(c2)

            directions = edge_directions.get(handle_0, [])
            directions.append(c1)
            edge_directions[handle_0] = directions

            directions = edge_directions.get(handle_1, [])
            directions.append(c2)
            edge_directions[handle_1] = directions

            handle = 'edge-' + handle_0 + '-' + handle_1
            if len(edges) > 1:
                handle += '-' + str(num)
            path = ET.SubElement(edge_group, 'path')
            path.set('at', handle)
            if directed:
                if mid_arrows:
                    path.set('mid-arrow', 'yes')
                else:
                    path.set('arrows', '1')

            if edge is None:
                path.set('stroke', edge_stroke)
                path.set('thickness', edge_thickness)
                path.set('dash', edge_dash)
            else:
                path.set('stroke', edge.get('stroke', edge_stroke))
                path.set('thickness', edge.get('thickness', edge_thickness))
                path.set('dash', edge.get('dash', edge_dash))

                # does this edge have a label?
                if len(edge) > 0 or (
                        edge.text is not None and len(edge.text.strip()) > 0
                ):
                    label_location = edge.get('label-location', '0.5')
                    if label_location == '0.5':
                        anchor = center
                    else:
                        label_location = un.valid_eval(label_location)
                        if label_location < 0.5:
                            anchor = math_util.evaluate_bezier(
                                [user_p0, c1, center],
                                2*label_location
                            )
                        else:
                            anchor = math_util.evaluate_bezier(
                                [center, c2, user_p1],
                                2*(label_location - 0.5)
                            )
                    direction = user_p1 - user_p0
                    if y >= 0:
                        label_direction = math_util.rotate(direction, -math.pi/2)
                    else:
                        label_direction = math_util.rotate(direction, math.pi/2)
                    alignment = label.get_alignment_from_direction(label_direction)
                    label_element = copy.deepcopy(edge)
                    label_element.tag = 'label'
                    edge_group.append(label_element)
                    if label_element.get('alignment', None) is None:
                        label_element.set('alignment', alignment)
                    label_element.set('anchor', '('+util.pt2long_str(anchor, spacer=",")+')')

            if abs(y) < 1e-10:
                # it's a straight line
                if directed:
                    path.tag = 'line'
                    if mid_arrows:
                        path.set('endpoints', ','.join(['('+util.pt2long_str(p, spacer=",")+')' for p in [user_p0, user_p1]]))
                        path.set('arrows', '0')
                        path.set('additional-arrows', '(0.5)')
                        y -= spread
                        continue
                    segment = [center, user_p1]
                    for _ in range(10):
                        q0, q1 = segment
                        c = 0.5*(q0+q1)
                        node = nodes.get(handle_1, None)
                        if node is None:
                            end_style = node_style
                        else:
                            end_style = node.get('style', node_style)
                        if point.inside(c, user_p1, float(node_size), end_style, future_ctm, buffer=arrow_buffer):
                            segment = [q0, c]
                        else:
                            segment = [c, q1]
                    path.set('endpoints', ','.join(['('+util.pt2long_str(p, spacer=",")+')' for p in [user_p0, segment[0]]]))
                    y -= spread
                    continue

            path.set('start', util.pt2long_str(user_p0, spacer=","))
            curveto = ET.SubElement(path, 'quadratic-bezier')
            curveto.set('controls', ','.join(['('+util.pt2long_str(p, spacer=",")+')' for p in [c1, center]]))

            # if we need to draw an arrow, we need to find where the edge intersects the node
            if not directed or mid_arrows:
                curveto = ET.SubElement(path, 'quadratic-bezier')
                curveto.set('controls', ','.join(['('+util.pt2long_str(p, spacer=",")+')' for p in [c2, user_p1]]))
            else:
                current_curve = [center, c2, user_p1]
                N = 6  # number of subdivisions
                node = nodes.get(handle_1, None)
                if node is None:
                    end_style = node_style
                else:
                    end_style = node.get('style', node_style)
                for _ in range(N):
                    p0, p1, p2 = current_curve
                    c0 = 0.5*(p0+p1)
                    c1 = 0.5*(p1+p2)
                    center = 0.5*(c0+c1)
                    if point.inside(center, user_p1, float(node_size), end_style, future_ctm, buffer=arrow_buffer):
                        current_curve = [p0, c0, center]
                    else:
                        current_curve = [center, c1, p2]
                        curveto = ET.SubElement(path, 'quadratic-bezier')
                        curveto.set('controls', ','.join(['('+util.pt2long_str(p, spacer=",")+')' for p in [c0, center]]))
            y -= spread

    # now we will add the loops
    for node, loop_record in loops.items():

        # first we find the direction in which we'll draw the loops
        directions = edge_directions.get(node, None)
        node_element = nodes.get(node, None)
        loop_orientation = None
        if node_element is not None:
            loop_orientation = node_element.get('loop-orientation', None)
        if directions is None or loop_orientation is not None:
            if loop_orientation is not None:
                loop_angle = -math.radians(un.valid_eval(loop_orientation))
            else:
                loop_angle = 0
            loop_gap = math.pi/1.75
        else:
            node_position = future_ctm.transform(positions[node])
            for i, direction in enumerate(directions):
                target_position = future_ctm.transform(direction)
                direction = target_position - node_position
                directions[i] = math.atan2(direction[1], direction[0])
            directions.sort()
            directions.append(directions[0]+2*math.pi)
            directions = np.array(directions)
            gaps = directions[1:] - directions[:-1]
            max_gap = np.argmax(gaps)
            loop_angle = (directions[max_gap+1] + directions[max_gap])/2
            loop_gap = min(0.5*gaps[max_gap], math.pi/1.75)

        num_loops = len(loop_record)
        node_position = positions[node]
        P0 = future_ctm.transform(node_position)
        node_size_f = float(node_size)
        for j, loop in enumerate(loop_record):
            ctm = CTM.CTM()
            ctm.translate(*P0)
            ctm.rotate(loop_angle, units="radians")
            scale = (2-0.75*j)*node_size_f
            ctm.scale(scale, scale)

            loop_scale = np.array([1,1])
            if global_loop_scale is not None:
                loop_scale = global_loop_scale
            if loop is not None:
                local_loop_scale = loop.get('loop-scale', None)
                if local_loop_scale is not None:
                    loop_scale = un.valid_eval(local_loop_scale)
            ctm.scale(*loop_scale)

            alpha = 4/3
            P1 = ctm.transform((0,-alpha))
            P2 = ctm.transform((2,-alpha))
            P3 = ctm.transform((2,0))
            P4 = ctm.transform((2,alpha))
            P5 = ctm.transform((0,alpha))

            P1, P2, P3, P4, P5 = [future_ctm.inverse_transform(p) for p in [P1, P2, P3, P4, P5]]

            loop_curves = [[node_position, P1, P2, P3], [P3, P4, P5, node_position]]

            path = ET.SubElement(edge_group, 'path')
            handle = 'loop-' + node
            if len(loop_record) > 1:
                handle += '-' + str(j)
            path.set('at', handle)
            path.set('start', '('+util.pt2long_str(node_position, spacer=",")+')')
            if directed:
                if mid_arrows:
                    path.set('mid-arrow', 'yes')
                else:
                    path.set('arrows', '1')

            if loop is None:
                path.set('stroke', edge_stroke)
                path.set('thickness', edge_thickness)
                path.set('dash', edge_dash)
            else:
                path.set('stroke', loop.get('stroke', edge_stroke))
                path.set('thickness', loop.get('thickness', edge_thickness))
                path.set('dash', loop.get('dash', edge_dash))

            curveto = ET.SubElement(path, 'cubic-bezier')
            curveto.set('controls', '('+','.join(['('+util.pt2long_str(p, spacer=",")+')' for p in [P1, P2, P3]])+')')

            if not directed or mid_arrows:
                curveto = ET.SubElement(path, 'cubic-bezier')
                curveto.set('controls', '('+','.join(['('+util.pt2long_str(p, spacer=",")+')' for p in [P4, P5, node_position]])+')')
            else:
                current_curve = loop_curves[1]
                N = 6  # number of subdivisions
                if node_element is None:
                    end_style = node_style
                else:
                    end_style = node_element.get('style', node_style)
                for _ in range(N):
                    p0, p1, p2, p3 = current_curve
                    p01 = 0.5*(p0+p1)
                    p12 = 0.5*(p1+p2)
                    p23 = 0.5*(p2+p3)
                    q1 = 0.5*(p01+p12)
                    q2 = 0.5*(p12+p23)
                    center = 0.5*(q1+q2)
                    if point.inside(center, node_position, node_size_f, end_style, future_ctm, buffer=arrow_buffer):
                        current_curve = [p0, p01, q1, center]
                    else:
                        current_curve = [center, q2, p23, p3]
                        curveto = ET.SubElement(path, 'cubic-bezier')
                        curveto.set('controls', ','.join(['('+util.pt2long_str(p, spacer=",")+')' for p in [p01, q1, center]]))

            if loop is None:
                path.set('stroke', edge_stroke)
                path.set('thickness', edge_thickness)
                path.set('dash', edge_dash)
            else:
                path.set('stroke', loop.get('stroke', edge_stroke))
                path.set('thickness', loop.get('thickness', edge_thickness))
                path.set('dash', loop.get('dash', edge_dash))

            # does this loop have a label?
            if loop is not None and (
                    len(loop) > 0 or (
                    loop.text is not None and len(loop.text.strip()) > 0
                    )
            ):
                label_location = un.valid_eval(loop.get('label-location', '0.5'))
                if label_location < 0.5:
                    anchor = math_util.evaluate_bezier(
                        loop_curves[0],
                        2*label_location
                    )
                    anchor_ep = math_util.evaluate_bezier(
                        loop_curves[0],
                        2*label_location + 0.0001
                    )
                else:
                    anchor = math_util.evaluate_bezier(
                        loop_curves[1],
                        2*(label_location - 0.5)
                    )
                    anchor_ep = math_util.evaluate_bezier(
                        loop_curves[1],
                        2*(label_location + 0.0001 - 0.5)
                    )
                direction = anchor_ep - anchor
                label_direction = math_util.rotate(direction, math.pi/2)
                alignment = label.get_alignment_from_direction(label_direction)
                
                label_element = copy.deepcopy(loop)
                label_element.tag = 'label'
                edge_group.append(label_element)
                if label_element.get('alignment', None) is None:
                    label_element.set('alignment', alignment)
                    label_element.set('anchor', '('+util.pt2long_str(anchor, spacer=",")+')')

    node_group = ET.SubElement(element, 'group')
    node_group.set('outline', 'tactile')

    for handle, position in positions.items():
        node = nodes.get(handle, None)
        p = ET.SubElement(node_group, 'point')
        p.set('p', '(' + util.pt2long_str(position, spacer=',') + ')')
        p.set('size', node_size)
        p.set('at', 'node-' + handle)
        
        if node is None:
            p.set('fill', node_fill)
            p.set('stroke', node_stroke)
            p.set('thickness', node_thickness)
            p.set('style', node_style)
        else:
            p.set('stroke', node.get('stroke', node_stroke))
            p.set('thickness', node.get('thickness', node_thickness))
            p.set('fill', node.get('fill', node_fill))
            p.set('style', node.get('style', node_style))

        if labels:
            label_element = None
            if node is not None:
                if (
                        len(node) > 0 or
                        (node.text is not None and
                         len(node.text.strip()) > 0)
                ):
                    label_element = copy.deepcopy(node)
                    label_element.tag = 'label'
                    node_group.append(label_element)
            if label_element is None:
                label_element = ET.SubElement(node_group, 'label')
                math_element = ET.SubElement(label_element, 'm')
                label_text = label_dictionary.get(handle, handle)
                math_element.text = label_text
            label_element.set('p', '(' + util.pt2long_str(position, spacer=',') + ')')
            label_element.set('alignment', 'center')
            label_element.set('offset', '(0,0)')
            label_element.set('clear-background', 'no')

    if auto_layout:
        coordinates.coordinates(element, diagram, parent, outline_status)
    else:
        group.group(element,diagram, parent, outline_status)
