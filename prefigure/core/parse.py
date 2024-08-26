import sys
import argparse
import importlib
import diagram
import user_namespace
import lxml.etree as ET

# This function does the main work of constructing a diagram.
# This can be called from outside the project to allow, say,
# for generating assets in a pretext document
def mk_diagram(element, format, output, publication, 
               filename, diagram_number = None):
    importlib.reload(user_namespace)
    diag = diagram.Diagram(element, filename, diagram_number, 
                           format, output, publication)
    diag.begin_figure()
    diag.parse()
    diag.place_labels()
    diag.end_figure()

# read arguments to obtain basic data about the diagram we're creating
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

# Since the annotations create an XML file, we want to be careful
# not to overwrite the source XML
if output is not None:
    if output[:-4]+'.xml' == filename or output+'.xml' == filename:
        print('Annotations will overwrite PreFigure source file:', filename)
        print('Choose a different output file')
        sys.exit()

# Load the publication file, if there is one
ns = {'pf': 'http://prefigure.org'}
if pub_file is not None:
    publication = ET.parse(pub_file)
    pubs_with_ns = publication.xpath('//pf:prefigure', namespaces=ns)
    pubs_without_ns = publication.xpath('//prefigure', namespaces=ns)
    try:
        publication = (pubs_with_ns + pubs_without_ns)[0]
        for child in publication:
            if child.tag is ET.Comment:
                continue
            child.tag = ET.QName(child).localname
    except IndexError:
        print('Publication file should have a <prefigure> element')
        publication = None
else:
    publication = None

# now we'll pull out each of the diagram elements from the file
# and create a separate image for each.  We'll allow authors to
# delcare diagrams within the pf namespace
tree = ET.parse(filename)
diagrams_with_ns = tree.xpath('//pf:diagram', namespaces=ns)
diagrams_without_ns = tree.xpath('//diagram', namespaces=ns)
diagrams = diagrams_with_ns + diagrams_without_ns

for diagram_number, element in enumerate(diagrams):
    if len(diagrams) == 1:
        diagram_number = None
    mk_diagram(element, format, output, publication,
               filename, diagram_number)

    
