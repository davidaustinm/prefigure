import numpy as np
from . import user_namespace as un
from . import label

import logging
logger = logging.getLogger('prefigure')

import warnings
with warnings.catch_warnings():
    warnings.filterwarnings('ignore', 'legacy print')
    np.set_printoptions(legacy="1.25")

diagram = None
def set_diagram(d):
    global diagram
    diagram = d

colors = {'gray': r'#777', 'lightgray': r'#ccc', 'darkgray': r'#333'}
textures = {'horizontal', 'vertical', 'diagonal',
            'backdiagonal', 'dot', 'diamond'}

# Some utilities to handle XML elements
def get_color(color):
    if color is None:
        return 'none'
    if isinstance(color, np.ndarray):
        return 'rgb({},{},{})'.format(color)
    return colors.get(color, color)

def add_attr(element, attr):
    for k, v in attr.items():
        element.set(k, str(v))

def get_attr(element, attr, default):
    try:
        attribute = un.valid_eval(element.get(attr, default))
        if isinstance(attribute, np.ndarray):
            return ','.join([float2longstr(a) for a in attribute])
        return str(attribute)
    except (TypeError, SyntaxError):  # this is a string that's not in the namespace
        return element.get(attr, default)
    
def set_attr(element, attr, default):
    value = get_attr(element, attr, default)
    value = label.evaluate_text(value)
    element.set(attr, value)

def get_1d_attr(element):
    d = {}
    if element.get('stroke') is not None:
        d['stroke'] = get_color(element.get('stroke'))
    if element.get('stroke-opacity') is not None:
        d['stroke-opacity'] = un.valid_eval(element.get('stroke-opacity'))
    if element.get('opacity') is not None:
        d['opacity'] = un.valid_eval(element.get('opacity'))
    if element.get('thickness') is not None:
        d['stroke-width'] = un.valid_eval(element.get('thickness'))
    if element.get('miterlimit') is not None:
        d['stroke-miterlimit'] = element.get('miterlimit')
    if element.get('linejoin') is not None:
        d['stroke-linejoin'] = element.get('linejoin')
    if element.get('linecap') is not None:
        d['stroke-linecap'] = element.get('linecap')
    if element.get('dash') is not None:
        d['stroke-dasharray'] = element.get('dash')
    d['fill'] = element.get('fill', 'none')
    return d

def set_tactile_fill(element):
    fill = element.get('fill', 'none')
    if fill.startswith('url'):
        return
    if fill in {'white', 'none'}:
        element.set('fill', fill)
    else:
        element.set('fill', 'lightgray')

def get_2d_attr(element):
    d = get_1d_attr(element)
    fill_color = get_color(element.get('fill'))
    texture = element.get('fill-pattern', None)
    if texture is not None:
        if texture in textures:
            if fill_color is None:
                fill_color = 'gray'
            url = diagram.add_texture(texture, fill_color)
            d['fill'] = fr'url(#{url})'
            element.set('fill', d['fill'])
        else:
            logger.error(f"{texture} is not a recognized texture")
    else:
        d['fill'] = get_color(element.get('fill'))
        element.set('fill', d['fill'])
    if element.get('fill-rule') is not None:
        d['fill-rule'] = element.get('fill-rule')
    if element.get('fill-opacity') is not None:
        d['fill-opacity'] = un.valid_eval(element.get('fill-opacity'))
    return d

def cliptobbox(g_element, element, diagram):
    if element.get('cliptobbox', 'no') == 'no':
        return
    id = diagram.get_clippath()
    g_element.set('clip-path', r'url(#{})'.format(id))

def float2str(x):
    return "%.1f" % x

def float2longstr(x):
    return "%.4f" % x

def pt2str(p, spacer = ' ', paren=False):
    text = spacer.join(["%.1f" % c for c in p])
    if paren:
        return '('+text+')'
    return text

def pt2long_str(p, spacer = ' '):
    return spacer.join(["%.4f" % c for c in p])

def np2str(p):
    return pt2str(p, spacer=',', paren=True)
