import os
import shutil
import subprocess
from pathlib import Path
from . import core
import logging
import lxml.etree as ET

# We're going to include some basic functions here so they can be
# called from one of several environments, either the standalone CLI,
# PreTeXt, or Pyodide

# First we set up logging.  Unless requested, we won't report a lot
log = logging.getLogger('prefigure')
log.setLevel(logging.WARNING)

# Now set up the format of log messgaes
console = logging.StreamHandler()
log_format = logging.Formatter('PreFigure: %(levelname)-8s: %(message)s')
console.setFormatter(log_format)
log.addHandler(console)

# are we inside PreTeXt?  If so, we check its verbosity level of log messages
ptx_log_name = 'ptxlogger'
if ptx_log_name in logging.Logger.manager.loggerDict:
    ptx_log = logging.getLogger('ptx_log_name')
    if ptx_log.getEffectiveLevel() == logging.DEBUG:
        log.setLevel(logging.DEBUG)


# we make the default environment pretext so we don't need to change
# the call from within pretext
def build(
        format,
        filename,
        publication=None,
        ignore_publication=False,
        suppress_caption=False,
        environment="pretext"
):
    pub_requested = not ignore_publication and publication is not None
    path = Path(filename)
    if path.suffix != '.xml':
        filename = str(path.parent / (path.stem + '.xml'))

    # We're going to look for a publication, possibly in a parent directory
    # unless we're told to ignore any publication file
    if ignore_publication and publication is None:
        publication = None
    else:
        if publication is None:
            pub_name = 'pf_publication.xml'
        else:
            pub_name = publication
        cwd = Path(os.getcwd())
        dirs = [cwd] + list(cwd.parents)
        pub_file_found = False
        for dir in dirs:
            pub = dir / pub_name
            if pub.exists():
                publication = pub
                log.info(f"Applying PreFigure publication file {publication}")
                pub_file_found = True
                break
        if publication is None or not pub_file_found:
            publication = None
            if pub_requested:
                log.warning("PreFigure publication file not found")
            else:
                log.info("No PreFigure publication file applied.")

    log.info(f"Building from PreFigure source {filename}")

    core.parse.parse(filename,
                     format,
                     publication,
                     suppress_caption,
                     environment)
    return filename


# Build from an input string and return a string formed from
# an XML tree containing the SVG and annotation trees
def build_from_string(format, input_string, environment="pyodide"):
    tree = ET.fromstring(input_string)
    log.setLevel(logging.DEBUG)
    diagrams = tree.xpath('//diagram')
    if len(diagrams) > 0:
        output_string = core.parse.mk_diagram(
            diagrams[0],
            format,
            None,     # publication file
            "prefig", # filename needed for label generation
            False,    # supress caption
            None,     # diagram number
            environment,
            return_string = True
        )
        return output_string
    return ''

def pdf(
        format,
        filename,
        build_first=True,
        publication=None,
        ignore_publication=False,
        dpi=72,
        environment="pretext"
):
    build_path = None
    if build_first:
        filename = build(format,
                         filename,
                         publication=publication,
                         ignore_publication=ignore_publication,
                         environment=environment)
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
            log.debug(f"Unable to find {filename}")
            return

    dpi = str(dpi)
    executable = shutil.which('rsvg-convert')
    if executable is None:
        log.debug("rsvg-convert is required to create PDFs.")
        log.debug("See the installation instructions at https://prefigure.org")
        return
    
    log.info(f"Converting {build_path} to PDF")
    output_file = build_path.parent / (build_path.stem + '.pdf')
    pdf_args = ['-a','-d',dpi,'-p',dpi,'-f','pdf','-o']
    pdf_args = ['rsvg-convert'] + pdf_args + [output_file,build_path]

    try:
        subprocess.run(pdf_args)
    except:
        log.error("PreFigure PDF conversion failed.  Is rsvg-convert available?")
        log.error("See the installation instructions athttps://prefigure.org")

    if environment == "pretext":
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
        environment="pretext"
):
    build_path = None
    if build_first:
        filename = build(format,
                         filename,
                         publication=publication,
                         ignore_publication=ignore_publication,
                         environment=environment)
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
            log.debug(f"Unable to find {filename}")
            return

    log.info(f"Converting {build_path} to PDF")
    output_file = build_path.parent / (build_path.stem + '.png')

    # we use rsvg-convert to make a PNG with dpi=300
    png_args = ['-a','-d', str(300),'-p', str(300),'-f','png','-o']
    png_args = ['rsvg-convert'] + png_args + [output_file,build_path]
    try:
        subprocess.run(png_args)
    except:
        log.error("PreFigure PNG conversion failed.  Is rsvg-convert available?")
        log.error("See the installation instructions at https://prefigure.org")

    if environment == "pretext":
        os.remove(build_path)
        annotations = str(build_path.parent/build_path.stem) + '-annotations.xml'
        try:
            os.remove(annotations)
        except FileNotFoundError:
            pass

def validate_source(xml_file):
    log_level = log.getEffectiveLevel()
    log.setLevel(logging.INFO)
    # we first load the RelaxNG schema
    engine_dir = Path(__file__).parent
    schema_rng = engine_dir / "resources" / "schema" / "pf_schema.rng"
    schema = ET.RelaxNG(file=schema_rng)

    log.info(f"Validating {xml_file} with PreFigure schema {schema_rng}")

    # now load the XML file and look for diagrams either in a pf namespace or no
    try:
        tree = ET.parse(xml_file)
    except:
        log.error(f"Could not load {xml_file}")
        log.setLevel(log_level)
        return
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
                log.info("Diagram is valid for SVG production")
            else:
                log.warning(f"diagram {num+1} is valid for SVG production")
        except:
            if len(all_diagrams) == 1:
                log.info("Diagram failed validation for SVG production")
            else:
                log.warning(f"diagram {num+1} failed validation for SVG production")

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
                log.info("Diagram is valid for tactile production")
            else:
                log.warning(f"diagram {num+1} is valid for tactile production")
        except:
            if len(all_diagrams) == 1:
                log.info("Diagram failed validation for tactile production")
            else:
                log.warning(f"diagram {num+1} failed validation for tactile production")
                
    log.setLevel(log_level)
