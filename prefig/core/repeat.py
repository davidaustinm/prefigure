import lxml.etree as ET
import logging
from . import user_namespace as un
import copy
import re
import numpy as np
from . import group
from . import label
from . import utilities

log = logging.getLogger('prefigure')

# EPUB restricts the characters that can appear in an id to
#   a-z|A-Z|0-9|_|-
# This is an issue here since <repeat> elements can create id's
# This regular expression checks characters to see if they are allowed
epub_check = re.compile(r'[A-Za-z0-9_-]')

# We also define substitutions for the most common disallowed characters
epub_dict = {'(': 'p',
             ')': 'q',
             '[': 'p',
             ']': 'q',
             '{': 'p',
             '}': 'q',
             ',': 'c',
             '.': 'd',
             '=': '_',
             r'#': 'h'}


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
    outline = element.get('outline')
    id = element.get('id')
    if not id.startswith('pf__'):
        id = 'pf__' + id
    element.clear()
    element.tag = 'group'
    if outline is not None:
        element.set('outline', outline)
    if id is not None:
        element.set('id', id)
        element_cp.set('id', id)

    for num, k in enumerate(iterator):
        if isinstance(k, np.ndarray):
            k_str = utilities.np2str(k)
        else:
            k_str = str(k)

        k_str_clean = epub_clean(k_str)
        # This is a change since we use the syntax "var_str" for the id suffix
        if count:
            suffix_str = var + "_" + k_str_clean
        else:
            suffix_str = var + "_" + str(num)

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

def epub_clean(s):
    epub_clean = [bool(epub_check.fullmatch(ch)) for ch in s]
    chars = []
    for index, ch in enumerate(s):
        if epub_clean[index]:
            chars.append(ch)
            continue
        chars.append(epub_dict.get(ch, '_'))
    return "".join(chars)
