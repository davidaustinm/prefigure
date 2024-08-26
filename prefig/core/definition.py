from . import user_namespace as un

# allows authors to define mathematical quantities
#    to be used in the diagram

def definition(element, diagram, parent, outline_status):
    substitution = element.get('substitution', 'yes') == 'yes'
    un.define(element.text, substitution)

    id_suffix = element.get('id-suffix')
    if id_suffix is not None:  # this definition is part of a repeat
        diagram.push_id_suffix('-' + id_suffix) 
        diagram.parse(element, parent, outline_status)
        diagram.pop_id_suffix()

def derivative(element, diagram, parent, outline_status):
    f = un.valid_eval(element.get('function'))
    name = element.get('name')
    un.derivative(f, name)
