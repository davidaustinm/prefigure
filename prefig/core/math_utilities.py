import numpy as np
import math
from . import user_namespace as un
from . import calculus

import logging
logger = logging.getLogger('prefigure')

diagram = None

def set_diagram(d):
    global diagram
    diagram = d

# introduce some useful mathematical operations
#   that are meant to be available to authors

def ln(x):
    return math.log(x)

def dot(u, v):
    return np.dot(np.array(u), np.array(v))

def distance(p, q):
    return length(np.array(p) - np.array(q))

def length(u):
    return np.linalg.norm(np.array(u))

def normalize(u):
    return 1/length(u) * np.array(u)

def midpoint(u, v):
    return 0.5*(np.array(u) + np.array(v))

def angle(p, units = 'deg'):
    angle = math.atan2(p[1], p[0])
    if units == 'deg':
        return math.degrees(angle)
    return angle

def roll(array):
    array = np.array(array)
    return np.roll(array, 1, axis=0)

def choose(n,k):
    n = int(n)
    k = int(k)
    return math.comb(n, k)

def append(input, item):
    l = list(input)
    l.append(item)
    if isinstance(input, np.ndarray):
        l = np.array(l)
    return l

def chi_oo(a, b, t):
    if t > a and t < b:
        return 1
    return 0

def chi_oc(a, b, t):
    if t > a and t <= b:
        return 1
    return 0

def chi_co(a, b, t):
    if t >= a and t < b:
        return 1
    return 0

def chi_cc(a, b, t):
    if t >= a and t <= b:
        return 1
    return 0

def rotate(v, theta):
    c = math.cos(theta)
    s = math.sin(theta)
    return np.array([c*v[0]-s*v[1], s*v[0]+c*v[1]])

def deriv(f, a):
    return calculus.derivative(f, a)

def grad(f, a):
    a = list(a)
    grad = []
    for j in range(len(a)):
        def f_trace(x):
            b = a[:]
            b[j] = x
            return f(*b)
        grad.append(calculus.derivative(f_trace, a[j]))
    return np.array(grad)

def zip_lists(a, b):
    return list(zip(a, b))

def filter(df, a, b, value):
    mask = df[b] == value
    return df[a][mask]

def evaluate_bezier(controls, t):
    dim = len(controls[0])
    N = len(controls)
    controls = np.array(controls)
    sum = np.array([0.0] * dim)
    if N == 3:
        coefficients = [1,2,1]
    else:
        coefficients = [1,3,3,1]
    for j in range(N):
        sum += coefficients[j] * (1-t)**(N-j-1) * t**j * controls[j]
    return sum

def eulers_method(f, t0, y0, t1, N):
    h = (t1 - t0)/N
    if isinstance(y0, np.ndarray):
        points = [[t0, *y0]]
    else:
        points = [[t0, y0]]
    t = t0
    y = y0
    for _ in range(N):
        y += f(t, y) * h
        t += h
        if isinstance(y, np.ndarray):
            points.append([t, *y])
        else:
            points.append([t, y])
    return np.array(points)

# dirac delta function to be used in solving ODEs
def delta(t, a):
    breaks = un.retrieve('__breaks')
    if breaks is not None:
        breaks.append(a)
        return 0
    delta_on = un.retrieve('__delta_on')
    if np.isclose(t, a) and delta_on:
        return 1

    return 0

def line_intersection(lines):
    p1, p2 = [np.array(c) for c in lines[0]]
    q1, q2 = [np.array(c) for c in lines[1]]

    diff = p2-p1
    normal = [-diff[1], diff[0]]

    v = q2 - q1
    denom = dot(normal, v)
    if abs(denom) < 1e-10:
        bbox = diagram.bbox()
        return np.array([(bbox[0]+bbox[2])/2, (bbox[1]+bbox[3])/2])
    t = dot(normal, q1-p1) / denom
    return q1 - t*v

# find the intersection of two graphs or the zero of just one
def intersect(functions, seed=None, interval=None):
    bbox = diagram.bbox()
    width = bbox[2] - bbox[0]
    height = bbox[3] - bbox[1]
    tolerance = 1e-06 * height

    upper = bbox[3] + height
    lower = bbox[1] - height

    # we want to allow for some flexibility. We can find the intersection
    # of
    # -- two graphs f(x), g(x):  intersect((f,g), seed)
    # -- zero of f(x):           intersect(f, seed)
    # -- solve f(x) = y:         intersect((f, y), seed)
    # -- two lines:              intersect((p1,p2),(q1,q2),seed)
    if isinstance(functions, np.ndarray):
        if isinstance(functions[0], np.ndarray):
            return line_intersection(functions)
        try:
            y_value = float(functions[1])
            f = lambda x: functions[0](x) - y_value
        except:
            f = lambda x: functions[0](x) - functions[1](x)
    else:
        f = functions

    if interval is None:
        interval = (bbox[0], bbox[2])

    x0 = seed
    y0 = f(x0)

    if abs(y0) < tolerance:
        return x0

    dx = 0.002 * width
    x = x0
    x_left = -np.inf
    while x >= interval[0]:
        x -= dx
        try:
            y = f(x)
        except:
            break
        if y > upper or y < lower:
            break
        if abs(y) < tolerance:
            x_left = x
            break
        if y * y0 < 0:
            x_left = x
            break
    if x_left != -np.inf and abs(f(x_left) - f(x_left + dx)) > height:
        x_left = -np.inf

    x = x0
    x_right = np.inf
    while x <= interval[1]:
        x += dx
        try:
            y = f(x)
        except:
            break
        if y > upper or y < lower:
            break
        if abs(y) < tolerance:
            x_right = x
            break
        if y * y0 < 0:
            x_right = x
            break

    if x_right != np.inf and abs(f(x_right) - f(x_right - dx)) > height:
        x_right = np.inf

    if x_left < interval[0] and x_right > interval[1]:
        # we didn't find anything
        return x0

    if x_left < interval[0]:
        x2 = x_right
        x1 = x_right - dx

    if x_right > interval[1]:
        x2 = x_left + dx
        x1 = x_left

    if abs(x0 - x_right) < abs(x0 - x_left):
        x1 = x_right - dx
        x2 = x_right
    else:
        x1 = x_left
        x2 = x_left + dx

    for _ in range(8):
        mid = (x1 + x2) / 2
        if f(mid) * f(x1) < 0:
            x2 = mid
        else:
            x1 = mid
    return (x1+x2)/2

