import csv
import numpy as np
from . import user_namespace as un

import logging
log = logging.getLogger('prefigure')

def read(element, diagram, parent, outline_status):
    filename = element.get('filename', None)
    if filename is None:
        log.error('A <read> element needs a @filename attribute')
        return
    name = element.get('name', None)
    if name is None:
        log.error('A <read> element needs a @name attribute')
        return
    filetype = element.get('type', 'csv')

    csv_data = {}
    with open(filename, newline='') as csvfile:
        reader = csv.reader(csvfile)
        headers = next(reader)
        for row in reader:
            for i, header in enumerate(headers):
                header_list = csv_data.get(header, [])
                header_list.append(float(row[i]))
                csv_data[header] = header_list

    for header in headers:
        csv_data[header] = np.array(csv_data[header])

    un.enter_namespace(name, csv_data)
    

    

    
