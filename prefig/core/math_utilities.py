import numpy as np
import scipy
import scipy.special
import math

# introduce some useful mathematical operations
#   that are meant to be available to authors

def dot(u, v):
    return np.dot(np.array(u), np.array(v))

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
    return int(scipy.special.binom(n,k))

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

def eulers_method(f, t0, y0, t1, N):
    h = (t1 - t0)/N
    if isinstance(y0, np.ndarray):
        points = [[t0, *y0]]
    else:
        points = [[t0, y0]]
    t = t0
    y = y0
    for _ in range(N):
        t += h
        y += f(t, y) * h
        if isinstance(y, np.ndarray):
            points.append([t, *y])
        else:
            points.append([t, y])
    return np.array(points)

