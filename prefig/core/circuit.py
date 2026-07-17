import lxml.etree as ET
import logging
import numpy as np
from . import user_namespace as un
from . import utilities as util
from . import math_utilities as math_util
from . import CTM
from . import circuit_geometry

log = logging.getLogger('prefigure')

tags = {
    'battery': circuit_geometry.shapes.battery,
    'op-amp': circuit_geometry.shapes.op_amp,
    'dc-current-source': circuit_geometry.shapes.dc_current_source,
    'diode': circuit_geometry.shapes.diode,
    'connection': circuit_geometry.connections.connection,
}

# Process a circuit tag
def circuit(element, diagram, parent, outline_group):
    convention = element.get('convention', 'US')
    scale = 20 * un.valid_eval(element.get('scale', '1'))
    data = {
        'convention': convention,
        'scale': scale,
    }
    for child in element:
        function = tags.get(child.tag, None)
        if function is None:
            log.error(f"{child.tag} element is not allowed in a <circuit>")
            continue
        function(child, diagram, parent, data)

