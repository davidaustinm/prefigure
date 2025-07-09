import csv
import numpy as np
from pathlib import Path
from . import user_namespace as un

import logging
log = logging.getLogger('prefigure')

def read(element, diagram, parent, outline_status):
    filename = element.get('filename', None)
    if filename is None:
        log.error('A <read> element needs a @filename attribute')
        return
    filename = Path(filename)
    external_root = diagram.get_external()
    if diagram.get_environment() == "pretext":
        filename = "data" / filename
    else:
        if external_root is not None:
            external_root = Path(external_root)
            filename = external_root / filename

    name = element.get('name', None)
    if name is None:
        log.error('A <read> element needs a @name attribute')
        return
    filetype = element.get('type', 'csv')

    if filetype == 'csv':
        load_csv(element, diagram, filename, name)

def load_csv(element, diagram, filename, name):
    csv_data = {}

    delimiter = element.get('delimiter',',')
    quotechar = element.get('quotechar',"'")
    str_cols  = element.get('string-columns','[]')
    str_cols = un.valid_eval(str_cols)
    str_cols  = set(str_cols)

    with open(filename, newline='') as csvfile:
        reader = csv.reader(csvfile,
                            delimiter=delimiter,
                            quotechar=quotechar)
        headers = next(reader)
        for row in reader:
            for i, header in enumerate(headers):
                header_list = csv_data.get(header, [])
                if header not in str_cols:
                    try:
                        header_list.append(float(row[i]))
                    except:
                        header_list.append(row[i])
                else:
                    header_list.append(row[i])
                csv_data[header] = header_list

    for header in headers:
        csv_data[header] = np.array(csv_data[header])

    un.enter_namespace(name, csv_data)
    

    

    
