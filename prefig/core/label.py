import os
import sys
import re
import math
import inspect
from pathlib import Path
import numpy as np
import lxml.etree as ET
from . import utilities as util
from . import CTM
from . import user_namespace as un
import tempfile
try:
    import cairo
except:
    print('Error importing Python package cairo, which is required for non-mathemaical labels.')
    print('See the PreFigure installation instructions at https://prefigure.org')

# These are tags that can occur in a label
label_tags = {'it', 'b', 'newline'}

def is_label_tag(tag):
    return tag in label_tags

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
    'hat': [0, 0],    # top horizontal axis label
    'va': [-1, -0.5],  # vertical axis label
    'va': [0, -0.5],   # right vertical axis label
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

try:
    surface = cairo.SVGSurface(None, 200, 200)
    context = cairo.Context(surface)
    context.select_font_face('sans')
    font_size = 14
    context.set_font_size(font_size)
    cairo_context = context
except:
    # If cairo is not installed, we've already given a warning so we'll keep going
    pass

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
    diagram.add_id(element, element.get('id'))
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

    if not (mj_dir / 'mj-sre-page.js').exists():
        from .. import scripts
        scripts.install_mj.main()
            

    mj_command = 'node {}/mj-sre-page.js --{} {} {} > {}'.format(mj_dir_str, format, options, mj_input, mj_output)

    os.system(mj_command)
    label_tree = ET.parse(mj_output)

    # for braille output, we'll create a group to hold all the labels
    # and their clear backgrounds and add it at the end of the diagram
    if diagram.output_format() == 'tactile':
        try:
            global louis
            import louis
        except:
            print('Failed to import louis so we cannot make braille labels')
            print('See the installation instructions at https://prefigure.org')
            print('The rest of the diagram will still be built.')
            return
        background_group = ET.SubElement(root, 'g')
        background_group.set('id', 'background-group')
        braille_group = ET.SubElement(root, 'g')
        braille_group.set('id', 'braille-group')

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
    if alignment == 'hat':
        offset = [-4*gap, 30]
    if alignment == 'va':
        offset = [-30, 0]
    if alignment == 'var':
        offset = [30, 0]
    if alignment == 'xl':  
        offset = [-10, 12]

    if element.get('offset', None) is not None:
        relative_offset = un.valid_eval(element.get('offset'))
        offset = [offset[0] + relative_offset[0],
                  offset[1] + relative_offset[1]]

    p[0] += offset[0]
    p[1] -= offset[1]

    # let's assemble the different pieces
    row = [[element.text, 'plain']]
    text_elements = [row]
    
    for el in element:
        if el.tag == 'newline':
            row = []
            text_elements.append(row)
        if el.tag == 'm':
            row.append(el)
        if el.tag == 'it':
            row.append([el.text, 'it'])
            for child in el:
                if child.tag != 'b':
                    print(f"<{child.tag}> is not allowed inside a <it>")
                    continue
                row.append([child.text, 'it'])
                row.append([child.tail, 'it'])
        
        if el.tag == 'b':
            row.append([el.text, 'b'])
            for child in el:
                if child.tag != 'it':
                    print(f"<{child.tag}> is not allowed inside a <b>")
                    continue
                row.append([child.text, 'b'])
                row.append([child.tail, 'b'])
        row.append([el.tail, 'plain'])

    # Let's make another pass through the elements removing
    # empty text and adding whitespace
    for num, row in enumerate(text_elements):
        # remove empty text
        new_row = []
        for el in row:
            if isinstance(el, list): # is this a text
                text = el[0]
                if text is not None:
                    text = text.strip()
                    if len(text) > 0:
                        if (
                                len(new_row) > 0 and
                                new_row[-1][1] == el[1]
                        ):
                            new_row[-1][0] += ' ' + text
                        else:
                            new_row.append([text, el[1]])
            else: # otherwise it's an <m>
                new_row.append(el)
        text_elements[num] = new_row

    typeform_dict = {'plain':0, 'it':1, 'b':4}
    space = louis.translateString(
        ["braille-patterns.cti", "en-us-g2.ctb"],
        ' ',
        typeform=[0]
    )
    # translate braille strings not in an <m>
    for num, row in enumerate(text_elements):
        row_text = ''
        while len(row) > 0:
            el = row.pop(0)
            if isinstance(el, list):
                text = el[0]
                if len(row) > 0:
                    text += ' '
                typeform = [typeform_dict[el[1]]] * len(text)
                braille_text = louis.translateString(
                    ["braille-patterns.cti", "en-us-g2.ctb"],
                    text,
                    typeform=typeform
                )
                row_text += braille_text
            else:
                m_tag_id = el.get('id')

                div = label_tree.xpath("//html/body/div[@id = '{}']".format(m_tag_id))[0]

                try:
                    insert = div.xpath('mjx-data/mjx-braille')[0]
                except IndexError:
                    print('Error in processing label, possibly a LaTeX error: ' + div.text)
                    sys.exit()
                row_text += insert.text
                if len(row) > 0:
                    row_text += space
        text_elements[num] = row_text

    interline = 28.8  # 0.4 inches
    width = 5 * gap * max([len(row) for row in text_elements])
    height = 5 * gap + interline * (len(text_elements) - 1)

    diagram.register_label_dims(element, (width, height))
    
    p[0] += width*displacement[0]
    p[1] -= height*displacement[1]

    # snap the point onto the 20dpi embossing grid
    p = [3.6 * round(c/3.6) for c in p]

    tform = CTM.translatestr(*p)
    group.set('transform', tform)

    # add a white background behind the label
    bg_margin = 9  # 1/8th of a inch
    rect = ET.SubElement(background_group, 'rect')
    rect.set('id', element.get('id') + '-background')
    rect.set('x', util.float2str(p[0]-bg_margin))
    rect.set('y', util.float2str(p[1]-height-bg_margin))
    #rect.set('x', util.float2str(-bg_margin))
    #rect.set('y', util.float2str(-height-bg_margin))
    rect.set('width', util.float2str(width+2*bg_margin))
    rect.set('height', util.float2str(height+2*bg_margin))
    rect.set('stroke', 'none')
    rect.set('fill', 'white')

    # show the lower-left corner point for debugging
    circle = ET.Element('circle')
    circle.set('cx', util.float2str(p[0]))
    circle.set('cy', util.float2str(p[1]))
    circle.set('r', '3')
    circle.set('fill', '#00f')
#    group.append(circle)

    # Now add the labels
    justify = element.get('justify', 'center')
    '''
    x = p[0] 
    y = p[1] - height + 5*gap
    '''
    x = 0
    y = -height + 5*gap
    for el in text_elements:
        text_element = ET.SubElement(group, 'text')
        x_line = x
        if justify == 'right':
            x_line = x + width - 5*gap*len(el)
        if justify == 'center':
            x_line = x+ (width - 5*gap*len(el))/2
            x_line = gap * round(x_line/gap)
        text_element.set('x', util.float2str(x_line))
        text_element.set('y', util.float2str(y))
        text_element.text = el
        text_element.set('font-family', "Braille29")
        text_element.set('font-size', "29px")
        y += interline


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


    # A label can have rows consisting of different components
    # comprised of text, with italics and bold, and <m> tags.
    # Our first task is to extract all of these components and measure them

    std_font_face = ['sans', 14, 'normal', 'normal']
    it_font_face =  ['sans', 14, 'italic', 'normal']
    b_font_face  =  ['sans', 14, 'normal', 'bold']
    it_b_font_face  =  ['sans', 14, 'italic', 'bold']

    label = element

    # We first pass through all the text and elements in the label
    # extracting the text and the font information

    row = [(label.text, std_font_face)]
    text_elements = [row]
    
    for el in label:
        if el.tag == 'newline':
            row = []
            text_elements.append(row)
        if el.tag == 'm':
            row.append(el)
        if el.tag == 'it':
            row.append((el.text, it_font_face))
            for child in el:
                if child.tag != 'b':
                    print(f"<{child.tag}> is not allowed inside a <it>")
                    continue
                row.append((child.text, it_b_font_face))
                row.append((child.tail, it_font_face))
        
        if el.tag == 'b':
            row.append((el.text, b_font_face))
            for child in el:
                if child.tag != 'it':
                    print(f"<{child.tag}> is not allowed inside a <b>")
                    continue
                row.append((child.text, it_b_font_face))
                row.append((child.tail, b_font_face))
        row.append((el.tail, std_font_face))

    # Let's make another pass through the elements removing
    # empty text and adding whitespace
    for num, row in enumerate(text_elements):
        new_row = []
        begin_space = False
        for element in row:
            if isinstance(element, tuple): # is this a text
                text = element[0]
                if text is not None:
                    text = text.strip()
                    if len(text) > 0:
                        new_row.append(mk_text_element(text,
                                                       element[1],
                                                       label_group))
                
            else: # otherwise it's an <m>
                new_row.append(mk_m_element(element,
                                            label_tree,
                                            label_group))
                begin_space = True
        text_elements[num] = new_row

    # let's go through each row and find the dimensions
    cairo_context.select_font_face('sans',
                                   cairo.FontSlant.NORMAL,
                                   cairo.FontWeight.NORMAL
                                   )
    space = cairo_context.text_extents(' ')[4]
    interline = un.valid_eval(label.get('interline', '3'))
    text_dimensions = []
    for num, row in enumerate(text_elements):
        width = 0
        above = [0]
        below = [0]
        for el in row:
            width += el[1]
            above.append(el[2])
            below.append(el[3])
        above = max(above)
        below = max(below)
        height = above + below
        width += (len(row)-1) * space
        if num == len(text_elements) - 1:
            interline = 0
        text_dimensions.append([width,
                                height+interline,
                                above,
                                below])

    # let's find the dimension of the bounding box
    width = max([row[0] for row in text_dimensions])
    height = sum([row[1] for row in text_dimensions])

    diagram.register_label_dims(label, (width, height))

    # finally, we set the location of each subelement
    justify = label.get('justify', 'center')
    y_location = 0
    for row, dims in zip(text_elements, text_dimensions):
        row_width = dims[0]
        row_above = dims[2]
        x_location = 0
        if justify == 'center':
            x_location = (width - row_width)/2
        if justify == 'right':
            x_location = width - row_width
        for el in row:
            component = el[0]
            component.set('x', util.float2str(x_location))
            x_location += el[1] + space
            if component.tag == 'text':
                component.set('y',
                              util.float2str(y_location+row_above)
                              )
            else:
                component.set('y',
                              util.float2str(y_location+
                                             row_above-
                                             el[2])
                              )
        y_location += dims[1]
                


    # Now we find the coordinate transform for the label_group
    tform = CTM.translatestr(p[0] + offset[0], p[1] - offset[1])

    sc = float(label.get('scale', '1'))
    if sc != 1:
        tform = tform + ' ' + CTM.scalestr(sc, sc)

    rot = label.get('rotate', None)
    if rot is not None:
        tform = tform + ' ' + CTM.rotatestr(float(rot))
    tform = tform + ' ' + CTM.translatestr(width*displacement[0], 
                                           -height*displacement[1])

    group.set('transform', tform)

    # add a white rectangle behind the label, if requested
    if label.get('clear-background', 'no') == 'yes':
        bg_margin = int(label.get('background-margin', '6'))
        rect = ET.SubElement(group, 'rect')
        rect.set('x', str(-bg_margin))
        rect.set('y', str(-bg_margin))
        rect.set('width', str(width+2*bg_margin))
        rect.set('height', str(height+2*bg_margin))
        rect.set('stroke', 'none')
        rect.set('fill', 'white')

    group.append(label_group)
    diagram.add_id(label_group, label.get('expr'))
#    group.set('type', 'label')


def mk_text_element(text_str, font_face, label_group):
    text_el = ET.SubElement(label_group, 'text')
    text_el.text = text_str
    text_el.set('font-family', font_face[0])
    text_el.set('font-size', str(font_face[1]))
    if font_face[2] == 'italic':
        text_el.set('font-style', 'italic')
    if font_face[3] == 'bold':
        text_el.set('font-weight', 'bold')

    if font_face[2] == 'italic':
        font_slant = cairo.FontSlant.ITALIC
    else:
        font_slant = cairo.FontSlant.NORMAL
    if font_face[3] == 'bold':
        font_weight = cairo.FontWeight.BOLD
    else:
        font_weight = cairo.FontWeight.NORMAL
    cairo_context.select_font_face(font_face[0],
                                   font_slant,
                                   font_weight)
    cairo_context.set_font_size(font_face[1])
    extents = cairo_context.text_extents(text_str)
    y_bearing = extents[1]
    t_height  = extents[3]
    xadvance  = extents[4]
    return [text_el, xadvance, -y_bearing, t_height+y_bearing]


def mk_m_element(m_tag, label_tree, label_group):
    m_tag_id = m_tag.get('id')
    div = label_tree.xpath("//html/body/div[@id = '{}']".format(m_tag_id))[0]
    ns = {'svg': 'http://www.w3.org/2000/svg'}
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

    width = dim_dict['width']

    above = dim_dict['height'] + dim_dict['style']
    below = -dim_dict['style']
    return [insert, width, above, below]


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

    
