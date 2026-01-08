import lxml.etree as ET
import logging
import copy

log = logging.getLogger('prefigure')

def annotations(element, diagram, parent, outline_status):
    # tactile diagrams have no annotations
    if diagram.output_format() == 'tactile':
        return

    # traverse the annotation tree and create the XML annotation output
    # We first add default annotations, such as grid-axes
    diagram.initialize_annotations()

    default_annotations = diagram.get_default_annotations()
    default_annotations_added = False
    for subelement in element:
        if not default_annotations_added:
            for index, annotation in enumerate(default_annotations):
                subelement.insert(index, annotation)
            default_annotations_added = True
        annotate(subelement, diagram)

def annotate(element, diagram, parent = None):
    if log.getEffectiveLevel() == logging.DEBUG:
        tag = element.tag
        log.debug(f"Processing annotation with ref={element.get('ref')}")

    if element.tag is ET.Comment:
        return
    if parent is None:
        parent = diagram.get_annotations_root()

    if element.get('ref', None) is not None:
        ref = element.get('ref')
        if not ref.startswith('pf__'):
            ref = 'pf__' + ref
        element.set('id', ref)
        element.attrib.pop('ref')
    else:
        log.info(f"An annotation has an empty attribute ref")
    element.attrib.pop('annotate', None)

    # let's check to see if this is a reference to an annotation branch
    id = element.get('id')
    if not id.startswith('pf__'):
        id = 'pf__' + id
    annotation = diagram.get_annotation_branch(id)
    if annotation is not None:
        annotate(annotation, diagram, parent)
        return

    # initialize this annotation
    annotation = ET.Element('annotation')
    diagram.add_annotation(annotation)
    annotation.set('id', element.get('id', 'none'))
    
    active = False

    for key, value in element.attrib.items():
        if (key == 'text'):
            active = True
            annotation.set('speech2', value)
        else:
            annotation.set(key, value)
        
    if len(element) > 0:
        el = ET.Element('grouped')
    else:
        if active:
            el = ET.Element('active')
        else:
            el = ET.Element('passive')
    el.text = element.get('id')
    annotation.append(el)

    # determine annotation's position
    toplevel = (parent.tag == 'annotations')
    pos = ET.Element('position')
    if active:
        if toplevel:
            position = len(parent)
        else:
            children = parent.find('children')
            if children is None:
                children = ET.Element('children')
                parent.append(children)
            position = len(children) + 1
            child = ET.Element('active')
            child.text = annotation.get('id')
            children.append(child)
    else:
        position = 0
    pos.text = str(position)
    annotation.append(pos)

    # if annotation has a parent, register it with parent
    if not toplevel:
        components = parent.find('components')
        if components is None:
            components = ET.Element('components')
            parent.append(components)
        if active:
            component = ET.Element('active')
        else:
            component = ET.Element('passive')
        component.text = annotation.get('id')
        components.append(component)

    # descend the tree
    for subelement in element:
        annotate(subelement, diagram, parent=annotation)

    # add parent element to this annotation
    if not toplevel:
        parents = ET.Element('parents')
        if (parent.find('grouped') is not None):
            comp = ET.Element('grouped')
        else:
            comp = ET.Element('active')
        comp.text = parent.get('id')
        parents.append(comp)
        annotation.append(parents)

    if element.get('sonify', 'no') == 'yes':
        sonification = ET.Element('sonification')
        annotation.append(sonification)
        ACTIVE = ET.Element('ACTIVE')
        ACTIVE.text = element.get('id')
        sonification.append(ACTIVE)

pronounciations = {
    'de-solve': 'D E solve',
    'define-shapes': 'define shapes',
    'angle-marker': 'angle marker',
    'area-between-curves': 'area between curves',
    'area-under-curve': 'area under curve',
    'grid-axes': 'grid axes',
    'implicit-curve': 'implicit curve',
    'parametric-curve': 'parametric curve',
    'plot-de-solution': 'plot D E solution',
    'riemann-sum': 'Riemann sum',
    'slope-field': 'slope field',
    'tick-mark': 'tick mark',
    'tangent-line': 'tangent line',
    'vector-field': 'vector field'
}

labeled_elements = {
    'label',
    'point',
    'xlabel',
    'ylabel',
    'angle-marker',
    'tick-mark',
    'item',
    'node',
    'edge'
}

label_subelements = {
    'm': 'math',
    'b': 'bold',
    'it': 'italics',
    'plain': 'plain',
    'newline': 'new line'
}

def diagram_to_speech(diagram):
    diagram = copy.deepcopy(diagram)

    element_num = 0
    for element in diagram.getiterator():
        if element.tag in label_subelements.keys():
            element.getparent().remove(element)
            continue
        attribs = copy.deepcopy(element.attrib)
        for attrib_name in list(element.attrib.keys()):
            element.attrib.pop(attrib_name)

        if element.tag == "diagram":
            element.set('ref', 'figure')
            intro = "This prefigure source file begins with a diagram having these attributes: "
        elif element.tag == "definition":
            element.set('ref', 'element-'+str(element_num))
            tag_speech = 'definition'
            intro = "A definition element defining " + element.text.strip()
        elif element.tag in labeled_elements:
            element.set('ref', 'element-'+str(element_num))
            tag_speech = pronounciations.get(element.tag, element.tag)
            label_text = label_to_speech(element)
            if len(label_text) > 0:
                if len(attribs) == 0:
                    intro = f"A {tag_speech} element with label {label_text}.  The element has no attributes."
                else:
                    intro = f"A {tag_speech} element with label {label_text}.  There are these attributes: "
            else:
                if len(attribs) == 0:
                    intro = f"A {tag_speech} element with no attributes."
                else:
                    intro = f"A {tag_speech} element with these attributes: "
            element.text = None
        else:
            element.set('ref', 'element-'+str(element_num))
            tag_speech = pronounciations.get(element.tag, element.tag)
            if len(attribs) == 0:
                intro = f"A {tag_speech} element with no attributes"
            else:
                intro = f"A {tag_speech} element with these attributes: "
        element.set("text", intro + attributes_to_speech(attribs))
        element_num += 1
        element.tag = "annotation"

    log.error(ET.tostring(diagram, pretty_print=True))
    return diagram

def attributes_to_speech(attribs):
    strings = []
    for key, value in attribs.items():
        strings.append(f"{key} has value {value}")
    return ', '.join(strings)

def label_to_speech(element):
    strings = []
    if (element.text is not None and
        len(element.text.strip()) > 0):
        strings.append(element.text.strip())
    for child in element:
        child_speech = label_subelements.get(child.tag, child.tag)
        strings.append('begin ' + child_speech)
        strings.append(child.text.strip())
        strings.append('end ' + child_speech)
        if (child.tail is not None and
            child.tail.strip() is not None):
            strings.append(child.tail.strip())
    return ' '.join(strings)
