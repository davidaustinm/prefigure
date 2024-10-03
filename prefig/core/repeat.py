import lxml.etree as ET
from . import user_namespace as un
import copy
from . import group
from . import label

# Allows a block of XML to repeat with a changing parameter

def repeat(element, diagram, parent, outline_status):
    parameter = element.get('parameter')
    var, expr = parameter.split('=')
    var = var.strip()
    start, stop = map(un.valid_eval, expr.split('..'))

    # we change this to a group element and then add the children
    # for each value of the parameter
    element_cp = copy.deepcopy(element)
    id = element.get('id')
    element.clear()
    element.tag = 'group'
    if id is not None:
        element.set('id', id)

    for k in range(start, stop+1):
        k_str = str(k)
        un.valid_eval(k_str, var)

        definition = ET.SubElement(element, 'definition')
        definition.text = var + '=' + str(k)
        definition.set('id-suffix', definition.text)

        for child in element_cp:
            definition.append(copy.deepcopy(child))
        
    annotation = None
    if element_cp.get('annotate', 'no') == 'yes' and outline_status != 'add_outline':
        annotation = ET.Element('annotation')
        for attrib in ['id', 'text', 'circular', 'sonify', 'speech']:
            if element_cp.get(attrib, None) is not None:
                annotation.set(attrib, element_cp.get(attrib))
        if annotation.get('text', None) is not None:
            annotation.set('text', label.evaluate_text(annotation.get('text')))
        if annotation.get('speech', None) is not None:
            annotation.set('speech', label.evaluate_text(annotation.get('speech')))
        diagram.push_to_annotation_branch(annotation)

    #diagram.parse(element, parent, outline_status)
    group.group(element, diagram, parent, outline_status)

    if annotation is not None:
        diagram.pop_from_annotation_branch()
