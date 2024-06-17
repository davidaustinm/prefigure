import os
import sys
import lxml.etree as ET
import tags
import user_namespace as un
import CTM
import label


class Diagram:
    def __init__(self, diagram_element, filename,
                 diagram_number, format):
        self.diagram_element = diagram_element
        self.filename = filename
        self.diagram_number = diagram_number
        self.format = format

        # create the XML tree for the svg output
        svg_uri = "http://www.w3.org/2000/svg"
        xlink_uri = "http://www.w3.org/1999/xlink"
        self.href = ET.QName(xlink_uri, "href")

        nsmap = {None: svg_uri,
                 'xlink': xlink_uri}
        self.root = ET.Element("svg", nsmap = nsmap)

        self.id_suffix = ['']
        self.add_id(self.root, diagram_element.get('id', 'diagram'))

        # prepare the XML tree for annotations, if there are any
        self.annotations_root = None
        self.default_annotations = []

        # set up the HTML tree for labels to send to MathJax
        self.label_group_dict = {}
        self.label_html_tree = ET.Element('html')
        self.label_html_body = ET.Element('body')
        self.label_html_tree.append(self.label_html_body)

        # each SVG element will have an id, we'll store a count of ids here
        self.ids = {}

        # track reusables that have been added for outlining
        self.reusables = {}

        # a dictionary to remember some network information
        self.network_coordinates = {}

    def add_label(self, element, group):
        self.label_group_dict[element] = [group, self.ctm()]

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

    def output_format(self):
        return self.format

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
        return ctm.transform(p)

    def begin_figure(self):
        # set up the dimensions of the diagram in SVG coordinates
        dims = self.diagram_element.get('dimensions')
        if dims is None:
            width = un.valid_eval(self.diagram_element.get('width'))
            height = un.valid_eval(self.diagram_element.get('height'))
        else:
            width, height = un.valid_eval(dims)

        margins = un.valid_eval(self.diagram_element.get('margins', '[0]*4'))
        if not isinstance(margins, list):
            margins = [margins] * 4

        # tactile diagrams will be embossed on 11.5"x11" paper
        if self.format == 'tactile':
            aspect_ratio = height / width
            pagemargin = 1 * 72
            pagewidth = 11.5 * 72 - 2*pagemargin
            pageheight = 11 * 72 - 2*pagemargin
            if aspect_ratio * pagewidth <= pageheight:
                width = pagewidth
                height = aspect_ratio * pagewidth
                margins = [pagemargin, (pageheight - height)/2 + pagemargin,
                           pagemargin, (pageheight - height)/2 + pagemargin]
            else:
                height = pageheight
                width = pageheight / aspect_ratio
                margins = [(pagewidth - width)/2 + pagemargin, pagemargin,
                           (pagewidth - width)/2 + pagemargin, pagemargin]

        w = width + margins[0]+margins[2]
        h = height + margins[1]+margins[3]
        self.root.set("width", str(w))
        self.root.set("height", str(h))

        # initialize the CTM and push it onto the CTM stack
        ctm = CTM.CTM()
        ctm.translate(0, height + margins[1] + margins[3])
        ctm.scale(1,-1)
        ctm.translate(margins[0], margins[1])
        bbox = [0,0,width,height]
        un.valid_eval(str(bbox), 'bbox')
        self.ctm_stack = [[ctm, bbox]]

        # initialize the SVG element 'defs' and add the clipping path
        self.defs = ET.SubElement(self.root, 'defs')

        clippath = ET.SubElement(self.defs, 'clipPath',
                                 attrib={'id': 'clip-to-bounding-box'})

        ET.SubElement(clippath, 'rect',
                      attrib={
                             'x': str(margins[0]),
                             'y': str(margins[3]),
                             'width': str(width),
                             'height': str(height)
                         })

    def place_labels(self):
        label.place_labels(self,
                           self.filename,
                           self.root,
                           self.label_group_dict,
                           self.label_html_tree)

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
        if not os.path.exists(output_dir):
            os.mkdir(output_dir)
        with ET.xmlfile(out + '.svg', encoding='utf-8') as xf:
            xf.write(self.root, pretty_print=True)

        if self.annotations_root is not None:
            diagram = ET.Element('diagram')
            diagram.append(self.annotations_root)
            et = ET.ElementTree(diagram)
            et.write(out+'.xml', pretty_print=True)

    # Here we parse the children of the given XML element
    # Resulting SVG elements will be placed below root
    def parse(self, element = None, root = None, outline_status = None):
        if element is None:
            element = self.diagram_element
        if root is None:
            root = self.root

        # We allow an element's attributes to be rewritten depending on
        # the format.  For instance, tactile diagrams sometimes require
        # modified attributes
        prefix = self.format + '-'
        for child in element:
            for attr, value in child.items():
                if attr.startswith(prefix):
                    child.set(attr[len(prefix):], value)

            tags.parse_element(child, self, root, outline_status)

    def ctm(self):
        return self.ctm_stack[-1][0]

    def bbox(self):
        return self.ctm_stack[-1][1]

    def ctm_bbox(self):
        return self.ctm_stack[-1]

    def push_ctm(self, ctm_bbox):
        self.ctm_stack.append(ctm_bbox)

    def pop_ctm(self):
        return self.ctm_stack.pop(-1)

    def add_reusable(self, element):
        if self.reusables.get(element.get('id'), False):
            return
        self.defs.append(element)
        self.reusables[element.get('id')] = True

    def has_reusable(self, reusable):
        return self.reusables.get(reusable, False)

    def push_id_suffix(self, suffix):
        self.id_suffix.append(suffix)

    def pop_id_suffix(self):
        self.id_suffix.pop(-1)

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

        ET.SubElement(parent, 'use', attrib={       
            'fill': fill,
            'stroke-width': str(int(width) + outline_width),
            'stroke': 'white',
            self.href: r'#' + outline_id
        }
        )

    # after the outline of a graphical component is added, we then add the 
    # component itself on top of the outline
    def finish_outline(self, element, stroke, thickness, fill, parent):
        ET.SubElement(parent, 'use', attrib={
            'id': element.get('id', 'none'),
            'fill': str(fill),
            'stroke-width': str(thickness),
            'stroke': str(stroke),
            'stroke-dasharray': element.get('dash', 'none'),
            self.href: r'#' + element.get('id') + '-outline'
        }
        )

    def initialize_annotations(self):
        if self.annotations_root is not None:
            print('Annotations need to be in a single tree')
            sys.exit()

        self.annotations_root = ET.Element('annotations')

    def add_default_annotation(self, annotation):
        self.default_annotations.append(annotation)

    def get_default_annotations(self):
        return self.default_annotations

    def get_annotations_root(self):
        return self.annotations_root

    def add_annotation(self, annotation):
        self.annotations_root.append(annotation)
