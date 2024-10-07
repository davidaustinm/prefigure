from . import user_namespace as un
import numpy as np

colors = {'gray': r'#777', 'lightgray': r'#ccc', 'darkgray': r'#333'}

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
            return np.array2string(attribute, separator=',')
        return str(attribute)
    except (TypeError, SyntaxError):  # this is a string that's not in the namespace
        return element.get(attr, default)
    
def set_attr(element, attr, default):
    element.set(attr, get_attr(element, attr, default))

def get_1d_attr(element):
    d = {}
    if element.get('stroke') is not None:
        d['stroke'] = get_color(element.get('stroke'))
    if element.get('stroke-opacity') is not None:
        d['stroke-opacity'] = element.get('stroke-opacity')
    if element.get('opacity') is not None:
        d['opacity'] = element.get('opacity')
    if element.get('thickness') is not None:
        d['stroke-width'] = element.get('thickness')
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

def get_2d_attr(element):
    d = get_1d_attr(element)
    d['fill'] = get_color(element.get('fill'))
    if element.get('fill-rule') is not None:
        d['fill-rule'] = element.get('fill-rule')
    if element.get('fill-opacity') is not None:
        d['fill-opacity'] = element.get('fill-opacity')
    return d

def cliptobbox(g_element, element, diagram):
    if element.get('cliptobbox', 'no') == 'no':
        return
    id = diagram.get_clippath()
    g_element.set('clip-path', r'url(#{})'.format(id))

def float2str(x):
    return "%.1f" % x

def pt2str(p, spacer = ' ', paren=False):
    text = spacer.join(["%.1f" % c for c in p])
    if paren:
        return '('+text+')'
    return text

def pt2long_str(p, spacer = ' '):
    return spacer.join(["%.4f" % c for c in p])

def np2str(p):
    return pt2str(p, spacer=',', paren=True)
