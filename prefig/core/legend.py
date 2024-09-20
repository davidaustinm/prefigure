import lxml.etree as ET
import copy
import numpy as np
from . import user_namespace as un
from . import utilities as util
from . import tags
from . import label
from . import point
from . import CTM


# This feels a little hack-y and I'm somewhat ambivalent about legends
# as they may not be a good practice for tactile diagrams.

class Legend:
    def __init__(self, element, diagram, parent, outline_status):
        self.element = element
        self.diagram = diagram
        self.parent = parent
        self.outline = outline_status == 'add_outline'
        self.tactile = diagram.output_format() == 'tactile'

        # we will put everything in a group and apply a final transform to it
        self.group = ET.Element('g')
        parent.append(self.group)

        # register this legend so we an come back to it
        diagram.add_legend(self)

        # let's get some basic data about the legend
        # self.def_anchor holds the SVG coordinates of the anchor
        anchor_str = element.get('anchor', '(bbox[2],bbox[3])')
        user_anchor = un.valid_eval(anchor_str)
        self.def_anchor = diagram.transform(user_anchor)
        
        alignment = element.get('alignment')
        if alignment == 'e':
            element.set('alignment', 'east')
        alignment = util.get_attr(element, 'alignment', 'c')
        self.displacement = label.alignment_displacement[alignment]

        # let's go through each of the child and set up the labels
        self.li_dict = {}
        key_width = 0
        point_width = 10

        # length of line as a key
        self.line_width = 24
        if self.tactile:
            self.line_width = 45

        dummy_group = ET.Element('g')
        for num, li in enumerate(element):
            if li.tag != 'item':
                print(f"{li.tag} is not allowed inside a <legend>")
                continue

            # first we'll create the label
            label_el = copy.deepcopy(li)
            label_el.tag = 'label'
            label_el.set('id', f"legend-label-{num}")
            label_el.set('alignment', 'se')
            label_el.set('anchor', element.get('anchor', anchor_str))
            label_el.set('abs-offset', '(0,0)')
            label_el.set('justify', 'left')
            label.label(label_el, diagram, dummy_group, None)
            dummy_group.clear()
            
            # and now the key
            ref = label_el.get('ref')
            search = f'//*[@id="{ref}"]'
            references = diagram.diagram_element.xpath(search)
            if len(references) != 1:
                print(f"{ref} should refer to exactly one element")
                continue
            key = references[0]
            if key.tag == 'point':
                key = copy.deepcopy(key)
                key.set('p', anchor_str)
                key.set('size', '4')
                key.set('id', f"legend-point-{num}")
                key_width = max(key_width, point_width)
            else:
                fill =  key.get('fill')
                if fill is None or fill == 'none':
                    key_el = ET.Element('line')
                    key_el.set('stroke', key.get('stroke'))
                    dash = key.get('dash', None)
                    if dash is not None:
                        key_el.set('stroke-dasharray', dash)
                    key_width = max(key_width, self.line_width)
                    key = key_el
                else:
                    key_el = ET.Element('point')
                    key_el.set('stroke', key.get('stroke'))
                    key_el.set('fill', fill)
                    key_el.set('style', key.get('style', 'box'))
                    key_el.set('size', '5')
                    key_width = max(key_width, point_width)
                    key = key_el

            self.li_dict[li] = [key, label_el]
        self.key_width = key_width

    def place_legend(self, diagram):
        # We need to revisit the legend after the labels are created
        # and set their positions

        # Labels in tactile diagrams have their own properties
        if self.tactile:
            self.place_tactile_legend(diagram)
            return

        # We're doing this at the very end so the diagram.ctm is the default
        outer_padding = 5
        center_padding = 10
        interline = un.valid_eval(self.element.get('vertical-skip', '7'))
        height = outer_padding
        label_width = 0

        # Determine the width of the labels and the height of the entire legend
        for li in self.element:
            key, label = self.li_dict[li]
            label_dims = self.diagram.get_label_dims(label)
            height += label_dims[1] + interline
            label_width = max(label_width, label_dims[0])
        height += outer_padding - interline
            
        width = label_width + 2*outer_padding + self.key_width + center_padding

        # offset from the anchor
        offset = [8*(self.displacement[0] + 0.5),
                  8*(self.displacement[1]-0.5)]

        # set up a translate for the legend's bounding box
        p = self.def_anchor
        tform = CTM.translatestr(p[0] + offset[0], p[1] - offset[1])
        scale = un.valid_eval(self.element.get('scale', '1'))
        dx = scale * width * self.displacement[0]
        dy = scale * height * self.displacement[1]
        tform = tform + ' ' + CTM.translatestr(dx, -dy)

        tform = tform + ' ' + CTM.scalestr(scale, scale)
        self.group.set('transform', tform)

        # The following component is the bounding box of the legend
        rect = ET.Element('rect')
        self.group.append(rect)
        rect.set('x', '0')
        rect.set('y', '0')
        rect.set('width', str(width))
        rect.set('height', str(height))
        rect.set('stroke', self.element.get('stroke', 'black'))
        rect.set('fill', 'white')
        rect.set('fill-opacity', self.element.get('opacity', '1'))

        # Now we'll go through and place the labels and keys
        label_x = outer_padding + self.key_width + center_padding
        y = outer_padding
        for li in self.element:
            key, label = self.li_dict[li]
            label_dims = self.diagram.get_label_dims(label)
            label_group = self.diagram.get_label_group(label)[0]
            tform = CTM.translatestr(label_x, y)
            label_group.set('transform', tform)
            self.group.append(label_group)

            key_y = y + label_dims[1]/2

            if key.tag == 'point':
                key_x = outer_padding + self.key_width/2
                user_point = self.diagram.inverse_transform((key_x,key_y))
                key.set('p', util.pt2str(user_point, spacer=","))
                point.point(key, self.diagram, self.group, None)
            if key.tag == 'line':
                key_x0 = outer_padding
                key_x1 = outer_padding + self.line_width
                key.set('x1', str(key_x1))
                key.set('y1', str(key_y))
                key.set('x2', str(key_x0))
                key.set('y2', str(key_y))
                key.set('stroke-width', '2')
                self.group.append(key)

            y += label_dims[1] + interline

        
    def place_tactile_legend(self, diagram):
        # This is the same outline as above but works for tactile labels
        
        # The labels in a tactile diagram have already been placed
        # in the SVG so we first need to remove them
        root = self.diagram.get_root()
        groups = root.findall('g')
        label_groups = []
        for group in groups:
            id = group.get('id', 'none')
            if id == 'background-group':
                for rectangle in group:
                    if rectangle.get('id', 'none').startswith('legend-label'):
                        group.remove(rectangle)
            if id == 'braille-group':
                for label in group:
                    if label.get('id','none').startswith('legend-label'):
                        label_groups.append(label)
                        group.remove(label)

        # We're doing this at the very end so the diagram.ctm is the default
        
        gap = 3.6 # gap between embossed dots
        outer_padding = 3*gap # 1/8th of an inch
        center_padding = 6*gap

        # TODO:  This feature needs to be checked with users
        # It's not clear what this spacing should be
        interline = 4 * gap  # the vertical space between two items
        height = outer_padding
        label_width = 0

        # Find dimensions of the bounding box
        for li in self.element:
            key, label = self.li_dict[li]
            label_dims = self.diagram.get_label_dims(label)
            height += label_dims[1] + interline
            label_width = max(label_width, label_dims[0])
        height += outer_padding - interline
            
        width = label_width + 2*outer_padding + self.key_width + center_padding
        
        # Get the offset
        offset = [8*(self.displacement[0] + 0.5),
                  8*(self.displacement[1]-0.5)]
        
        offset = [o + 6*np.sign(o) for o in offset]
        if self.displacement[0] == 0:
            offset[0] += 6
        if self.displacement[1] == -1:
            offset[1] -= 6

        # Set up the translation for the bounding box    
        p = self.def_anchor
        dx = width * self.displacement[0]
        dy = height * self.displacement[1]

        translate = (p[0] + offset[0] + dx,
                     p[1] - offset[1] - dy)
        translate = [gap * round(c/gap) for c in translate]
        tform = CTM.translatestr(*translate)

        self.group.set('transform', tform)

        # Here is the bounding box
        rect = ET.Element('rect')
        self.group.append(rect)
        rect.set('x', '0')
        rect.set('y', '0')
        rect.set('width', str(width))
        rect.set('height', str(height))
        rect.set('stroke', self.element.get('stroke', 'black'))
        rect.set('fill', 'white')
            
        # Now place the labels and keys
        label_x = outer_padding + self.key_width + center_padding
        y = outer_padding
        for num, li in enumerate(self.element):
            key, label = self.li_dict[li]
            label_dims = self.diagram.get_label_dims(label)
            label_group = self.diagram.get_label_group(label)[0]
            label_x = gap * round(label_x/gap)
            y = gap * round(y/gap)
            tform = CTM.translatestr(label_x, y)
            label_group.set('transform', tform)
            label = label_groups[num]
            label_height = gap * round(label_dims[1]/gap)
            tform = CTM.translatestr(0, label_height)
            label.set('transform', tform)
            label_group.append(label_groups[num])
            self.group.append(label_group)

            key_y = y + label_dims[1]/2

            if key.tag == 'point':
                key_x = outer_padding + self.key_width/2
                user_point = self.diagram.inverse_transform((key_x,key_y))
                key.set('p', util.pt2str(user_point, spacer=","))
                point.point(key, self.diagram, self.group, None)
            if key.tag == 'line':
                key_x0 = outer_padding
                key_x1 = outer_padding + self.line_width
                key.set('x1', str(key_x1))
                key.set('y1', str(key_y))
                key.set('x2', str(key_x0))
                key.set('y2', str(key_y))
                key.set('stroke-width', '2')
                self.group.append(key)

            y += label_dims[1] + interline

        
def legend(element, diagram, parent, outline_status):
    
    # if we're finishing outline, we'll wait until
    # we process the legend at the end
    if outline_status == 'finish_outline':
        return

    Legend(element, diagram, parent, outline_status)

