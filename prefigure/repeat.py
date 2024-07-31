import lxml.etree as ET
from prefigure import user_namespace as un
import copy

# Allows a block of XML to repeat with a changing parameter

def repeat(element, diagram, parent, outline_status):
    parameter = element.get('parameter')
    var, expr = parameter.split('=')
    var = var.strip()
    start, stop = map(un.valid_eval, expr.split('..'))

    # we change this to a group element and then add the children
    # for each value of the parameter
    element_cp = copy.deepcopy(element)
    element.clear()
    element.tag = 'group'

    for k in range(start, stop+1):
        k_str = str(k)
        un.valid_eval(k_str, var)

        definition = ET.SubElement(element, 'definition')
        definition.text = var + '=' + str(k)
        definition.set('id-suffix', definition.text)

        for child in element_cp:
            definition.append(copy.deepcopy(child))
        
    diagram.parse(element, parent, outline_status)

