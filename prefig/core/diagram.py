import os
import lxml.etree as ET
import numpy as np
import logging
import copy
from . import tags
from . import user_namespace as un
from . import utilities as util
from . import CTM
from . import label

log = logging.getLogger('prefigure')

class Diagram:
    def __init__(self,
                 diagram_element,
                 filename,
                 diagram_number,
                 format,
                 output,
                 publication,
                 suppress_caption,
                 environment):
        self.diagram_element = diagram_element
        self.filename = filename
        self.diagram_number = diagram_number
        self.format = format
        self.output = output
        self.suppress_caption = suppress_caption
        self.environment = environment
        self.caption = ""

        label.init(self.format, self.environment)

        # create the XML tree for the svg output
        svg_uri = "http://www.w3.org/2000/svg"
        xml_uri = "http://www.w3/org/1999/xml"

        nsmap = {None: svg_uri,
                 'xml': xml_uri}
        self.root = ET.Element("svg", nsmap = nsmap)

        self.id_suffix = ['']
        self.add_id(self.root, diagram_element.get('id', 'diagram'))

        # prepare the XML tree for annotations, if there are any
        self.annotations_root = None
        self.default_annotations = []

        # store annotation branches
        self.annotation_branches = {}
        self.annotation_branch_stack = []

        # set up the HTML tree for labels to send to MathJax
        self.label_group_dict = {}
        self.label_html_tree = ET.Element('html')
        self.label_html_body = ET.Element('body')
        self.label_html_tree.append(self.label_html_body)

        # a dictionary for holding shapes
        self.shape_dict = {}

        # each SVG element will have an id, we'll store a count of ids here
        self.ids = {}

        # track reusables that have been added for outlining
        self.reusables = {}

        # a dictionary to remember some network information
        self.network_coordinates = {}

        # stack for managing bounding boxes and clipping
        self.clippaths = []

        # list for legends
        self.legends = []

        # dictionary for label dimensions
        self.label_dims = {}

        

        # read in defaults from publication file
        self.defaults = {}
        if publication is not None:
            for subelement in publication:
                self.defaults[subelement.tag] = subelement

        if self.defaults.get('macros', None) is not None:
            label.add_macros(self.defaults.get('macros').text)

    def add_legend(self, legend):
        self.legends.append(legend)
        
    def add_label(self, element, group):
        self.label_group_dict[element] = [group, copy.deepcopy(self.ctm())]

    def set_caption(self, text):
        self.caption = text

    def caption_suppressed(self):
        return self.suppress_caption

    def get_label_group(self, element):
        return self.label_group_dict.get(element)

    def register_label_dims(self, element, dimensions):
        self.label_dims[element] = dimensions

    def get_label_dims(self, element):
        dims = self.label_dims.get(element, None)
        if dims is None:
            log.error(f"Cannot find dimensions for a label")
        return dims

    def add_id(self, element, id = None):
        # We'll add an id attribute to the SVG element
        # If not specified, the id is obtained from a count of elements
        element.set('id', self.find_id(element, id))

    def find_id(self, element, id = None):
        # We'll add an id attribute to the SVG element
        # If not specified, the id is obtained from a count of elements
        suffix = ''.join(self.id_suffix)
        if id is None:
            self.ids[element.tag] = self.ids.get(element.tag, -1) + 1
            return element.tag+'-'+str(self.ids[element.tag])+suffix
        else:
            return id + suffix

    def append_id_suffix(self, element):
        return self.find_id(element, element.get('id', None))
    
    def output_format(self):
        return self.format

    def set_output_format(self, format):
        self.format = format

    # get the HTML tree so that we can add text for labels
    def label_html(self):
        return self.label_html_body

    def save_network_coordinates(self, network, coordinates):
        self.network_coordinates[network] = coordinates

    def get_network_coordinates(self, network):
        return self.network_coordinates[network]

    # transform a point into SVG coordinates
    def transform(self, p):
        ctm, b = self.ctm_stack[-1]
        try:
            return ctm.transform(p)
        except:
            log.error(f"Unable to apply coordinate transform to {p}")
            return np.array([0,0])

    def inverse_transform(self, p):
        ctm, b = self.ctm_stack[-1]
        try:
            return ctm.inverse_transform(p)
        except:
            log.error(f"Unable to apply inverse coordinate transform to {p}")
            return np.array([0,0])

    def begin_figure(self):
        # set up the dimensions of the diagram in SVG coordinates
        dims = self.diagram_element.get('dimensions')
        try:
            if dims is None:
                width = un.valid_eval(self.diagram_element.get('width'))
                height = un.valid_eval(self.diagram_element.get('height'))
            else:
                width, height = un.valid_eval(dims)
        except:
            log.error("Unable to parse the dimensions of this diagram")
            return

        margins = self.diagram_element.get('margins', '[0,0,0,0]')
        if self.format == 'tactile':
            margins = self.diagram_element.get('tactile-margins', margins)
        try:
            margins = un.valid_eval(margins)
        except:
            log.error("Unable to parse margins={element.get('margins')}")
            return
        if not isinstance(margins, np.ndarray):
            margins = [margins] * 4
        self.margins = margins

        ctm = CTM.CTM()
        # tactile diagrams will be embossed on 11.5"x11" paper
        if self.format == 'tactile':
            total_width = width + margins[0] + margins[2]
            total_height = height + margins[1] + margins[3]
            diagram_aspect = total_width / total_height
            page_aspect = 10.5 / 8.8  # area available for graphics

            if diagram_aspect >= page_aspect:
                s = 756 / total_width
                lly = s * total_height + 79.2
                self.centerline = 378 + 36 # half of 756 + margin
            else:
                s = 633.6 / total_height
                lly = 712.8
                self.centerline = s*total_width/2 + 36
            self.bottomline = lly
            ctm.translate(36, lly)
            ctm.scale(s, -s)
            ctm.translate(margins[0], margins[1])
            self.root.set("width", "828")
            self.root.set("height", "792")

            # bounding rectangle
            '''
            rect = ET.SubElement(self.root, 'rect')
            rect.set('x', '0')
            rect.set('y', '0')
            rect.set('width', '828')
            rect.set('height', '792')
            rect.set('stroke', 'black')
            rect.set('fill', 'none')
            '''
        else:
            w = width + margins[0]+margins[2]
            h = height + margins[1]+margins[3]
            self.root.set("width", str(w))
            self.root.set("height", str(h))

            # initialize the CTM and push it onto the CTM stack
            ctm.translate(0, height + margins[1] + margins[3])
            ctm.scale(1,-1)
            ctm.translate(margins[0], margins[1])

        bbox = [0,0,width,height]
        un.enter_namespace('bbox', bbox)
        self.ctm_stack = [[ctm, bbox]]

        # initialize the SVG element 'defs' and add the clipping path
        self.defs = ET.SubElement(self.root, 'defs')

        clippath = ET.Element('clipPath')
        ET.SubElement(clippath, 'rect',
                      attrib={
                             'x': util.float2str(margins[0]),
                             'y': util.float2str(margins[3]),
                             'width': util.float2str(width),
                             'height': util.float2str(height)
                         })
        self.push_clippath(clippath)

    def push_clippath(self, clippath):
        self.defs.append(clippath)
        self.add_id(clippath)
        self.clippaths.append(clippath.get('id'))

    def pop_clippath(self):
        self.clippaths.pop(-1)

    def get_clippath(self):
        return self.clippaths[-1]
    
    def place_labels(self):
        label.place_labels(self,
                           self.filename,
                           self.root,
                           self.label_group_dict,
                           self.label_html_tree)

        for legend in self.legends:
            legend.place_legend(self)

        if self.format == 'tactile':
            # first we'll place the caption
            if len(self.caption) == 0:
                caption = label.nemeth_on
            else:
                self.caption = label.braille_translator.translate(
                    self.caption,
                    [0] * len(self.caption)
                )
                caption = self.caption + ' ' + label.nemeth_on

            gap = 3.6  # space between embossing dots

            text_element = ET.SubElement(self.root, 'text')
            text_element.text = caption
            text_element.set("x", "144")  # start in cell 7 per BANA, was 36
            text_element.set("y", "50.4")
            text_element.set('font-family', "Braille29")
            text_element.set('font-size', "29px")

            # add a nemeth off indicator at the bottom
            text_element = ET.SubElement(self.root, 'text')
            text_element.text = label.nemeth_off
            text_element.set('x', '36')
            y = self.bottomline + 12 * gap  # bottom of diagram + blank line
            y = label.snap_to_embossing_grid(y)
            text_element.set('y', util.float2str(y))
            text_element.set('font-family', "Braille29")
            text_element.set('font-size', "29px")
        
    def end_figure(self):
        # form the output filenames
        if self.diagram_number is None:
            suffix = ''
        else:
            suffix = '-' + str(self.diagram_number)

        input_dir = os.path.dirname(self.filename)
        basename = os.path.basename(self.filename)[:-4] + suffix
        output_dir = os.path.join(input_dir, 'output')
        out = os.path.join(output_dir, basename)
        try:
            if not os.path.exists(output_dir):
                os.mkdir(output_dir)
            with ET.xmlfile(out + '.svg', encoding='utf-8') as xf:
                xf.write(self.root, pretty_print=True)
        except:
            log.error(f"Unable to write SVG at {out+'.svg'}")
            return

        if self.annotations_root is not None:
            diagram = ET.Element('diagram')
            diagram.append(self.annotations_root)
            et = ET.ElementTree(diagram)
            if self.environment == "pretext":
                output_file = out + '-annotations.xml'
            else:
                output_file = out + '.xml'
            try:
                et.write(output_file, pretty_print=True)
            except:
                log.error(f"Unable to write annotations in {output_file}")
                return
        else:
            try:
                os.remove(out+'.xml')
            except OSError:
                pass

            try:
                os.remove(out+'-annotations.xml')
            except OSError:
                pass


    # If we only want a string, we assemble the XML tree
    # consisting of the SVG and annotations and return as a string
    def end_figure_to_string(self):
        svg_string = ET.tostring(self.root).decode('utf-8')
        annotation_string = None
        if self.annotations_root is not None:
            diagram = ET.Element("diagram")
            diagram.append(self.annotations_root)
            annotation_string = ET.tostring(diagram).decode('utf-8')
        return svg_string, annotation_string

    # Here we parse the children of the given XML element
    # Resulting SVG elements will be placed below root
    def parse(self, element = None, root = None, outline_status = None):
        if element is None:
            element = self.diagram_element
        if root is None:
            root = self.root
        # strip out the namespace prefix
        element.tag = ET.QName(element).localname
        # We allow an element's attributes to be rewritten depending on
        # the format.  For instance, tactile diagrams sometimes require
        # modified attributes
        prefix = self.format + '-'
        for child in element:
            if child.tag is ET.Comment:
                continue
            child.tag = ET.QName(child).localname
            # we're publicly using 'at' rather than 'id' for handles
            if child.get('at') is not None:
                child.set('id', child.get('at'))
            # see if the publication flie has any defaults
            defaults = self.defaults.get(child.tag, None)
            if defaults is not None:
                for attr, value in defaults.attrib.items():
                    if child.get(attr, None) is None:
                        child.set(attr, value)
            # replace any format-specific attributes
            for attr, value in child.items():
                if attr.startswith(prefix):
                    child.set(attr[len(prefix):], value)
            try:
                tags.parse_element(child, self, root, outline_status)
            except Exception as e:
                log.error(f"Error in parsing element {child.tag}")
                log.error(str(e))
                return
            if (
                    child.get('annotate', 'no') == 'yes' and
                    outline_status != 'add_outline'
            ):
                tag = child.tag
                if tag != 'group' and tag != 'repeat':
                    annotation = ET.Element('annotation')
                    for attrib in ['id', 'text', 'sonify', 'circular', 'speech']:
                        if child.get(attrib, None) is not None:
                            annotation.set(attrib, child.get(attrib))
                    if annotation.get('text', None) is not None:
                        annotation.set('text', label.evaluate_text(annotation.get('text')))
                    if annotation.get('speech', None) is not None:
                        annotation.set('speech', label.evaluate_text(annotation.get('speech')))
                    self.add_annotation_to_branch(annotation)

    def ctm(self):
        return self.ctm_stack[-1][0]

    def bbox(self):
        return self.ctm_stack[-1][1]

    def ctm_bbox(self):
        return self.ctm_stack[-1]

    def get_margins(self):
        return self.margins

    def push_ctm(self, ctm_bbox):
        self.ctm_stack.append(ctm_bbox)

    def pop_ctm(self):
        return self.ctm_stack.pop(-1)

    def add_reusable(self, element):
        if self.has_reusable(element.get('id', 'none')):
            return
        self.defs.append(element)
        self.reusables[element.get('id')] = element

    def has_reusable(self, reusable):
        return self.reusables.get(reusable, None) is not None
    
    def get_reusable(self, reusable):
        return self.reusables.get(reusable, None)
    
    def push_id_suffix(self, suffix):
        self.id_suffix.append(suffix)

    def pop_id_suffix(self):
        self.id_suffix.pop(-1)

    def get_root(self):
        return self.root

    # when a graphical component is outlined, we first add the component's path
    # to <defs> so that it can be reused, then we stroke it with a thick white
    def add_outline(self, element, path, parent, outline_width = None):
        if outline_width is None:
            if self.output_format() == 'tactile':
                outline_width = 18
            else:
                outline_width = 4

        stroke = path.attrib.pop('stroke', 'none')
        width = path.attrib.pop('stroke-width', '1')
        fill = path.attrib.pop('fill', 'none')
        path.attrib.pop('stroke-dasharray', 'none')

        self.add_id(element, element.get('id'))
        outline_id = element.get('id') + '-outline'
        path.set('id', outline_id)
        self.add_reusable(path)

        use = ET.SubElement(parent, 'use', attrib={       
            'fill': fill,
            'stroke-width': str(int(width) + outline_width),
            'stroke': 'white',
            'href': r'#' + outline_id
        }
        )
        # We need to be careful with arrow heads.  The references to
        # the arrow heads are in path, which is now a reusable.  We will
        # retrieve the references, change the references to point to the
        # outlined arrow heads, and add them to the use element.  In the
        # finish_outline function, we'll remove the references from the
        # reusable since otherwise we'll only see the original arrow heads
        for marker in ['marker-end', 'marker-start', 'marker-mid']:
            reference = path.get(marker, None)
            if reference is not None:
                reference = reference.replace(')', '-outline)')
                use.set(marker, reference)

    # after the outline of a graphical component is added, we then add the 
    # component itself on top of the outline
    def finish_outline(self, element, stroke, thickness, fill, parent):
        use = ET.SubElement(parent, 'use', attrib={
            'id': element.get('id', 'none'),
            'fill': str(fill),
            'stroke-width': str(thickness),
            'stroke': str(stroke),
            'stroke-dasharray': element.get('dash', 'none'),
            'href': r'#' + element.get('id', 'none') + '-outline'
        }
        )
        # labeled points and angle markers are in a <g> with the 
        # point's id.  To avoid duplicate id's, we'll remove the
        # id from the graphical component
        if element.get('id', 'none') == parent.get('id', 'none'):
            use.attrib.pop('id')

        # We have to clean up the arrow heads.  The references to the
        # arrow heads are in the reusable so we'll retrieve them and
        # and include them with the use element.
        reusable = self.get_reusable(element.get('id') + '-outline')
        for marker in ['marker-start', 'marker-end', 'marker-mid']:
            if reusable.get(marker, 'none') != 'none':
                use.set(marker, reusable.get(marker))
                reusable.attrib.pop(marker)

    def initialize_annotations(self):
        if self.annotations_root is not None:
            log.error('Annotations need to be in a single tree')
            return

        self.annotations_root = ET.Element('annotations')

    def add_default_annotation(self, annotation):
        self.default_annotations.append(annotation)

    def get_default_annotations(self):
        return self.default_annotations

    def get_annotations_root(self):
        return self.annotations_root

    def add_annotation(self, annotation):
        self.annotations_root.append(annotation)

    def push_to_annotation_branch(self, annotation):
        if len(self.annotation_branch_stack) == 0:
            self.annotation_branches[annotation.get('id')] = annotation
        else:
            self.add_annotation_to_branch(annotation)
        self.annotation_branch_stack.append(annotation)

    def pop_from_annotation_branch(self):
        self.annotation_branch_stack.pop(-1)

    def add_annotation_to_branch(self, annotation):
        if len(self.annotation_branch_stack) == 0:
            self.annotation_branches[annotation.get('id')] = annotation
            return
        self.annotation_branch_stack[-1].append(annotation)
        annotation.set('id', self.append_id_suffix(annotation))

    def get_annotation_branch(self, id):
        return self.annotation_branches.pop(id, None)

    def recall_shape(self, shape_id):
        return self.shape_dict.get(shape_id, None)

    def add_shape(self, shape):
        self.defs.append(shape)
        id = shape.get('id', shape.get('at'))
        self.shape_dict[id] = shape
        
    def get_shape(self, shape_id):
        shape = self.recall_shape(shape_id)
        if shape is not None:
            return shape
        
        paths = self.root.findall('path')
        for path in paths:
            if path.get('id', None) == shape_id:
                return path

        log.error(f"We cannot find a <shape> with id={shape_id}")
        return None
