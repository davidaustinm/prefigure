import sys
import argparse
import importlib
import logging
import lxml.etree as ET
from . import diagram
from . import user_namespace

log = logging.getLogger('prefigure')

# `user_namespace` implements <definition> by writing straight into its own
# module globals (`globals()[name] = value`), so a definition that happens to
# reuse a Python builtin's name (e.g. `<definition>dir = ...</definition>`)
# overwrites that builtin *in the module's own namespace*. That's normally
# invisible, since a standalone `prefig build` is a fresh process every time.
# But `importlib.reload()` re-executes a module's source using its *existing*
# globals dict rather than a fresh one, so in a long-lived process (the
# playground's Pyodide session, or anything else building multiple diagrams
# without restarting Python) the clobbered name survives into the next
# build's reload and can break the module's own top-level code (e.g. its own
# `dir(math)` call resolves to the leftover value instead of the builtin).
# We snapshot the pristine name set once, right after first import, and strip
# anything else out before every reload so each build starts from a clean
# module namespace, matching what a fresh process would give it.
_USER_NAMESPACE_BASELINE = frozenset(vars(user_namespace).keys())

# This function does the main work of constructing a diagram.
# This can be called from outside the project to allow, say,
# for generating assets in a pretext document
def mk_diagram(element,
               format,
               publication,
               filename,
               suppress_caption,
               diagram_number,
               environment,
               return_string=False):

    for name in list(vars(user_namespace).keys()):
        if name not in _USER_NAMESPACE_BASELINE:
            delattr(user_namespace, name)
    importlib.reload(user_namespace)
    output = None # add at a later date
    diag = diagram.Diagram(element, filename, diagram_number, 
                           format, output, publication, suppress_caption,
                           environment)
    log.debug("Initializing PreFigure diagram")
    try:
        diag.begin_figure()
    except:
        log.error("There was a problem initializing the PreFigure diagram")
        log.error("Debugging information is available with 'prefig -vv build filename'")
        return
    log.debug("Processing PreFigure elements")
    try:
        diag.parse()
    except:
        log.error("There was a problem parsing a PreFigure element")
        log.error("Debugging information is available with 'prefig -vv build filename'")
        return
    log.debug("Positioning labels")
    try:
        diag.place_labels()
    except:
        log.error("There was a problem placing the labels in the diagram")
        log.error("Debugging information is available with 'prefig -vv build filename'")
        return
    log.debug("Writing the diagram and any annotations")
    try:
        diag.annotate_source()
    except:
        log.error("There was a problem generating annotations for this diagram")
        log.error("Debugging information is available with 'prefig -vv build filename'")
    try:
        if return_string:
            return diag.end_figure_to_string()
        else:
            diag.end_figure()
    except:
        log.error("There was a problem finishing the diagram")
        log.error("Debugging information is available with 'prefig -vv build filename'")
        return

def parse(filename, format, pub_file, suppress_caption, environment):
    # Load the publication file, if there is one
    ns = {'pf': 'https://prefigure.org'}
    if pub_file is not None:
        try:
            publication = ET.parse(pub_file)
        except:
            log.error(f"Unable to parse publication file {pub_file}")
            return
        pubs_with_ns = publication.xpath('//pf:prefigure', namespaces=ns)
        pubs_without_ns = publication.xpath('//prefigure', namespaces=ns)
        try:
            publication = (pubs_with_ns + pubs_without_ns)[0]
            for child in publication:
                if child.tag is ET.Comment:
                    continue
                child.tag = ET.QName(child).localname
        except IndexError:
            log.warning('Publication file should have a <prefigure> element')
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

        for elem in element.iter():
            # Skip comments and processing instructions,
            # because they do not have names
            if not (
                    isinstance(elem, ET._Comment)
                    or isinstance(elem, ET._ProcessingInstruction)
            ):
                # Remove a namespace URI in the element's name
                elem.tag = ET.QName(elem).localname
        
        if len(diagrams) == 1:
            diagram_number = None
            check_duplicate_handles(element, set())

            # we're publicly using 'at' rather than 'id' for handles
            for elem in element.iter():
                if elem.get('at', None) is not None:
                    elem.set('id', elem.get('at'))

            mk_diagram(element, format, publication,
                       filename, suppress_caption, diagram_number,
                       environment)

def check_duplicate_handles(element, handles):
    for child in element:
        id1 = child.get('id', None)
        id2 = child.get('at', None)
        for id in [id1, id2]:
            if id is not None:
                if id in handles:
                    log.warning(f"Duplicate handle: {id}.  Unexpected behavior could result.")
                else:
                    handles.add(id)
        check_duplicate_handles(child, handles)

if __name__ == '__main__':
    main()
    
