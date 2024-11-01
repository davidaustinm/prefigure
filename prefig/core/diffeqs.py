import numpy as np
import lxml.etree as ET
import sys
import scipy.integrate
from . import user_namespace as un
from . import utilities as util
from . import arrow

def de_solve(element, diagram, parent, outline_status):
    if outline_status == 'finish_outline':
        return

    f = un.valid_eval(element.get('function'))

    t0 = un.valid_eval(element.get('t0'))
    y0 = un.valid_eval(element.get('y0'))
    if not isinstance(y0, np.ndarray):
        y0 = np.array([y0])

    t1 = diagram.bbox()[2]
    t1 = un.valid_eval(element.get('t1', str(t1)))

    N = un.valid_eval(element.get('N', '100'))
    t = np.linspace(t0, t1, N)

    method = element.get('method', 'RK45')

    if element.get('max-step', None) is not None:
        max_step = un.valid_eval(element.get('max-step'))

        solution = scipy.integrate.solve_ivp(f,
                                             (t0, t1),
                                             y0,
                                             t_eval=t,
                                             max_step=max_step,
                                             method=method)
    else:
        solution = scipy.integrate.solve_ivp(f,
                                             (t0, t1),
                                             y0,
                                             t_eval=t,
                                             method=method)
        
    solution = np.stack((solution.t, *solution.y))

    try:
        name = element.get('name')
    except KeyError:
        print('The solution to a differential equation needs a name.  In', 
              element.get('id', '[element not named]'))
        sys.exit()
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
        solution = un.valid_eval(element.get('solution'))

    # The author can specify which quantities to plot through the axes attribute
    # We'll just treat this as a string and break out the quantities on
    # the x and y axes.  By default, the axes are t and y, which would be apppropriate
    # for a single ODE.  For a system, phase portraits can be constructed using
    # axes='(y0, y1)', for instance, as the axes
    axes = element.get('axes', '(t,y)'). strip()[1: -1].split(',')
    x_axis, y_axis = [a.strip() for a in axes]

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
