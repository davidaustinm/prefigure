import numpy as np
import logging
import copy
import math
from . import math_utilities as math_util
from . import utilities as util
from . import user_namespace as un

log = logging.getLogger('prefigure')

# current transformation matrix:
#   models a 2d affine coordinate transform using homogeneous coordinates.
#     x' = ax + by + c
#     y' = dx + ey + f
#   transform will be the 2x3 matrix:
#     [ [a, b, c],
#       [d, e, f] ]
#   2d points (x, y) will be represented as [x, y, 1]
#   
#   Could be improved by using numpy operations

def identity():
    return [[1,0,0],[0,1,0]]

def translation(x,y):
    return [[1,0,x],[0,1,y]]

def scaling(sx, sy):
    return [[sx,0,0],[0,sy,0]]

def rotation(theta, units="deg"):
    if units == "deg":
        theta *= math.pi/180
    c = math.cos(theta)
    s = math.sin(theta)
    return [[c,-s,0],[s,c,0]]

def matrix2str(m):
    return 'matrix(' + ','.join([str(c) for c in m[0]+m[1]]) + ')'

def concat(m, n):
    c = [[n[0][0], n[1][0], 0],
         [n[0][1], n[1][1], 0],
         [n[0][2], n[1][2], 1]]
    return [[math_util.dot(m[0], c[0]), math_util.dot(m[0], c[1]), math_util.dot(m[0], c[2])],
            [math_util.dot(m[1], c[0]), math_util.dot(m[1], c[1]), math_util.dot(m[1], c[2])]]

def translatestr(x, y):
    return 'translate('+util.pt2str((x, y), spacer=',')+')'

def scalestr(x, y):
    return 'scale('+str(x)+','+str(y)+')'

def rotatestr(theta):
    return 'rotate('+util.float2str(-theta)+')'

class CTM:
    def __init__(self, ctm = None):
        if ctm is None:
            self.ctm = identity()
            self.inverse = identity()
        else:
            self.ctm = ctm
        self.ctm_stack = []

    def push(self):
        self.ctm_stack.append([self.ctm, self.inverse])

    def pop(self):
        if len(self.ctm_stack) == 0:
            log.error("Attempt to restore an empty transform")
            return
        self.ctm, self.inverse = self.ctm_stack.pop(-1)

    def translate(self, x, y):
        m = translation(x, y)
        self.ctm = concat(self.ctm, m)
        minv = translation(-x, -y)
        self.inverse = concat(minv, self.inverse)

    def scale(self, x, y):
        s = scaling(x, y)
        self.ctm = concat(self.ctm, s)
        sinv = scaling(1/x, 1/y)
        self.inverse = concat(sinv, self.inverse)

    def rotate(self, theta, units="deg"):
        m = rotation(theta, units)
        self.ctm = concat(self.ctm, m)
        minv = rotation(-theta, units)
        self.inverse = concat(minv, self.inverse)

    def inverse_transform(self, p):
        p = list(p).copy()
        p.append(1)
        return np.array([math_util.dot(self.inverse[i], p) for i in range(2)])

    def transform(self, p):
        p = list(p).copy()
        p.append(1)
        return np.array([math_util.dot(self.ctm[i], p) for i in range(2)])

    def copy(self):
        return copy.deepcopy(self) # CTM(copy.deepcopy(self.ctm))

def transform_group(element, diagram, root, outline_status):
    if outline_status != "finish_outline":
        diagram.ctm().push()
        element.tag = "group"

    diagram.parse(element, root=root, outline_status=outline_status)

    if outline_status != "finish_outline":
        diagram.ctm().pop()

def transform_translate(element, diagram, root, outline_status):
    if outline_status == "finish_outline":
        return
    try:
        p = un.valid_eval(element.get("by"))
    except:
        log.error(f"Error in <translate> parsing by={element.get('by')}")
        return
    diagram.ctm().translate(*p)

def transform_rotate(element, diagram, root, outline_status):
    if outline_status == "finish_outline":
        return
    try:
        angle = un.valid_eval(element.get("by"))
    except:
        log.error(f"Error in <rotate> parsing by={element.get('by')}")
        return
    try:
        p = un.valid_eval(element.get("about", "(0,0)"))
    except:
        log.error(f"Error in <rotate> parsing about={element.get('about')}")
        return

    if element.get("degrees", "yes") == "yes":
        units = "deg"
    else:
        units = "rad"

    ctm = diagram.ctm()
    ctm.translate(*p)
    ctm.rotate(angle, units=units)
    ctm.translate(*(-p))

def transform_scale(element, diagram, root, outline_status):
    if outline_status == "finish_outline":
        return
    try:
        s = un.valid_eval(element.get("by"))
    except:
        log.error(f"Error in <scale> parsing by={element.get('by')}")
        return

    ctm = diagram.ctm()
    if isinstance(s, np.ndarray):
        ctm.scale(*s)
    else:
        ctm.scale(s, s)
