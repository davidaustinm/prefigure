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
    
