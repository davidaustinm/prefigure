import numpy as np
from . import math_utilities as math_util
from . import utilities as util
import copy
import math

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

    def concat(self, m):
        return CTM(concat(self.ctm, m))

    def copy(self):
        return copy.deepcopy(self) # CTM(copy.deepcopy(self.ctm))
