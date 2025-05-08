import numpy as np
import lxml.etree as ET
import sys
import logging
import scipy.integrate
from . import user_namespace as un
from . import utilities as util
from . import arrow

log = logging.getLogger('prefigure')

def de_solve(element, diagram, parent, outline_status):
    if outline_status == 'finish_outline':
        return

    try:
        f = un.valid_eval(element.get('function'))
    except:
        log.error(f"Error in ODE solver:  cannot retrieve function={element.get('function')}")
        return

    try:
        t0 = un.valid_eval(element.get('t0'))
    except:
        log.error(f"Error in ODE solver:  cannot retrieve t0={element.get('t0')}")
        return
    try:
        y0 = un.valid_eval(element.get('y0'))
    except:
        log.error(f"Error in ODE solver:  cannot retrieve y0={element.get('y0')}")
        return

    if not isinstance(y0, np.ndarray):
        y0 = np.array([y0])

    t1 = diagram.bbox()[2]
    t1 = un.valid_eval(element.get('t1', str(t1)))
    N = un.valid_eval(element.get('N', '100'))
    method = element.get('method', 'RK45')

    max_step = None
    if element.get('max-step', None) is not None:
        max_step = un.valid_eval(element.get('max-step'))

    # in case f contains delta functions, we will determine where those occur
    breaks = un.find_breaks(f, t0, y0)
    if len(breaks) > 0:
        _breaks = []
        for b in breaks:
            if b >= t0 and b < t1:
                _breaks.append(b)
        breaks = _breaks
    breaks.sort()
    breaks.append(t1)

    solution_t = None
    solution_y = None

    if len(breaks) > 0:
        if np.isclose(t0, breaks[0]):
            y0 = y0 + un.measure_de_jump(f, t0, y0)
            breaks.pop(0)

    while len(breaks) > 0:
        next_t = breaks.pop(0)
        t = np.linspace(t0, next_t, N)

        if max_step is not None:
            solution = scipy.integrate.solve_ivp(f,
                                                 (t0, next_t),
                                                 y0,
                                                 t_eval=t,
                                                 max_step=max_step,
                                                 method=method)
        else:
            solution = scipy.integrate.solve_ivp(f,
                                                 (t0, next_t),
                                                 y0,
                                                 t_eval=t,
                                                 method=method)
        t0 = next_t
        y0 = solution.y.T[-1]
        y0 = y0 + un.measure_de_jump(f, t0, y0)
        if solution_t is None:
            solution_t = solution.t
            solution_y = solution.y
        else:
            solution_t = np.hstack((solution_t, solution.t))
            solution_y = np.hstack((solution_y, solution.y))
        
    solution = np.stack((solution_t, *solution_y))

    name = element.get('name', None)
    if name is None:
        log.error(f"Error in ODE solver setting name={element.get('name')}")
        return
    un.enter_namespace(name, solution)

def plot_de_solution(element, diagram, parent, outline_status):
    if outline_status == 'finish_outline':
        finish_outline(element, diagram, parent)
        return
    if element.get('function') is not None:
        element.set('name', '__de_solution')
        de_solve(element, diagram, parent, None)
        solution = un.valid_eval('__de_solution')
    else:
        try:
            solution = un.valid_eval(element.get('solution'))
        except:
            log.error(f"Error in <plot-de-solution> finding solution={element.get('solution')}")
            return

    # The author can specify which quantities to plot through the axes attribute
    # We'll just treat this as a string and break out the quantities on
    # the x and y axes.  By default, the axes are t and y, which would be apppropriate
    # for a single ODE.  For a system, phase portraits can be constructed using
    # axes='(y0, y1)', for instance, as the axes

    try:
        axes = element.get('axes', '(t,y)'). strip()[1: -1].split(',')
        x_axis, y_axis = [a.strip() for a in axes]
    except:
        log.error(f"Error in <plot-de-solution> setting axes={element.get('axes')}")
        return

    if x_axis.startswith('y'):
        axis0 = solution[int(x_axis[1:])+1]
    else:
        axis0 = solution[0]

    if y_axis == 'y':
        axis1 = solution[1]
    else:
        axis1 = solution[int(y_axis[1:])+1]

    curve = zip(axis0, axis1)
    p = diagram.transform(next(curve))
    cmds = ['M ' + util.pt2str(p)]
    while True:
        try:
            p = diagram.transform(next(curve))
            cmds.append('L ' + util.pt2str(p))
        except StopIteration:
            break
    d = ' '.join(cmds)

    if diagram.output_format() == 'tactile':
        element.set('stroke', 'black')
    else:
        util.set_attr(element, 'stroke', 'blue')
        util.set_attr(element, 'fill', 'none')
    util.set_attr(element, 'thickness', '2')

    path = ET.Element('path')
    diagram.add_id(path, element.get('id'))
    path.set('d', d)
    util.add_attr(path, util.get_2d_attr(element))
#    path.set('type', 'parametric curve')

    if element.get('arrow', 'no') == 'yes':
        arrow.add_arrowhead_to_path(diagram, 'marker-end', path)

    element.set('cliptobbox', element.get('cliptobbox', 'yes'))
    util.cliptobbox(path, element, diagram)
    
    if outline_status == 'add_outline':
        diagram.add_outline(element, path, parent)
        return

    if element.get('outline', 'no') == 'yes' or diagram.output_format() == 'tactile':
        diagram.add_outline(element, path, parent)
        finish_outline(element, diagram, parent)
    else:
        parent.append(path)

def finish_outline(element, diagram, parent):
    diagram.finish_outline(element,
                           element.get('stroke'),
                           element.get('thickness'),
                           element.get('fill', 'none'),
                           parent)
