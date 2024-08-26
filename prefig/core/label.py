import os
import sys
import re
import math
import inspect
try:
    import louis
except:
    print("Failed to import louis")
import cairo
from pathlib import Path
import numpy as np
import lxml.etree as ET
from . import utilities as util
from . import CTM
from . import user_namespace as un
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

def get_alignment_from_direction(direction):
    direction_angle = math.degrees(math.atan2(direction[1], direction[0]))
    align = round(direction_angle/45) % 8
    return alignment_circle[align]

nemeth_on =  '⠸⠩ '
nemeth_off = '⠸⠱ '

# We use pycairo to measure the dimensions of svg text labels
# so we need a cairo context.  This is not needed for tactiles diagrams

surface = cairo.SVGSurface(None, 200, 200)
context = cairo.Context(surface)
context.select_font_face('sans')
font_size = 14
context.set_font_size(font_size)
cairo_context = context

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
    diagram.add_id(element)
    group.set('id', element.get('id'))

    # For non-tactile output, add a group to the diagram.
    # We'll return and insert the label into the group later
    # after everything else has been processed
    if diagram.output_format() != 'tactile':
        parent.append(group)

    # We first go pull out the <m> tags and write them into
    # an HTML file to be processed by MathJax
    # We also want to know if the label is a single lower-case
    # letter so that we can add a letter indicator
    text = element.text
    if text is None:
        text = ''
    else: element.text = evaluate_text(text)
    plain_text = text
    for math in element.findall('m'):
        diagram.add_id(math)
        math_id = math.get('id')
        math.text = evaluate_text(math.text).strip()
        math_text = '\({}\)'.format(math.text)

        # add the label's text to the HTML tree
        div = ET.SubElement(diagram.label_html(), 'div')
        div.set('id', math_id)
        div.text = math_text

        text += math_text
        plain_text += str(math_text)
        if math.tail is not None:
            math.tail = evaluate_text(math.tail)
            text += math.tail
            plain_text += str(math.tail)
    text = text.strip()

    align = util.get_attr(element, 'alignment', 'c')
    if align.startswith('2') or align == 'e':
        align = 'east'

    element.set('alignment', align)
    if element.get('anchor', None) is not None:
        element.set('p', element.get('anchor'))
    element.set('p', util.get_attr(element, 'p', '[0,0]'))

    # if we're making a tactile diagram and the text is a single
    # lower-case letter, we'll add a letter indicator in front
    if diagram.output_format() == 'tactile' and len(plain_text) == 1:
        char_distance = ord(plain_text[0]) - ord('a')
        if char_distance >= 0 and char_distance < 26:
            element.set('add-letter-indicator', 'yes')

# Allow substitutions from the user namespace
def evaluate_text(text):
    tokens = re.split(r"(\${[^}]*})", text)
    return ''.join([str(un.valid_eval(token[2:-1])) if token.startswith('${') else token for token in tokens])

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
    mj_dir = path.absolute() / 'mj_sre'
    mj_dir_str = str(mj_dir)

    if not mj_dir.is_dir():
        from .. import scripts
        scripts.install_mj.main()
            

    mj_command = 'node {}/mj-sre-page.js --{} {} {} > {}'.format(mj_dir_str, format, options, mj_input, mj_output)

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

        if diagram.output_format() == 'tactile':
            position_braille_label(label, diagram, ctm, background_group, 
                                   braille_group, label_tree)
        else:
            position_svg_label(label, diagram, ctm, group, label_tree)

    working_dir.cleanup()

# use this to retrieve elements from the mathjax output
#        div = label_tree.xpath("//html/body/div[@id = '{}']".format(id))[0]

def position_braille_label(element, diagram, ctm, 
                           background_group, braille_group, label_tree):
    group = ET.SubElement(braille_group, 'g')
    group.set('id', element.get('id'))

    # Determine the anchor point p and then adjust it using
    # the alignment and offset
    if element.get('user-coords', 'yes') == 'yes':
        p = ctm.transform(un.valid_eval(element.get('p')))
    else:
        p = un.valid_eval(element.get('p'))
    alignment = util.get_attr(element, 'alignment', 'center')
    displacement = braille_displacement[alignment]

    # TODO:  improve automatic tactile offsets
    offset = util.get_attr(element, 'abs-offset', 'none')
    if offset == 'none':
        offset = [8*(displacement[0] + 0.5), 8*(displacement[1]+0.5)]
    else:
        offset = un.valid_eval(offset)

    offset = [o + 6*np.sign(o) for o in offset]
    if displacement[0] == 0:
        offset[0] += 6
    if displacement[1] == -1:
        offset[1] -= 6
    if alignment == 'n' or alignment == 'north':
        offset[1] += 5

    # 'gap' is the distance, in points, between two braille dots,
    # 1/20th of an inch
    gap = 3.6
    if alignment == 'ha':
        offset = [-4*gap, -30]
    if alignment == 'va':
        offset = [-30, 0]
    if alignment == 'xl':
        offset = [-10, 12]

    if element.get('offset', None) is not None:
        relative_offset = un.valid_eval(element.get('offset'))
        offset = [offset[0] + relative_offset[0],
                  offset[1] + relative_offset[1]]

    p[0] += offset[0]
    p[1] -= offset[1]

    if element.text is not None and len(element.text.strip()) > 0:
        text = element.text.lstrip()
        typeform = [0] * len(text)
        braille_text = louis.translateString(["braille-patterns.cti", "en-us-g2.ctb"], 
                                             text, typeform=[0]*len(text)) 
    else:
        braille_text = ''

    for math in element.findall('m'):
        id = math.get('id')
        div = label_tree.xpath("//html/body/div[@id = '{}']".format(id))[0]

        try:
            insert = div.xpath('mjx-data/mjx-braille')[0]
        except IndexError:
            print('Error in processing label, possibly a LaTeX error: ' + div.text)
            sys.exit()

        math_text = math.text
        regex = re.compile('[a-zA-Z]')
        if len(math_text) == 1 and len(regex.findall(math_text)) > 0:
            # if we want italics, set typeform=[1]
            typeform = [0]
            insert.text = louis.translateString(["braille-patterns.cti", "en-us-g2.ctb"], 
                                                math.text, typeform=typeform)
        else:
            if element.get('nemeth-switch', 'no') == 'yes':
                insert.text = nemeth_on + insert.text + nemeth_off

        braille_text += insert.text

        if math.tail is not None and len(math.tail.strip()) > 0:
            typeform = [0] * len(math.tail)
            braille_text += louis.translateString(["braille-patterns.cti", "en-us-g2.ctb"], 
                                                math.tail, typeform=typeform)

    if element.get('add-letter-indicator', 'no') == 'yes':
        # braille_text = '\u2830' + braille_text
        pass
    w = 5*gap*len(braille_text)
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
    text_element.text = braille_text
    text_element.set('font-family', "Braille29")
    text_element.set('font-size', "29px")

def position_svg_label(element, diagram, ctm, group, label_tree):
    # We're going to put everything inside a group
    label_group = ET.Element('g')

    # Determine the anchor point p and then adjust it using
    # the alignment and offset
    # TODO:  improve auto offsets
    if element.get('user-coords', 'yes') == 'yes':
        p = ctm.transform(un.valid_eval(element.get('p')))
    else:
        p = un.valid_eval(element.get('p'))
    alignment = util.get_attr(element, 'alignment', 'center')
    displacement = alignment_displacement[alignment]

    offset = util.get_attr(element, 'abs-offset', 'none')
    if offset == 'none':
        offset = [8*(displacement[0] + 0.5), 8*(displacement[1]-0.5)]
    else:
        offset = un.valid_eval(offset)

    if element.get('offset', None) is not None:
        relative_offset = un.valid_eval(element.get('offset'))
        offset = [offset[0] + relative_offset[0],
                  offset[1] + relative_offset[1]]

    # A label can have different components comprised of alternating
    # text and MathJax.  We now need to position each component appropriately.
    # Horizontal placement is relatively simple so we'll do it as we pass through
    # the list.  Vertical placement is more involved so well make a pass through
    # the list and record all the relevant data in vertical_data.  An entry in 
    # this list will be the SVG element and how far above and below the baseline
    # the label extends (both measured positively)
    number_m = len(element.findall('m'))
    width = 0
    vertical_data = []
    # Are we starting off with some text?
    if element.text is not None and len(element.text.strip()) > 0:
        text = element.text.lstrip()
        if number_m == 0:
            text = text.rstrip()
        context = cairo_context
        x_bearing, y_bearing, t_width, t_height, x_advance, y_advance = context.text_extents(text)

        text_el = ET.SubElement(label_group, 'text')
        text_el.set('x', util.float2str(-x_bearing))
        text_el.set('font-family', 'sans')
        text_el.set('font-size', str(font_size))
        text_el.text = text
        width += x_advance

        vertical_data.append([text_el, -y_bearing, t_height+y_bearing])

    # Now we'll go through the MathJax labels
    ns = {'svg': 'http://www.w3.org/2000/svg'}
    for num, math in enumerate(element.findall('m')):
        id = math.get('id')
        div = label_tree.xpath("//html/body/div[@id = '{}']".format(id))[0]
        try:
            insert = div.xpath('mjx-data/mjx-container/svg:svg',
                               namespaces=ns)[0]
        except IndexError:
            print('Error in processing label, possibly a LaTeX error: ' + div.text)
            sys.exit()

        # Express dimensions in px for rsvg-convert
        dim_dict = {}
        for attr in ['style', 'width', 'height']:
            fields = insert.get(attr).split()
            dimension = fields[-1]
            index = dimension.find('ex')
            dimension = float(dimension[:index]) * 8
            dim_dict[attr] = dimension
            pts = '{0:.3f}px'.format(dimension)
            fields[-1] = pts
            insert.set(attr, ' '.join(fields))
        label_group.append(insert)   

        insert.set('x', str(width))
        width += dim_dict['width']

        above = dim_dict['height'] + dim_dict['style']
        below = -dim_dict['style']
        vertical_data.append([insert, above, below])

        if math.tail is not None and len(math.tail.strip()) > 0:
            text = ' ' + math.tail.strip()
            if num + 1 == number_m:
                text = text.rstrip()
            context = cairo_context
            x_bearing, y_bearing, t_width, t_height, x_advance, y_advance = context.text_extents(text)
            text_el = ET.SubElement(label_group, 'text')
            width += 3
            text_el.set('x', util.float2str(width))
            text_el.set('font-family', 'sans')
            text_el.set('font-size', str(font_size))
            text_el.text = text
            width += x_advance

            vertical_data.append([text_el, -y_bearing, t_height+y_bearing])

    # Now that we've placed all the components and gathered their vertical
    # data, we'll go back through and adjust the vertical positioning
    above = max([entry[1] for entry in vertical_data])
    below = max([entry[2] for entry in vertical_data])
    height = above + below
    for component_data in vertical_data:
        component = component_data[0]
        if component.tag == 'text':
            component.set('y', util.float2str(above))
        else:
            component.set('y', util.float2str(above-component_data[1]))

    tform = CTM.translatestr(p[0] + offset[0], p[1] - offset[1])

    sc = float(element.get('scale', '1'))
    if sc != 1:
        tform = tform + ' ' + CTM.scalestr(sc, sc)

    rot = element.get('rotate', None)
    if rot is not None:
        tform = tform + ' ' + CTM.rotatestr(float(rot))
    tform = tform + ' ' + CTM.translatestr(width*displacement[0], 
                                           -height*displacement[1])

    group.set('transform', tform)

    # add a white rectangle behind the label, if requested
    if element.get('clear-background', 'no') == 'yes':
        bg_margin = int(element.get('background-margin', '6'))
        rect = ET.SubElement(group, 'rect')
        rect.set('x', str(-bg_margin))
        rect.set('y', str(-bg_margin))
        rect.set('width', str(width+2*bg_margin))
        rect.set('height', str(height+2*bg_margin))
        rect.set('stroke', 'none')
        rect.set('fill', 'white')

    group.append(label_group)
    diagram.add_id(label_group, element.get('expr'))
    group.set('type', 'label')

# add a caption to a tactile diagram in the upper-left corner
#   e.g. "Figure 2.3.4"
def caption(element, diagram, parent, outline_status):
    if diagram.output_format() != "tactile":  
        return
    element.tag = 'label'
    element.set('alignment', 'ne')
    element.set('offset', '(0,10)')
    box = diagram.bbox()
    element.set('p', util.pt2str((box[0], box[3]), spacer=","))
    label(element, diagram, parent)
