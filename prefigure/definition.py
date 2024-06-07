import user_namespace as un

# allows authors to define mathematical quantities
#    to be used in the diagram

def definition(element, diagram, parent, outline_status):
    substitution = element.get('substitution', 'yes') == 'yes'
    un.define(element.text, substitution)

def derivative(element, diagram, parent, outline_status):
    f = un.valid_eval(element.get('function'))
    name = element.get('name')
    un.derivative(f, name)
