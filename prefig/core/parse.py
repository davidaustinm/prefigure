import sys
import argparse
import importlib
import lxml.etree as ET
from . import diagram
from . import user_namespace

# This function does the main work of constructing a diagram.
# This can be called from outside the project to allow, say,
# for generating assets in a pretext document
def mk_diagram(element, format, publication, 
               filename, diagram_number = None):
    importlib.reload(user_namespace)
    output = None # add at a later date
    diag = diagram.Diagram(element, filename, diagram_number, 
                           format, output, publication)
    diag.begin_figure()
    diag.parse()
    diag.place_labels()
    diag.end_figure()

def parse(filename, format, pub_file):
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
            mk_diagram(element, format, publication,
                       filename, diagram_number)


if __name__ == '__main__':
    main()
    
