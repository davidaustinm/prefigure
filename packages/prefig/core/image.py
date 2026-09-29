import lxml.etree as ET
import logging
import numpy as np
import base64
import lxml.etree as ET
from . import user_namespace as un
from . import group
from . import utilities as util
from . import CTM

log = logging.getLogger('prefigure')

type_dict = {'jpg':'jpeg',
             'jpeg':'jpeg',
             'png':'png',
             'gif':'gif',
             'svg':'svg'}

def image(element, diagram, parent, outline_group):
    if len(element) == 0:
        log.error("An <image> must contain content to replace the image in a tactile build")
        return;

    if diagram.output_format() == 'tactile':
        element.tag = 'group'
        group.group(element, diagram, parent, outline_group)
        return

    source = element.get('source', None)
    if source is None:
        log.error("An <image> needs a @source attribute")
        return
    try:
        ll = un.valid_eval(element.get('lower-left', '(0,0)'))
        dims = un.valid_eval(element.get('dimensions', '(1,1)'))
        center = element.get('center', None)
        if center is not None:
            center = un.valid_eval(center)
            ll = center - 0.5 * dims
        else:
            center = ll + 0.5*dims
        rotation = un.valid_eval(element.get('rotate', '0'))
        scale = un.valid_eval(element.get('scale', '1'))
    except:
        log.error("Error parsing placement data in an <image>")
        return

    file_type = None
    if element.get('filetype', None) is not None:
        file_type = type_dict.get(element.get('filetype'), None)
    if file_type is None:
        suffix = source.split('.')[-1]
        file_type = type_dict.get(suffix, None)
    if file_type is None:
        log.error(f"Cannot determine the type of image in {source}")
        return

    ll_svg = diagram.transform(ll)
    ur_svg = diagram.transform(ll + dims)
    center_svg = diagram.transform(center)
    width, height = ur_svg - ll_svg
    height = -height

    if diagram.get_environment() == 'pretext':
        source = 'data/' + source
    else:
        assets_dir = diagram.get_external()
        if assets_dir is not None:
            assets_dir = assets_dir.strip()
            if assets_dir[-1] != '/':
                assets_dir += '/'
            source = assets_dir + source

    opacity = element.get('opacity', None)
    if opacity is not None:
        opacity = un.valid_eval(opacity)
    if file_type == 'svg':
        svg_tree = ET.parse(source)
        svg_root = svg_tree.getroot()
        svg_width = svg_root.get('width', None)
        svg_height = svg_root.get('height', None)

        object = ET.SubElement(parent, 'foreignObject')
        object.set('x', util.float2str(center_svg[0]-width/2))
        object.set('y', util.float2str(center_svg[1]-height/2))
        object.set('width', util.float2str(width))
        object.set('height', util.float2str(height))
        svg_root.set('width', '100%')
        svg_root.set('height', '100%')
        if svg_root.get('viewBox', None) is None:
            svg_root.set('viewBox', f"0 0 {svg_width} {svg_height}")
        object.append(svg_root)
        diagram.add_id(object, element.get('id'))
        svg_root.attrib.pop('id', None)
        if opacity is not None:
            object.set('opacity', util.float2str(opacity))
        return

    image_el = ET.SubElement(parent, 'image')
    diagram.register_svg_element(element, image_el)
    image_el.set('x', util.float2str(-width/2))
    image_el.set('y', util.float2str(-height/2))
    image_el.set('width', util.float2str(width))
    image_el.set('height', util.float2str(height))
    if opacity is not None:
        image_el.set('opacity', util.float2str(opacity))
    diagram.add_id(image_el, element.get('id'))

    with open(source, "rb") as image_file:
        encoded_string = base64.b64encode(image_file.read())
        encoded_string = encoded_string.decode('utf-8')
    ref = f"data:image/{file_type};base64,{encoded_string}"
    image_el.set('href', ref)

    transform_pieces = [CTM.translatestr(*center_svg)]

    if isinstance(scale, np.ndarray):
        transform_pieces.append(CTM.scalestr(*scale))
    elif scale != '1':
        transform_pieces.append(f"scale({scale})")
            
    if rotation != 0:
        transform_pieces.append(f"rotate({-rotation})")

    image_el.set('transform', ' '.join(transform_pieces))
    
def qr_code(element, diagram, parent, outline_group):
    text = element.text
    if text is None:
        log.error('A <qr-code> element needs text')
        return

    import segno
    import io
    qr = segno.make(text, error='H')
    buffer = io.BytesIO()
    qr.save(buffer, kind='svg', scale=10)
    svg_root = ET.fromstring(buffer.getvalue())

    try:
        ll = un.valid_eval(element.get('lower-left', '(0,0)'))
        dims = un.valid_eval(element.get('dimensions', '(1,1)'))
        center = element.get('center', None)
        if center is not None:
            center = un.valid_eval(center)
            ll = center - 0.5 * dims
        else:
            center = ll + 0.5 * dims
    except:
        log.error("Error parsing placement data in a <qr-code>")
        return

    if diagram.output_format() == 'tactile':
        element.tag = 'group'
        if len(element) == 1:
            if element[0].tag == 'label':
                if element[0].get('anchor', None) is None:
                    element[0].set('anchor', f"({center[0]},{center[1]})")
        group.group(element, diagram, parent, outline_group)
        return

    ll_svg = diagram.transform(ll)
    ur_svg = diagram.transform(ll + dims)
    width = ur_svg[0] - ll_svg[0]
    height = ll_svg[1] - ur_svg[1]

    viewBox = svg_root.get('viewBox', None)
    if viewBox:
        vb_parts = viewBox.split()
        vb_w, vb_h = float(vb_parts[2]), float(vb_parts[3])
    else:
        vb_w = float(svg_root.get('width', '100').rstrip('pt'))
        vb_h = float(svg_root.get('height', '100').rstrip('pt'))

    sx = width / vb_w
    sy = height / vb_h

    transform_pieces = [
        f"translate({util.float2str(ll_svg[0])},{util.float2str(ur_svg[1])})",
        f"scale({util.float2str(sx)},{util.float2str(sy)})",
    ]

    g_el = ET.SubElement(parent, 'g')
    g_el.set('transform', ' '.join(transform_pieces))
    diagram.add_id(g_el, element.get('id'))

    for child in svg_root:
        local = ET.QName(child.tag).localname
        new_el = ET.SubElement(g_el, local)
        new_el.attrib.update(child.attrib)
        new_el.set('fill', 'none')
        if child.text:
            new_el.text = child.text
