import sys
import getopt
import importlib
import diagram
import user_namespace
import lxml.etree as ET

# read arguments to determine which format we're going to use
# TODO:  add output option

argumentList = sys.argv[1:]
options = "f:"
long_options = ["format="]
format = 'svg'

try:
    arguments, values = getopt.getopt(argumentList, options, long_options)
    for currentArgument, currentValue in arguments:
        if currentArgument in ("-f", "--format"):
            format = currentValue

except getopt.error as err:
    print(str(err))

try:
    filename = sys.argv[-1]
except IndexError:
    print('Usage: python parse.py -f format xml_filename')
    sys.exit()

# now we'll pull out each of the diagram elements from the file
# and create a separate image for each

tree = ET.parse(filename)
diagrams = list(tree.iter(tag='diagram'))

for diagram_number, element in enumerate(diagrams):
    importlib.reload(user_namespace)
    if len(diagrams) == 1:
        diagram_number = None
    diagram = diagram.Diagram(element, filename, diagram_number, format)
    diagram.begin_figure()
    diagram.parse()
    diagram.place_labels()
    diagram.end_figure()
