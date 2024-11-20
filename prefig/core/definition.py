from . import user_namespace as un
import logging

log = logging.getLogger('prefigure')

# allows authors to define mathematical quantities
#    to be used in the diagram

def definition(element, diagram, parent, outline_status):
    substitution = element.get('substitution', 'yes') == 'yes'
    try:
        un.define(element.text, substitution)
    except SyntaxError as e:
        log.error(f"Error in definition: {str(e)}")

    id_suffix = element.get('id-suffix')
    if id_suffix is not None:  # this definition is part of a repeat
        diagram.push_id_suffix('-' + id_suffix) 
        diagram.parse(element, parent, outline_status)
        diagram.pop_id_suffix()

def derivative(element, diagram, parent, outline_status):
    try:
        f = un.valid_eval(element.get('function'))
    except SyntaxError as e:
        log.error(str(e))
        return
    name = element.get('name', None)
    if name is None:
        log.error(f"A <derivative> element needs a name attribute")
        return
    un.derivative(f, name)
