import os
import shutil
import subprocess
from pathlib import Path
from . import core
import logging
import lxml.etree as ET

# We're going to include some basic functions here so they can be
# called from an import

#log = logging.getLogger('ptxlogger')

def build(
        format,
        filename,
        publication=None,
        ignore_publication=False,
        standalone=False
):
    path = Path(filename)
    if path.suffix != '.xml':
        filename = str(path.parent / (path.stem + '.xml'))

#    log.info(f'Building from PreFigure source {filename}')

    # We're going to look for a publication, possibly in a parent directory
    # unless we're told to ignore any publication file
    if ignore_publication:
        publication = None
    else:
        if publication is None:
            pub_name = 'pf_publication.xml'
        else:
            pub_name = publication
        cwd = Path(os.getcwd())
        dirs = [cwd] + list(cwd.parents)
        for dir in dirs:
            pub = dir / pub_name
            if pub.exists():
                publication = pub
                break

    core.parse.parse(filename, format, publication, standalone)
    return filename


def pdf(
        format,
        filename,
        build_first=True,
        publication=None,
        ignore_publication=False,
        dpi=72,
        standalone=False
):
    build_path = None
    if build_first:
        filename = build(format,
                         filename,
                         publication=publication,
                         ignore_publication=ignore_publication)
        filename = Path(filename)
        build_path = filename.parent / 'output' / (filename.stem + '.svg')
    else:
        filename = Path(filename)
    
    if filename.suffix != '.svg':
        filename = filename.parent / (filename.name + '.svg')

    if build_path is None:
        filename_str = str(filename)
        for dir, dirs, files in os.walk(os.getcwd()):
            files = set(files)
            if filename_str in files:
                build_path = dir / filename
        if build_path is None:
#            log.debug(f'Unable to find {filename}')
            return

    dpi = str(dpi)
    executable = shutil.which('rsvg-convert')
    if executable is None:
#        log.debug('rsvg-convert is required to create PDFs.')
#        log.debug('See the installation instructions at https://prefigure.org')
        return
    
#    log.info(f'Converting {build_path} to PDF')
    output_file = build_path.parent / (build_path.stem + '.pdf')
    pdf_args = ['-a','-d',dpi,'-p',dpi,'-f','pdf','-o']
    pdf_args = ['rsvg-convert'] + pdf_args + [output_file,build_path]
    subprocess.run(pdf_args)

    if not standalone:
        os.remove(build_path)
        annotations = str(build_path.parent/build_path.stem) + '-annotations.xml'
        try:
            os.remove(annotations)
        except FileNotFoundError:
            pass

def png(
        format,
        filename,
        build_first=True,
        publication=None,
        ignore_publication=False,
        standalone=False
):
    build_path = None
    if build_first:
        filename = build(format,
                         filename,
                         publication=publication,
                         ignore_publication=ignore_publication)
        filename = Path(filename)
        build_path = filename.parent / 'output' / (filename.stem + '.svg')
    else:
        filename = Path(filename)
    
    if filename.suffix != '.svg':
        filename = filename.parent / (filename.stem + '.svg')

    if build_path is None:
        filename_str = str(filename)
        for dir, dirs, files in os.walk(os.getcwd()):
            files = set(files)
            if filename_str in files:
                build_path = dir / filename
        if build_path is None:
#            log.debug(f'Unable to find {filename}')
            return

#    log.info(f'Converting {build_path} to PDF')
    output_file = build_path.parent / (build_path.stem + '.png')

    import cairosvg
    cairosvg.svg2png(url=str(build_path), write_to=str(output_file))

    if not standalone:
        os.remove(build_path)
        annotations = str(build_path.parent/build_path.stem) + '-annotations.xml'
        try:
            os.remove(annotations)
        except FileNotFoundError:
            pass

def validate_source(xml_file):
    # we first load the RelaxNG schema
    engine_dir = Path(__file__).parent
    schema_rng = engine_dir / "resources" / "schema" / "pf_schema.rng"
    schema = ET.RelaxNG(file=schema_rng)

    # now load the XML file and look for diagrams either in a pf namespace or no
    tree = ET.parse(xml_file)
    ns = {'pf':'https://prefigure.org'}
    pf_diagrams = tree.xpath('//pf:diagram', namespaces=ns)
    diagrams = tree.xpath('//diagram')
    all_diagrams = pf_diagrams + diagrams

    # iterate through the diagrams we have found
    for num, diagram in enumerate(all_diagrams):
        tactile_attr_dict = {}
        # strip out the namespace, if one is present
        for elem in diagram.getiterator():
            # Skip comments and processing instructions,
            # because they do not have names
            if not (
                isinstance(elem, ET._Comment)
                or isinstance(elem, ET._ProcessingInstruction)
            ):
                # Remove a namespace URI in the element's name
                elem.tag = ET.QName(elem).localname

                # save and remove attributes that begin with 'tactile-' 
                tactile_attrs = {}
                for attr, value in elem.attrib.items():
                    if attr.startswith('tactile-'):
                        tactile_attrs[attr[8:]] = value
                        elem.attrib.pop(attr)
                tactile_attr_dict[elem] = tactile_attrs

        # now validate
        try:
            schema.assertValid(diagram)
            if len(all_diagrams) == 1:
                print("Diagram is valid for SVG production")
            else:
                print(f"diagram {num+1} is valid for SVG production")
        except:
            if len(all_diagrams) == 1:
                print("Diagram failed validation for SVG production")
            else:
                print(f"diagram {num+1} failed validation for SVG production")

    # now validate tactile diagram
    for diagram in all_diagrams:
        # work through the XML tree and replace tactile attributes
        for elem in diagram.getiterator():
            if not (
                    isinstance(elem, ET._Comment)
                    or isinstance(elem, ET._ProcessingInstruction)
            ):
                tactile_attrs = tactile_attr_dict[elem]
                for attr, value in tactile_attrs.items():
                    elem.set(attr, value)
                    
        # now validate
        try:
            schema.assertValid(diagram)
            if len(all_diagrams) == 1:
                print("Diagram is valid for tactile production")
            else:
                print(f"diagram {num+1} is valid for tactile production")
        except:
            if len(all_diagrams) == 1:
                print("Diagram failed validation for tactile production")
            else:
                print(f"diagram {num+1} failed validation for tactile production")
                
