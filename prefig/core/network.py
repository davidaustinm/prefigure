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

log = logging.getLogger('prefigure')

# Add a graphical element describing a network
def network(element, diagram, parent, outline_status):
    if outline_status == 'finish_outline':
        coords = diagram.get_network_coordinates(element)
        coordinates.coordinates(coords, diagram, parent, outline_status)
        return

    # Is the network directed?
    directed = element.get('directed', 'no') == 'yes'

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
    loop_count = {}      # dictionary for number of loops
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
                    loop_count[handle] = loop_count.get(handle, 0) + 1
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
                loop_count[node] = loop_count.get(node, 0) + 1
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
            loop_count[p] = loop_count.get(p, 0) + 1
            continue
        vertices = [p, q]

        placed = False
        leaving_edges = directed_edges.get(tuple(vertices), [])
        for i, e in enumerate(leaving_edges):
            if e is None:
                leaving_edges[i] = edge
                placed = True
                break
        if not placed and not directed:
            vertices = [q, p]
            leaving_edges = directed_edges.get(tuple(vertices), [])
            for i, e in enumerate(leaving_edges):
                if e is None:
                    leaving_edges[i] = edge
                    placed = True
                    break
        if not placed:
            vertices = [p, q]
            leaving_edges = directed_edges.get(tuple(vertices), [])
            leaving_edges.append(edge)
            vertices.sort()
            all_edges[tuple(vertices)] = all_edges.get(tuple(vertices), 0) + 1

        directed_edges[tuple(vertices)] = leaving_edges
        

    # Use networkx to layout the graph if we need to
    if len(positions) != len(nodes):
        # may need to use MultiGraph or MultiDiGraph
        # layout seems best with a plain Graph though
        G = nx.Graph()

        for key in nodes.keys():
            G.add_node(key)
        for edge, edges in directed_edges.items():
            for _ in range(len(edges)):
                G.add_edge(edge[0], edge[1])

        layout = element.get('layout', None)

        if layout is None or layout == 'spring':
            seed = int(element.get('seed', '1'))
            positions = nx.spring_layout(G, seed=seed)
        elif layout == 'bfs':
            start = element.get('start', None)
            if start is None:
                log.error('bfs network layout needs a starting node')
                return
            positions = nx.bfs_layout(G, start=start)
        elif layout == 'spectral':
            positions = nx.spectral_layout(G)

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

    if diagram.output_format() == 'tactile':
        node_fill = 'white'
        node_stroke = 'black'

    element.clear()
    element.tag = 'coordinates'
    element.set('bbox', bbox_str)

    # We'll add groups for edges and then nodes. first the edges
    edge_group = ET.SubElement(element, 'group')
    edge_group.set('outline', 'tactile')
    
    spread = 15
    # remember that directed_edges is a dictionary:
    #   key = (p, q)
    #   value = all the edges from p to q
    for edge, edges in directed_edges.items():
        handle_0, handle_1 = edge
        endpoints = [handle_0, handle_1]
        endpoints.sort()
        edge = tuple(endpoints)
        y = (all_edges[edge] - 1)/2 * spread
#        for _ in range(len(edges)):
        for edge in edges:
            ctm = CTM.CTM()
            user_p0 = positions[handle_0]
            user_p1 = positions[handle_1]
            p0 = diagram.transform(user_p0)
            p1 = diagram.transform(user_p1)
            u = p1 - p0
            angle = math.atan2(u[1], u[0])
            length = math_util.length(u)
            ctm.translate(*p0)
            ctm.rotate(angle, units="rad")
            center = ctm.transform((length/2, y))
            c1 = ctm.transform((length/4, y))
            c2 = ctm.transform((3*length/4, y))
            center = diagram.inverse_transform(center)
            c1 = diagram.inverse_transform(c1)
            c2 = diagram.inverse_transform(c2)

            path = ET.SubElement(edge_group, 'path')
            if directed:
                path.set('mid-arrow', 'yes')

            path.set('start', util.pt2long_str(user_p0, spacer=","))
            if edge is None:
                path.set('stroke', edge_stroke)
                path.set('thickness', edge_thickness)
                path.set('dash', edge_dash)
            else:
                path.set('stroke', edge.get('stroke', edge_stroke))
                path.set('thickness', edge.get('thickness', edge_thickness))
                path.set('dash', edge.get('dash', edge_dash))

            curveto = ET.SubElement(path, 'quadratic-bezier')
            curveto.set('controls', ','.join(['('+util.pt2long_str(p, spacer=",")+')' for p in [c1, center]]))
            curveto = ET.SubElement(path, 'quadratic-bezier')
            curveto.set('controls', ','.join(['('+util.pt2long_str(p, spacer=",")+')' for p in [c2, user_p1]]))

            y -= spread

    node_group = ET.SubElement(element, 'group')
    node_group.set('outline', 'tactile')

    for handle, position in positions.items():
        node = nodes.get(handle, None)
        p = ET.SubElement(node_group, 'point')
        p.set('p', '(' + util.pt2long_str(position, spacer=',') + ')')
        p.set('size', node_size)

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
            label_text = handle
            if node is not None:
                label_text = node.get('label', label_text)
            l = ET.SubElement(node_group, 'label')
            math_element = ET.SubElement(l, 'm')
            math_element.text = label_text
            l.set('p', '(' + util.pt2long_str(position, spacer=',') + ')')
            l.set('alignment', 'center')
            l.set('offset', '(0,0)')
            l.set('clear-background', 'no')
            
    coordinates.coordinates(element, diagram, parent, outline_status)
#    diagram.save_network_coordinates(element, coords)
