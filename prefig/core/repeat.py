import lxml.etree as ET
import logging
from . import user_namespace as un
import copy
import numpy as np
from . import group
from . import label
from . import utilities

log = logging.getLogger('prefigure')

# Allows a block of XML to repeat with a changing parameter

def repeat(element, diagram, parent, outline_status):
    try:
        parameter = element.get('parameter')
        fields = parameter.split('=')
        count = False  # keep track of how we are iterating
        if len(fields) == 2:
            var, expr = fields
            var = var.strip()
            start, stop = map(un.valid_eval, expr.split('..'))
            iterator = range(start, stop+1)
            count = True
        else:
            fields = [f.strip() for f in parameter.split()]
            var = fields[0]
            iterator = un.valid_eval(' '.join(fields[2:]))
    except:
        log.error(f"Unable to parse parameter {parameter} in <repeat>")
        return

    # we change this to a group element and then add the children
    # for each value of the parameter
    element_cp = copy.deepcopy(element)
    id = element.get('id')
    element.clear()
    element.tag = 'group'
    if id is not None:
        element.set('id', id)

    for num, k in enumerate(iterator):
        if isinstance(k, np.ndarray):
            k_str = utilities.np2str(k)
        else:
            k_str = str(k)

        un.enter_namespace(k_str, k)
        if count:
            suffix_str = var + "=" + k_str
        else:
            suffix_str = var + "=" + str(num)

        definition = ET.SubElement(element, 'definition')
        definition.text = var + '=' + k_str
        definition.set('id-suffix', suffix_str)

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
