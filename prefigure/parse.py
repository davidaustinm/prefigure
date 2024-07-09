import sys
import argparse
import importlib
import diagram
import user_namespace
import lxml.etree as ET

# read arguments to determine which format we're going to use
# TODO:  add output option

parser = argparse.ArgumentParser(description='Compile a PreFigure source file')
parser.add_argument('filename')
parser.add_argument('-f', '--format', default="svg")
parser.add_argument('-p', '--publication_file')
parser.add_argument('-o', '--output')

args = parser.parse_args()
filename = args.filename
format = args.format
output = args.output
pub_file = args.publication_file

if output is not None:
    if output[:-4]+'.xml' == filename or output+'.xml' == filename:
        print('Annotations will overwrite PreFigure source file:', filename)
        print('Choose a different output file')
        sys.exit()

if pub_file is not None:
    publication = ET.parse(pub_file).iter(tag='prefigure')
    try:
        publication = list(publication)[0]
    except IndexError:
        print('Publication file should have a <prefigure> element')
        publication = None
else:
    publication = None

# now we'll pull out each of the diagram elements from the file
# and create a separate image for each
tree = ET.parse(filename)
diagrams = list(tree.iter(tag='diagram'))

for diagram_number, element in enumerate(diagrams):
    importlib.reload(user_namespace)
    if len(diagrams) == 1:
        diagram_number = None
    diagram = diagram.Diagram(element, filename, diagram_number, 
                              format, output, publication)
    diagram.begin_figure()
    diagram.parse()
    diagram.place_labels()
    diagram.end_figure()
