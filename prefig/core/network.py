import lxml.etree as ET
import networkx as nx
import numpy as np
import math
from . import user_namespace as un
from . import math_utilities as m_util
from . import utilities as util
from . import coordinates
from . import label

# Add a graphical element describing a network
def network(element, diagram, parent, outline_status):
    if outline_status == 'finish_outline':
        coords = diagram.get_network_coordinates(element)
        coordinates.coordinates(coords, diagram, parent, outline_status)
        return
    # first we'll use networkx to lay out the graph
    G = nx.Graph()

    for node in element.findall('node'):
        G.add_node(node.get('at'))
    for edge in element.findall('edge'):
        G.add_edge(edge.get('source'),
                   edge.get('target'))

    seed = un.valid_eval(element.get('seed', '1'))
    pos = nx.spring_layout(G, seed=seed)

    box = []
    for key, value in pos.items():
        if len(box) == 0:
            box = [value[0], value[1],
                   value[0], value[1]]
        else:
            box[0] = min(box[0], value[0])
            box[2] = max(box[2], value[0])
            box[1] = min(box[1], value[1])
            box[3] = max(box[3], value[1])

    coords = ET.Element('coordinates')
    coords.set('bbox', util.pt2long_str(box, spacer=','))

    network_group = ET.SubElement(coords, 'group')
    diagram.add_id(network_group, element.get('at'))
    if outline_status == 'add_outline':
        network_group.set('outline', 'always')

    edgedefault = element.get('edgedefault', 'none')

    for edge in element.findall('edge'):
        edgegroup = ET.SubElement(network_group, 'group')
        diagram.add_id(edgegroup, edge.get('at'))
        if outline_status == 'add_outline':
            edgegroup.set('outline', 'always')

        source = pos[edge.get('source')]
        target = pos[edge.get('target')]

        if edgedefault == 'directed':
            head = np.array(target) - np.array(source)
            v = ET.SubElement(edgegroup, 'vector')
            v.set('v', '(' + util.pt2long_str(head, spacer=',') + ')')
            v.set('tail', '(' + util.pt2long_str(source, spacer=',') + ')')
            v.set('stroke', 'black')
            v.set('thickness', '3')
            v.set('head-location', '0.5')
        else:
            l = ET.Element(edgegroup, 'line')
            l.set('p1', '(' + util.pt2longstr(source, spacer=',') + ')')
            l.set('p2', '(' + util.pt2longstr(target, spacer=',') + ')')
            l.set('stroke', 'black')
            l.set('thickness', '3')

        m = m_util.midpoint(source, target)
        label_offset = 6
        if diagram.output_format() == 'tactile':
            label_offset = 8
        o = label_offset * m_util.normalize(np.array(target) - np.array(source))
        o = [o[1], -o[0]]
        o = np.array(o) + un.valid_eval(edge.get('offset', '[0,0]'))
        align = round((180/math.pi*math.atan2(o[1], o[0]))/45) % 8
        l = ET.SubElement(edgegroup, 'label')
        math_element = ET.SubElement(l, 'm')
        math_element.text = r'\text{}' + str(edge.get('weight'))
        l.set('p', '(' + util.pt2long_str(m, spacer=',') + ')')
        l.set('alignment', label.alignment_circle[align])
        l.set('offset', '(' + util.pt2long_str(o, spacer=',') + ')')
        l.set('clear-background', 'no')

    for node in element.findall('node'):
        nodegroup = ET.SubElement(network_group, 'group')
        diagram.add_id(nodegroup, node.get('at')+'-pt')
        if outline_status == 'add_outline':
            nodegroup.set('outline', 'always')

        position = pos[node.get('at')]
        p = ET.SubElement(nodegroup, 'point')
        p.set('p', '(' + util.pt2long_str(position, spacer=',') + ')')
        if diagram.output_format() == 'tactile':
            p.set('size', '25')
        else:
            p.set('size', '15')
        p.set('fill', node.get('fill', 'darkorange'))
        p.set('stroke', 'black')
        p.set('thickness', '2')

        if node.get('label') is not None:
            l = ET.SubElement(nodegroup, 'label')
            math_element = ET.SubElement(l, 'm')
            math_element.text = node.get('label')
            l.set('p', '(' + util.pt2long_str(position, spacer=',') + ')')
            l.set('alignment', 'center')
            l.set('offset', '(0,0)')
            l.set('clear-background', 'no')

    coordinates.coordinates(coords, diagram, parent, outline_status)
    diagram.save_network_coordinates(element, coords)
