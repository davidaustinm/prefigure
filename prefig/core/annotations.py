import lxml.etree as ET
import logging

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
        element.set('id', element.get('ref'))
        element.attrib.pop('ref')
    else:
        log.info(f"An annotation has an empty attribute ref")
    element.attrib.pop('annotate', None)

    # let's check to see if this is a reference to an annotation branch
    annotation = diagram.get_annotation_branch(element.get('id'))
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
