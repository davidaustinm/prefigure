import lxml.etree as ET

# add a group to the diagram for one of two possible reasons:
# to group graphical components to be annotated as a group or 
# to place outlines behind all of the grouped components before 
# stroking them
def group(element, diagram, parent, outline_status):
    # determine whether we will outline the grouped components together
    outline = element.get('outline', None)
    if outline == 'always' or outline == diagram.output_format():
        # we'll pass through the grouped components twice first adding
        # the outline
        group = ET.SubElement(parent, 'g')
        diagram.add_id(group, element.get('id'))
        diagram.parse(element, group, outline_status = 'add_outline')

        # then stroking the grouped components
        group = ET.SubElement(parent, 'g')
        diagram.add_id(group, element.get('id'))
        diagram.parse(element, group, outline_status = 'finish_outline')

        return

    group = ET.SubElement(parent, 'g')
    diagram.add_id(group, element.get('id'))
    diagram.parse(element, group, outline_status)
