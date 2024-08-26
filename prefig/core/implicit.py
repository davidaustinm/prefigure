## Add a graphical element for implicit curves

import lxml.etree as ET
from . import user_namespace as un
from . import utilities as util
from .import math_utilities as m_util

# add an implicit curve to a diagram
def implicit_curve(element, diagram, parent, outline_status):
    ImplicitCurve(element, diagram, parent, outline_status)

class QuadTree():
    def __init__(self, b, d):
        self.corners = b
        self.depth = d
    def subdivide(self):
        bottom = m_util.midpoint(self.corners[0], self.corners[1])
        left = m_util.midpoint(self.corners[0], self.corners[3])
        right = m_util.midpoint(self.corners[1], self.corners[2])
        top = m_util.midpoint(self.corners[2], self.corners[3])
        mid = m_util.midpoint(bottom, top)
        return [QuadTree([self.corners[0], bottom, mid, left], self.depth-1),
                QuadTree([bottom, self.corners[1], right, mid], self.depth-1),
                QuadTree([left, mid, top, self.corners[3]], self.depth-1),
                QuadTree([mid, right, self.corners[2], top], self.depth-1)
                ]
    def intersects(self, g):
        sign = g.value(self.corners[3])
        for i in range(4):
            nextsign = g.value(self.corners[i])
            if sign * nextsign <= 0:
                return True
            sign = nextsign
        return False

    def findzero(self, p1, p2, g):
        dx = p2[0]-p1[0]
        dy = p2[1]-p1[1]
        change = 0.00001
        if dx != 0:
            dx = change*abs(dx)/dx
            dy = 0
            dt = dx
        else:
            dy = change*abs(dy)/dy
            dx = 0
            dt = dy
        p = p1
        diff = 1
        N = 0
        while abs(diff) > 0.000001 and N < 50:
            f = g.value(p)
            if f == 0:
                break
            df = (g.value([p[0] + dx, p[1] + dy]) - f)/dt
            diff = f/float(df)
            if dx != 0:
                nextp = [p[0] - diff, p[1]]
            else:
                nextp = [p[0], p[1] - diff]
            N += 1
            p = nextp
        return p

    def segments(self, g):
        corner = self.corners[3]
        sign = g.value(corner)
        segments = []
        lastZero = None
        for i in range(4):
            nextcorner = self.corners[i]
            nextsign = g.value(nextcorner)
            if sign == 0 and nextsign == 0:
                segments.append([corner, nextcorner])
            elif sign * nextsign <= 0:
                if lastZero is None:
                    lastZero = self.findzero(corner, nextcorner, g)
                else:
                    thisZero = self.findzero(corner, nextcorner, g)
                    segments.append([lastZero, thisZero])
                    lastZero = thisZero
            corner = nextcorner
            sign = nextsign
        return segments

class LevelSet():                
    def __init__(self, f, k):
        self.f = f
        self.k = k
    def value(self, p):
        return self.f(p[0], p[1]) - self.k

class ImplicitCurve():
    def __init__(self, element, diagram, parent, outline_status):
        if outline_status == "finish_outline":
            finish_outline(element, diagram, parent)
            return

        if diagram.output_format() == 'tactile':
            element.set('stroke', 'black')
        else:
            util.set_attr(element, 'stroke', 'black')
        util.set_attr(element, 'thickness', '2')

        self.bbox = diagram.bbox()
        f = un.valid_eval(element.get('function'))
        k = un.valid_eval(element.get('k', '0'))
        self.depth = int(un.valid_eval(element.get('depth', '8')))
        self.initialdepth = int(un.valid_eval(element.get('initial-depth','4')))
        self.levelset = LevelSet(f, k)
        self.k = k

        segments = self.getpoints()
        cmds = []
        for s in segments:
            s0 = diagram.transform(s[0][:2])
            s1 = diagram.transform(s[1][:2])
            cmds.append('M ' + util.pt2str(s0))
            cmds.append('L ' + util.pt2str(s1))
        d = ' '.join(cmds)

        path = ET.Element('path')
        diagram.add_id(path, element.get('id'))
        path.set('d', d)

        util.add_attr(path, util.get_1d_attr(element))

        if outline_status == 'add_outline':
            diagram.add_outline(element, path, parent)
            return

        if element.get('outline', 'no') == 'yes' or diagram.output_format() == 'tactile':
            diagram.add_outline(element, path, parent)
            finish_outline(element, diagram, parent)
        else:
            parent.append(path)

    def getpoints(self):
        root = QuadTree([ [self.bbox[0], self.bbox[1]],
                          [self.bbox[2], self.bbox[1]],
                          [self.bbox[2], self.bbox[3]],
                          [self.bbox[0], self.bbox[3]]
                          ], self.depth)
        tree = [root]
        for i in range(self.initialdepth):
            newtree = []
            for node in tree:
                newtree = newtree + node.subdivide()
            tree = newtree
        points = []
        while len(tree) > 0:
            node = tree.pop(0)
            if node.depth == 0:
                segments = node.segments(self.levelset)
                for s in segments:
                    points.append([[s[0][0], s[0][1], self.k],
                                   [s[1][0], s[1][1], self.k]])

            elif node.intersects(self.levelset):
                tree = tree + node.subdivide()

        return points

def finish_outline(element, diagram, parent):
    diagram.finish_outline(element,
                           element.get('stroke'),
                           element.get('thickness'),
                           element.get('fill', 'none'),
                           parent)
