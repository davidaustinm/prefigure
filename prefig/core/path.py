## Add a graphical element for an SVG path

import lxml.etree as ET
import logging
import math
from . import user_namespace as un
from . import utilities as util
from . import math_utilities as math_util
from . import arrow
from . import CTM
from . import tags

log = logging.getLogger('prefigure')

# These are the tags that can appear within a path
path_tags = {'moveto',
             'rmoveto',
             'lineto',
             'rlineto',
             'horizontal',
             'vertical',
             'cubic-bezier',
             'quadratic-bezier',
             'smooth-cubic',
             'smooth-quadratic'}

def is_path_tag(tag):
    return tag in path_tags
             

# Process a path tag into a graphical component
def path(element, diagram, parent, outline_status):
    if outline_status == 'finish_outline':
        finish_outline(element, diagram, parent)
        return

    if diagram.output_format() == 'tactile':
        if element.get('stroke') is not None:
            element.set('stroke', 'black')
        if element.get('fill') is not None:
            element.set('fill', 'lightgray')
    util.set_attr(element, 'stroke', 'none')
    util.set_attr(element, 'fill', 'none')
    util.set_attr(element, 'thickness', '2')

    cmds = ["M"]
    start = element.get('start', None)
    if start is None:
        log.error("A <path> element needs a @start attribute")
        return
    try:
        user_start = un.valid_eval(start)
    except:
        log.error(f"Error in <path> defining start={element.get('start')}")
        return
    current_point = user_start
    start = diagram.transform(user_start)
    cmds.append(util.pt2str(start))

    try:
        for child in element:
            log.debug(f"Processing {child.tag} inside <path>")
            cmds, current_point = process_tag(child,
                                              diagram,
                                              cmds,
                                              current_point)
    except:
        log.error("Error in <path> processing subelements")
        return

    if element.get('closed', 'no') == 'yes':
        cmds.append('Z')
    d = ' '.join(cmds)
    path = ET.Element('path')
    diagram.add_id(path, element.get('id'))
    path.set('d', d)
    util.add_attr(path, util.get_2d_attr(element))
#    path.set('type', 'path')
    element.set('cliptobbox', element.get('cliptobbox', 'yes'))
    util.cliptobbox(path, element, diagram)

    arrows = int(element.get('arrows', '0'))
    forward = 'marker-end'
    backward = 'marker-start'
    if element.get('reverse', 'no') == 'yes':
        forward, backward = backward, forward
    if arrows > 0:
        arrow.add_arrowhead_to_path(
            diagram,
            forward,
            path,
            arrow_width=element.get('arrow-width', None),
            arrow_angles=element.get('arrow-angles', None)
        )
    if arrows > 1:
        arrow.add_arrowhead_to_path(
            diagram,
            backward,
            path,
            arrow_width=element.get('arrow-width', None),
            arrow_angles=element.get('arrow-angles', None)
        )

    if element.get('mid-arrow', 'no') == 'yes':
        arrow.add_arrowhead_to_path(
            diagram,
            'marker-mid',
            path
        )

    if outline_status == 'add_outline':
        diagram.add_outline(element, path, parent)
        return

    if element.get('outline', 'no') == 'yes' or diagram.output_format() == 'tactile':
        diagram.add_outline(element, path, parent)
        finish_outline(element, diagram, parent)

    else:
        parent.append(path)

# Here are some tags that we can append to a path        
graphical_tags = {'graph',
                  'parametric-curve',
                  'polygon'}

def process_tag(child, diagram, cmds, current_point):
    if child.tag == "moveto":
        try:
            user_point = un.valid_eval(child.get('point'))
            point = diagram.transform(user_point)
        except:
            log.error(f"Error in <moveto> defining point={child.get('point')}")
            return
        cmds.append('M')
        cmds.append(util.pt2str(point))
        current_point = user_point
        return cmds, current_point
    
    if child.tag == "rmoveto":
        try:
            user_point = un.valid_eval(child.get('point'))
        except:
            log.error(f"Error in <rmoveto> defining point={child.get('point')}")
            return
        current_point = current_point + user_point
        point = diagram.transform(current_point)
        cmds.append('M')
        cmds.append(util.pt2str(point))
        return cmds, current_point
    
    if child.tag == "horizontal":
        try:
            distance = un.valid_eval(child.get('distance'))
        except:
            log.error(f"Error in <horizontal> defining distance={child.get('distance')}")
            return
        user_point = (current_point[0] + distance,
                      current_point[1])
        child.tag = 'lineto'
        child.set('point', util.pt2long_str(user_point,spacer=","))
            
    if child.tag == "vertical":
        try:
            distance = un.valid_eval(child.get('distance'))
        except:
            log.error(f"Error in <vertical> defining distance={child.get('distance')}")
            return
        user_point = (current_point[0],
                      current_point[1] + distance)
        child.tag = 'lineto'
        child.set('point', util.pt2long_str(user_point,spacer=","))
            
    if child.tag == "rlineto":
        try:
            user_point = un.valid_eval(child.get('point'))
            user_point = current_point + user_point
        except:
            log.error(f"Error in <rlineto> defining point={child.get('point')}")
            return
        child.tag = "lineto"
        child.set('point', util.pt2long_str(user_point,spacer=","))
    
    if child.tag == "lineto":
        if child.get('decoration', None) is not None:
            return decorate(child,
                            diagram,
                            current_point,
                            cmds)
        try:
            user_point = un.valid_eval(child.get('point'))
            point = diagram.transform(user_point)
        except:
            log.error(f"Error in <lineto> defining point={child.get('point')}")
            return
        cmds.append('L')
        cmds.append(util.pt2str(point))
        current_point = user_point
        return cmds, current_point
        
    if child.tag == "cubic-bezier":
        cmds.append('C')
        try:
            user_control_pts = un.valid_eval(child.get('controls'))
            control_pts= [diagram.transform(p) for p in user_control_pts]
        except:
            log.error(f"Error in <cubic-bezier> defining controls={child.get('controls')}")
            return
        cmds.append(' '.join([util.pt2str(p) for p in control_pts]))
        current_point = user_control_pts[-1]
        return cmds, current_point
    if child.tag == "quadratic-bezier":
        cmds.append('Q')
        try:
            user_control_pts = un.valid_eval(child.get('controls'))
            control_pts= [diagram.transform(p) for p in user_control_pts]
        except:
            log.error(f"Error in <cubic-bezier> defining controls={child.get('controls')}")
            return
        cmds.append(' '.join([util.pt2str(p) for p in control_pts]))
        current_point = user_control_pts[-1]
        return cmds, current_point
    # Let's take these out to facilitate shape handling
    '''
    if child.tag == "smooth-cubic":
        cmds.append('S')
        user_control_pts = un.valid_eval(child.get('controls'))
        control_pts= [diagram.transform(p) for p in user_control_pts]
        cmds.append(' '.join([util.pt2str(p) for p in control_pts]))
        current_point = user_control_pts[-1]
        return cmds, current_point
    if child.tag == "smooth-quadratic":
        cmds.append('T')
        user_point = un.valid_eval(child.get('point'))
        point = diagram.transform(user_point)
        cmds.append(util.pt2str(point))
        current_point = user_point
        return cmds, current_point
    '''
    if child.tag == 'arc':
        try:
            center = un.valid_eval(child.get('center'))
            radius = un.valid_eval(child.get('radius'))
            angular_range = un.valid_eval(child.get('range'))
        except:
            log.error("Error in <arc> defining data: @center, @radius, or @range")
            return
        if child.get('degrees', 'yes') == 'yes':
            angular_range = [math.radians(a) for a in angular_range]
        N = 100
        t = angular_range[0]
        dt = (angular_range[1] - angular_range[0]) / N
        user_start = [center[0] + radius * math.cos(t),
                      center[1] + radius * math.sin(t)]
        start = diagram.transform(user_start)
        cmds += ['L', util.pt2str(start)]
        for _ in range(N):
            t += dt
            user_point = [center[0] + radius * math.cos(t),
                          center[1] + radius * math.sin(t)]
            point = diagram.transform(user_point)
            cmds += ['L', util.pt2str(point)]
        return cmds, current_point

    if child.tag == 'repeat':
        try:
            parameter = child.get('parameter')
            var, expr = parameter.split('=')
            var = var.strip()
            start, stop = map(un.valid_eval, expr.split('..'))
        except:
            log.error(f"Error in <repeat> defining parameter={child.get('parameter')}")
            return

        for k in range(start, stop+1):
            k_str = str(k)
            un.valid_eval(k_str, var)

            for sub_child in child:
                cmds, current_point = process_tag(sub_child,
                                                  diagram,
                                                  cmds,
                                                  current_point)
        return cmds, current_point

    if child.tag in graphical_tags:
        dummy_parent = ET.Element('group')
        tags.parse_element(child, diagram, dummy_parent)
        child_cmds = dummy_parent.getchildren()[0].get('d').strip()
        if child_cmds[0] == 'M':
            child_cmds = 'L' + child_cmds[1:]
        if child_cmds[-1] == 'Z':
            child_cmds = child_cmds[:-1].strip()
        cmds.append(child_cmds)
        coordinates = child_cmds.split()
        final_point = [float(c) for c in coordinates[-2:]]
        current_point = diagram.inverse_transform(final_point)
        return cmds, current_point
        
    log.warning(f"Unknown tag in <path>: {child.tag}")
    return cmds, current_point
        
def decorate(child, diagram, current_point, cmds):
    user_point = un.valid_eval(child.get('point'))
    ctm = CTM.CTM()
    p0 = diagram.transform(current_point)
    p1 = diagram.transform(user_point)
    diff = p1 - p0
    length = math_util.length(diff)
    ctm.translate(*p0)
    ctm.rotate(math.atan2(diff[1], diff[0]), units="rad")

    decoration = child.get('decoration')
    decoration_data = [d.strip() for d in decoration.split(';')]
    if decoration_data[0] == 'coil':
        # number, center, dimensions
        try:
            data = [d.split('=') for d in decoration_data[1:]]
            data = {k:v for k, v in data}
            dimensions = un.valid_eval(data.get('dimensions', '(10,5)'))
            location = un.valid_eval(data.get('center', '0.5'))
        except:
            log.error("Error processing decoration data for a coil")
            return
        if data.get('number', None) is None:
            number = math.floor((length - dimensions[0]/2)/dimensions[0])
        else:
            number = un.valid_eval(data.get('number'))
        half_coil_fraction = (number+0.5)*dimensions[0] / length
        while (
                location - half_coil_fraction < 0 or
                location + half_coil_fraction > 1
        ):
            number -= 1
            half_coil_fraction = (number+0.5)*dimensions[0] / length
        start_coil = length * (location - half_coil_fraction)
        end_coil = length * (location + half_coil_fraction)
        coil_length = 2 * half_coil_fraction * length

        N = 40
        dt = 2*math.pi/N
        t = 0
        #x_init = leftover/2
        x_init = start_coil
        x_pos = x_init + dimensions[0]/2
        iterates = math.floor((number+0.5)*N)
        cmds += ['L', util.pt2str(ctm.transform((x_init,0)))]
        dx = (coil_length-dimensions[0])/iterates
        for _ in range(iterates):
            y = -dimensions[1] * math.sin(t)
            x_pos += dx
            x = x_pos - dimensions[0]/2 * math.cos(t)
            t += dt
            cmds += ['L', util.pt2str(ctm.transform((x, y)))]
        cmds += ['L', util.pt2str(ctm.transform((x,0)))]
        cmds += ['L', util.pt2str(ctm.transform((length, 0)))]

    if decoration_data[0] == 'zigzag':
        # number, location, dimensions
        try:
            data = [d.split('=') for d in decoration_data[1:]]
            data = {k:v for k, v in data}
            dimensions = un.valid_eval(data.get('dimensions', '(10,5)'))
            location = un.valid_eval(data.get('center', '0.5'))
        except:
            log.error("Error processing zigzag decoration data")
            return
        if data.get('number', None) is None:
            number = math.floor((length - dimensions[0]/2)/dimensions[0])
        else:
            number = un.valid_eval(data.get('number'))

        half_zig_fraction = number*dimensions[0] / length
        while (
                location - half_zig_fraction < 0 or
                location + half_zig_fraction > 1
        ):
            number -= 1
            half_zig_fraction = number*dimensions[0] / length
        start_zig = length * (location - half_zig_fraction)
        end_zig = length * (location + half_zig_fraction)
        zig_length = 2 * half_zig_fraction * length
        

        N = 4
        dt = 2*math.pi/N
        t = 0
        x_pos = start_zig
        iterates = math.floor(number*N)
        cmds += ['L', util.pt2str(ctm.transform((x_pos,0)))]
        dx = zig_length/iterates
        y = 0
        for _ in range(iterates):
            t += dt
            x_pos += dx
            y = -dimensions[1] * math.sin(t)
            cmds += ['L', util.pt2str(ctm.transform((x_pos, y)))]
        cmds += ['L', util.pt2str(ctm.transform((x_pos,0)))]
        cmds += ['L', util.pt2str(ctm.transform((length, 0)))]

    if decoration_data[0] == 'wave':
        # number, location, dimensions
        try:
            data = [d.split('=') for d in decoration_data[1:]]
            data = {k:v for k, v in data}
            dimensions = un.valid_eval(data.get('dimensions', '(10,5)'))
            location = un.valid_eval(data.get('center', '0.5'))
        except:
            log.error("Error in wave decoration data")
            return
        if data.get('number', None) is None:
            number = math.floor((length - dimensions[0]/2)/dimensions[0])
        else:
            number = un.valid_eval(data.get('number'))

        half_wave_fraction = number*dimensions[0] / length
        while (
                location - half_wave_fraction < 0 or
                location + half_wave_fraction > 1
        ):
            number -= 1
            half_wave_fraction = number*dimensions[0] / length
        start_wave = length * (location - half_wave_fraction)
        end_wave = length * (location + half_wave_fraction)
        wave_length = 2 * half_wave_fraction * length
        

        N = 30
        dt = 2*math.pi/N
        t = 0
        x_pos = start_wave
        iterates = math.floor(number*N)
        cmds += ['L', util.pt2str(ctm.transform((x_pos,0)))]
        dx = wave_length/iterates
        y = 0
        for _ in range(iterates):
            t += dt
            x_pos += dx
            y = -dimensions[1] * math.sin(t)
            cmds += ['L', util.pt2str(ctm.transform((x_pos, y)))]
        cmds += ['L', util.pt2str(ctm.transform((x_pos,0)))]
        cmds += ['L', util.pt2str(ctm.transform((length, 0)))]

    if decoration_data[0] == 'capacitor':
        try:
            data = [d.split('=') for d in decoration_data[1:]]
            data = {k:v for k, v in data}
            dimensions = un.valid_eval(data.get('dimensions', '(10,5)'))
            location = un.valid_eval(data.get('center', '0.5'))
        except:
            log.error("Error in capacitor decoration data")
            return
        x_mid = length * location
        x0 = x_mid - dimensions[0]/2
        x1 = x_mid + dimensions[0]/2

        cmds += ['L', util.pt2str(ctm.transform((x0,0)))]
        cmds += ['M', util.pt2str(ctm.transform((x0,dimensions[1])))]
        cmds += ['L', util.pt2str(ctm.transform((x0,-dimensions[1])))]
        cmds += ['M', util.pt2str(ctm.transform((x1,dimensions[1])))]
        cmds += ['L', util.pt2str(ctm.transform((x1,-dimensions[1])))]
        cmds += ['M', util.pt2str(ctm.transform((x1,0)))]
        cmds += ['L', util.pt2str(ctm.transform((length, 0)))]

    return cmds, user_point
    

def finish_outline(element, diagram, parent):
    diagram.finish_outline(element,
                           element.get('stroke'),
                           element.get('thickness'),
                           element.get('fill', 'none'),
                           parent)

