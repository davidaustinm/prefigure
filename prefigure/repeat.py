import lxml.etree as ET
import user_namespace as un

# Allows a block of XML to repeat with a changing parameter

def repeat(element, diagram, parent, outline_status):
    parameter = element.get('parameter')
    var, expr = parameter.split('=')
    var = var.strip()
    param_0, param_1 = map(un.valid_eval, expr.split('..'))

    group = ET.SubElement(parent, 'g')
    diagram.add_id(group, element.get('id'))
    group.set('type', 'repeat')

    for k in range(param_0, param_1+1):
        k_str = str(k)
        un.valid_eval(k_str, var)
        diagram.push_id_suffix('-' + var + '=' + k_str) 
        diagram.parse(element, group, outline_status)
        diagram.pop_id_suffix

