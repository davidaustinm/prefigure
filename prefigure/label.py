import os
import sys
import re
import inspect
from pathlib import Path
import numpy as np
import lxml.etree as ET
import utilities as util
import CTM
import user_namespace as un
import tempfile

# Labels will be handled here.
# An important point is how a label will be aligned relative to its anchor.
# These dictionaries define alignments on a 3x3 grid with the anchor at the center.
alignment_displacement = {
    'southeast': [0, 0],
    'east': [0, 0.5],
    'northeast': [0, 1],
    'north': [-0.5, 1],
    'northwest': [-1, 1],
    'west': [-1, 0.5],
    'southwest': [-1, 0],
    'south': [-0.5, 0],
    'center': [-0.5, 0.5],
    'se': [0, 0],
    'e':  [0, 0.5],
    'ne': [0, 1],
    'n':  [-0.5, 1],
    'nw': [-1, 1],
    'w':  [-1, 0.5],
    'sw': [-1, 0],
    's':  [-0.5, 0],
    'c':  [-0.5, 0.5],
    'xaxis-label': [-0.5, 0],
    'ha': [-0.5, 0],  # horizontal axis label
    'va': [-1, 0.5],  # vertical axis label
    'xl': [-1, 1]
}

braille_displacement = {
    'southeast': [0, -1],
    'east': [0, -0.5],
    'northeast': [0, 0],
    'north': [-0.5, 0],
    'northwest': [-1, 0],
    'west': [-1, -0.5],
    'southwest': [-1, -1],
    'south': [-0.5, -1],
    'center': [-0.5, -0.5],
    'se': [0, -1],
    'e':  [0, -0.5],
    'ne': [0, 0],
    'n':  [-0.5, 0],
    'nw': [-1, 0],
    'w':  [-1, -0.5],
    'sw': [-1, -1],
    's':  [-0.5, -1],
    'c':  [-0.5, -0.5],
    'xaxis-label': [0, -1],
    'ha': [0, -1],     # horizontal axis label
    'va': [-1, -0.5],  # vertical axis label
    'xl': [-1, 0]
}

# For labels on arrows
alignment_circle = [
    'east', 'northeast', 'north', 'northwest',
    'west', 'southwest', 'south', 'southeast'
]

# Now we'll place a label into a diagram.  Labels are created by
# mathjax so we're going to put all the labels into an HTML file
# to be processed together.  As a result, labels will be added to
# the diagram after all the other components have been processed.
def label(element, diagram, parent, outline_status = None):
    if outline_status == 'add_outline':  # we're not ready for labels
        return

    # Define a group to hold the label.  
    group = ET.Element('g')
    diagram.add_label(element, group)
    label_id = diagram.find_id(element)
    group.set('id', label_id)

    # For non-tactile output, add a group to the diagram.
    # We'll return and insert the label into the group later
    # after everything else has been processed
    if diagram.output_format() != 'tactile':
        parent.append(group)

    # No matter the output format, we'll write the text into
    # an HTML file to be processed by MathJax
    # TODO:  allow labels with plain text in addition to math
    text = element.text
    if text is None:
        text = ''
    plain_text = text
    for math in element.findall('m') + element.findall('me'):
        if math.tag == 'm':
            text += '\({}\)'.format(math.text)
            plain_text += str(math.text)
        else:
            text += '$${}$$'.format(math.text)
            plain_text += str(math.text)
        if math.tail is not None:
            text += math.tail
            plain_text += str(math.tail)
    text = text.strip()

    # Allow substitutions from the user namespace
    tokens = re.split(r"(\${[^}]*})", text)
    text = ''.join([str(un.valid_eval(token[2:-1])) if token.startswith('${') else token for token in tokens])

    # if we're making a tactile diagram and the text is a single
    # lower-case letter, we'll add a letter indicator in front
    if diagram.output_format() == 'tactile' and len(plain_text) == 1:
        char_distance = ord(plain_text[0]) - ord('a')
        if char_distance >= 0 and char_distance < 26:
            element.set('add-letter-indicator', 'yes')

    # add the label's text to the HTML tree
    div = ET.SubElement(diagram.label_html(), 'div')
    div.set('id', label_id)
    div.text = text


def place_labels(diagram, filename, root, label_group_dict, label_html_tree):
    # if there are no labels, there's nothing to do
    if len(label_group_dict) == 0:
        return

    # prepare the MathJax command
    output_format = diagram.output_format()
    filename = filename[:-4]
    basename = os.path.basename(filename)
    working_dir = tempfile.TemporaryDirectory()
    mj_input = os.path.join(working_dir.name, basename) + '-labels.html'
    mj_output = os.path.join(working_dir.name, basename) + '-' + output_format + '.html'

    # write the HTML file
    with ET.xmlfile(mj_input, encoding='utf-8') as xf:
        xf.write(label_html_tree, pretty_print=True)

    options = ''
    if output_format == 'tactile':
        format = 'braille'
    else:
        options = '--svgenhanced --depth deep'
        format = 'svg'

    # have MathJax process the HTML file and load the resulting
    # SVG labels into label_tree 
    path = Path(os.path.dirname(os.path.abspath(inspect.getfile(inspect.currentframe()))))
    mj_path = str(path.parent.absolute() / 'js') 

    mj_command = 'node {}/mj-sre-page.js --{} {} {} > {}'.format(mj_path, format, options, mj_input, mj_output)
    os.system(mj_command)
    label_tree = ET.parse(mj_output)

    # for braille output, we'll create a group to hold all the labels
    # and their clear backgrounds and add it at the end of the diagram
    if diagram.output_format() == 'tactile':
        background_group = ET.SubElement(root, 'g')
        braille_group = ET.SubElement(root, 'g')

    # now go through each label and place them in the diagram
    for label, group_ctm in label_group_dict.items():
        group, ctm = group_ctm
        id = group.get('id')
        div = label_tree.xpath("//html/body/div[@id = '{}']".format(id))[0]

        if diagram.output_format() == 'tactile':
            position_braille_label(label, diagram, ctm,
                                   background_group, braille_group, div)
        else:
            position_svg_label(label, diagram, ctm, group, div)

    working_dir.cleanup()

def position_braille_label(element, diagram, ctm, background_group,
                           braille_group, div):
    group = ET.SubElement(braille_group, 'g')
    group.set('id', div.get('id'))

    # Determine the anchor point p and then adjust it using
    # the alignment and offset
    p = ctm.transform(un.valid_eval(element.get('p')))
    alignment = util.get_attr(element, 'alignment', 'center')
    displacement = braille_displacement[alignment]

    # TODO:  improve automatic tactile offsets
    offset = util.get_attr(element, 'offset', 'none')
    if offset == 'none':
        offset = [8*(displacement[0] + 0.5), 8*(displacement[1]+0.5)]
    else:
        offset = un.valid_eval(offset)

    offset = [o + 6*np.sign(o) for o in offset]
    if displacement[0] == 0:
        offset[0] += 6
    if displacement[1] == -1:
        offset[1] -= 6

    # 'gap' is the distance, in points, between two braille dots,
    # 1/20th of an inch
    gap = 3.6
    if alignment == 'ha':
        offset = [-4*gap, -30]
    if alignment == 'va':
        offset = [-30, 0]
    if alignment == 'xl':
        offset = [-10, 12]

    p[0] += offset[0]
    p[1] -= offset[1]
    try:
        insert = div.xpath('mjx-data/mjx-braille')[0]
    except IndexError:
        print('Error in processing label, possibly a LaTeX error: ' + div.text)
        sys.exit()
    text = insert.text
    if element.get('add-letter-indicator', 'no') == 'yes':
        text = '\u2830' + text
    w = 5*gap*len(text)
    h = 5*gap

    p[0] += w*displacement[0]
    p[1] -= h*displacement[1]

    # snap the point onto the 20dpi embossing grid
    p = [3.6 * round(c/3.6) for c in p]

    # add a white background behind the label
    bg_margin = 9  # 1/8th of a inch
    rect = ET.SubElement(background_group, 'rect')
    rect.set('x', util.float2str(p[0]-bg_margin))
    rect.set('y', util.float2str(p[1]-h-bg_margin))
    rect.set('width', util.float2str(w+2*bg_margin))
    rect.set('height', util.float2str(h+2*bg_margin))
    rect.set('stroke', 'none')
    rect.set('fill', '#fff')

    # show the lower-left corner point for debugging
    circle = ET.Element('circle')
    circle.set('cx', util.float2str(p[0]))
    circle.set('cy', util.float2str(p[1]))
    circle.set('r', '3')
    circle.set('fill', '#00f')
#    group.append(circle)

    # Now add the label
    text_element = ET.SubElement(group, 'text')
    text_element.set('x', util.float2str(p[0]))
    text_element.set('y', util.float2str(p[1]))
    text_element.text = text
    text_element.set('font-family', "Braille29")
    text_element.set('font-size', "29px")

def position_svg_label(element, diagram, ctm, group, div):
    ns = {'svg': 'http://www.w3.org/2000/svg'}
    try:
        insert = div.xpath('//mjx-data/mjx-container/svg:svg',
                           namespaces=ns)[0]
    except IndexError:
        print('Error in processing label, possibly a LaTeX error: ' + div.text)
        sys.exit()

    # Determine the anchor point p and then adjust it using
    # the alignment and offset
    # TODO:  improve auto offsets
    p = ctm.transform(un.valid_eval(element.get('p')))
    alignment = util.get_attr(element, 'alignment', 'center')
    displacement = alignment_displacement[alignment]

    offset = util.get_attr(element, 'offset', 'none')
    if offset == 'none':
        offset = [8*(displacement[0] + 0.5), 8*(displacement[1]-0.5)]
    else:
        offset = un.valid_eval(offset)

    w = 8*float(insert.get('width')[:-2])
    h = 8*float(insert.get('height')[:-2])

    tform = CTM.translatestr(p[0] + offset[0], p[1] - offset[1])

    sc = float(element.get('scale', '1'))
    if sc != 1:
        tform = tform + ' ' + CTM.scalestr(sc, sc)

    rot = element.get('rotate', None)
    if rot is not None:
        tform = tform + ' ' + CTM.rotatestr(float(rot))
    tform = tform + ' ' + CTM.translatestr(w*displacement[0], -h*displacement[1])

    group.set('transform', tform)

    # add a white rectangle behind the label, if requested
    if element.get('clear-background', 'no') == 'yes':
        bg_margin = int(element.get('background-margin', '6'))
        rect = ET.SubElement(group, 'rect')
        rect.set('x', str(-bg_margin))
        rect.set('y', str(-bg_margin))
        rect.set('width', str(w+2*bg_margin))
        rect.set('height', str(h+2*bg_margin))
        rect.set('stroke', 'none')
        rect.set('fill', '#fff')

    # Express dimensions in px for rsvg-convert
    for attr in ['style', 'width', 'height']:
        fields = insert.get(attr).split()
        dimension = fields[-1]
        index = dimension.find('ex')
        dimension = float(dimension[:index]) * 8
        pts = '{0:.3f}px'.format(dimension)
        fields[-1] = pts
        insert.set(attr, ' '.join(fields))

    group.append(insert)
    diagram.add_id(group, element.get('expr'))
    group.set('type', 'label')

# add a caption to a tactile diagram in the upper-left corner
#   e.g. "Figure 2.3.4"
# TODO:  improve using Michael Cantino's feedback
def caption(element, diagram, parent, outline_status):
    if diagram.output_format() != "tactile":  
        return
    element.tag = 'label'
    element.set('alignment', 'ne')
    element.set('offset', '(0,10)')
    box = diagram.bbox()
    element.set('p', str((box[0], box[3])))
    label(element, diagram, parent)
