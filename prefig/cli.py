import click
import click_log
import os
import sys
import psutil
import shutil
import subprocess
import time
import logging
import webbrowser
from pathlib import Path
from . import core
from . import engine

# the log is configured inside engine and will be available now
log = logging.getLogger('prefigure')
log.handlers.clear()
click_handler = logging.StreamHandler(sys.stdout)
click_handler.setFormatter(click_log.ColorFormatter())
log.addHandler(click_handler)


CONTEXT_SETTINGS = dict(help_option_names=["-h", "--help"])


@click.group(invoke_without_command=True, context_settings=CONTEXT_SETTINGS)
@click.pass_context
@click.option(
    '-v',
    '--verbose',
    count=True,
    help='-v for information and -vv for debugging'
)
def main(ctx, verbose):
    '''
    This is the command line interface to PreFigure 

    More information is available at https://prefigure.org

    For help with a specific command, append --help to the command,
    e.g. prefig build --help
    '''
    if verbose == 1:
        log.setLevel(logging.INFO)
    elif verbose > 1:
        log.setLevel(logging.DEBUG)
    if ctx.invoked_subcommand is None:
        click.echo("Run `prefig --help` for help.")

        
@main.command(
    help="Initializes the local installation of PreFigure"
)
def init():
    log_level = log.getEffectiveLevel()
    log.setLevel(log_level)
    log.info('Initializing PreFigure installation')

    prefig_root = Path(__file__).parent
    destination = prefig_root / "core" / "mj_sre"

    shutil.copytree(
        prefig_root / "resources" / "js",
        destination,
        dirs_exist_ok = True
    )

    shutil.rmtree(destination / "node_modules", ignore_errors=True)

    # we need to change into MathJax directory to install
    # on Windows and linux
    wd = os.getcwd()
    os.chdir(destination)
    log.info(f"Installing MathJax libraries in {destination}")
    try:
        subprocess.run(["npm", "install"]) #, f"--prefix={destination}"])
    except:
        log.warning("MathJax installation failed.  Is npm installed on your system?")
        log.setLevel(log_level)
    os.chdir(wd)

    log.info("Installing the Braille29 font")
    home = Path(os.path.expanduser('~'))
    source = prefig_root / 'resources' / 'fonts'
    shutil.copytree(
        source,
        home / '.fonts',
        dirs_exist_ok = True
    )

    try:
        subprocess.run(['fc-cache', '-f'])
        log.info("Successfully installed the Braille29 font")
    except:
        log.warning("Unable to install the Braille29 font")

    log.info("PreFigure initialization is complete")
    log.setLevel(log_level)


@main.command(
    help="Sets up a new PreFigure project"
)
def new():
    log_level = log.getEffectiveLevel()
    log.setLevel(logging.INFO)
    log.info("Setting up new PreFigure project")

    prefig_root = Path(__file__).parent
    source = prefig_root / 'resources' / 'diagcess'
    cwd = Path(os.getcwd())
    
    shutil.copytree(
        source,
        cwd,
        dirs_exist_ok = True
    )

    source = prefig_root / 'resources' / 'template'
    
    shutil.copytree(
        source,
        cwd,
        dirs_exist_ok = True
    )

    os.makedirs(cwd / 'source', exist_ok = True)
    log.setLevel(log_level)

    
@main.command(
    help="Installs PreFigure examples in current directory"
)
def examples():
    log_level = log.getEffectiveLevel()
    log.setLevel(logging.INFO)
    log.info("Installing PreFigure examples into current directory")

    prefig_root = Path(__file__).parent
    source = prefig_root / 'resources' / 'examples'
    cwd = Path(os.getcwd())
    
    shutil.copytree(
        source,
        cwd / 'examples',
        dirs_exist_ok = True
    )

    log.setLevel(log_level)


@main.command(
    help="Build a PreFigure diagram from source"
)
@click.option(
    "-f",
    "--format",
    default="svg",
    help="Desired output format, either 'svg' (default) or 'tactile'"
)
@click.option(
    "-p",
    "--publication",
    type=click.Path(),
    default=None,
    help="Location of publication file.  If no location is given, we'll look for a project 'pf_publication.xml' in a parent directory."
)
@click.option(
    '-i',
    "--ignore_publication",
    is_flag=True,
    default=False,
    help="Ignore any publication file"
)
@click.option(
    '-s',
    "--suppress-caption",
    is_flag=True,
    default=False,
    help="Suppress the creation of a diagram caption when creating tactile diagrams"
)
@click.argument(
    "filename",
    type=click.Path()
)
def build(format, publication, ignore_publication, suppress_caption, filename):
    return engine.build(format,
                        filename,
                        publication=publication,
                        ignore_publication=ignore_publication,
                        suppress_caption=suppress_caption,
                        environment="pf_cli")

@main.command(
    help="Convert the PreFigure SVG into a PDF"
)
@click.option(
    "-d",
    "--dpi",
    default=72,
    help="Desired resolution for the conversion.  It is essential that tactile diagrams have a resolution of 72."
)
@click.option(
    "-b",
    "--build_first",
    is_flag=True,
    default=False,
    help="Build from PreFigure source before converting to PDF"
)
@click.option(
    "-f",
    "--format",
    default="svg",
    help="Desired output format, if building from source.  Either 'svg' (default) or 'tactile'"
)
@click.option(
    "-p",
    "--publication",
    type=click.Path(),
    default=None,
    help="Location of publication file if building from source.  If no location is given, we'll look for a project 'pf_publication.xml' in a parent directory."
)
@click.option(
    '-i',
    "--ignore_publication",
    is_flag=True,
    default=False,
    help="If building from source, ignore any publication file"
)
@click.argument(
    "filename",
    type=click.Path()
)
@click.pass_context
def pdf(
        ctx,
        dpi,
        build_first,
        format,
        publication,
        ignore_publication,
        filename):

    engine.pdf(
        format,
        filename,
        dpi=dpi,
        build_first=build_first,
        publication=publication,
        ignore_publication=ignore_publication,
        environment="pf_cli"
    )

@main.command(
    help="Convert the PreFigure SVG into a PNG"
)
@click.option(
    "-b",
    "--build_first",
    is_flag=True,
    default=False,
    help="Build from PreFigure source before converting to PDF"
)
@click.option(
    "-f",
    "--format",
    default="svg",
    help="Desired output format, if building from source.  Either 'svg' (default) or 'tactile'"
)
@click.option(
    "-p",
    "--publication",
    type=click.Path(),
    default=None,
    help="Location of publication file if building from source.  If no location is given, we'll look for a project 'pf_publication.xml' in a parent directory."
)
@click.option(
    '-i',
    "--ignore_publication",
    is_flag=True,
    default=False,
    help="If building from source, ignore any publication file"
)
@click.argument(
    "filename",
    type=click.Path()
)
@click.pass_context
def png(
        ctx,
        build_first,
        format,
        publication,
        ignore_publication,
        filename):

    engine.png(
        format,
        filename,
        build_first=build_first,
        publication=publication,
        ignore_publication=ignore_publication,
        environment="pf_cli"
    )

@main.command(
    help="View a diagram and annotations in a web browser"
)

@click.option(
    '-i',
    "--ignore_annotations",
    is_flag=True,
    default=False,
    help="Ignore any annotations"
)

@click.option(
    '-r',
    '--restart_server',
    is_flag=True,
    default=False,
    help="Restart server, if it is running"
)
@click.option(
    '-p',
    '--port',
    default='8345',
    help="Use this port to start the server"
)
@click.argument(
    "filename",
    type=str
)
def view(filename, ignore_annotations, restart_server, port):
    # Let's look for the diagcess tools, possibly in a parent directory
    log_level = log.getEffectiveLevel()
    log.setLevel(logging.INFO)

    cwd = Path(os.getcwd())
    dirs = [cwd] + list(cwd.parents)
    diagcess_dir = None
    for dir in dirs:
        html_file = dir / 'diagcess.html'
        if html_file.exists():
            diagcess_dir = dir
            break

    # if we didn't find the diagcess tools, we'll install them here
    if diagcess_dir is None:
        prefig_root = Path(__file__).parent
        source = prefig_root / 'resources' / 'diagcess'
        cwd = Path(os.getcwd())
    
        shutil.copytree(
            source,
            cwd,
            dirs_exist_ok = True
        )
        diagcess_dir = cwd

    diagcess_file = diagcess_dir / 'diagcess.html'

    # Now we'll look for the output SVG file to view
    # If we're given an xml file, we'll modify the filename
    if filename.endswith('.xml'):
        filename = filename[:-4] + '.svg'

    if not filename.endswith('.svg'):
        filename += '.svg'

    view_path = None
    path = Path(filename)
    try:
        os.chdir(path.parent)
    except:
        log.warning(f"There is no directory {path.parent}")
        return

    for dir, dirs, files in os.walk(os.getcwd()):
        files = set(files)
        if path.name in files:
            view_path = path.parent / dir / path.name
    if view_path is None:
        log.warning(f'Unable to find {filename}')
        return

    # We are going to start the server from the home directory
    if os.environ.get('CODESPACES'):
        home_dir = os.environ.get('CODESPACE_VSCODE_FOLDER')
    else:
        home_dir = os.path.expanduser('~')

    active_port = find_active_server(port, restart_server)
    if active_port is None:
        subprocess.Popen(
            ['python3', '-m', 'http.server', port, '-d', home_dir]
        )
        active_port = port
        
    if os.environ.get('CODESPACES'):
        url_preamble = f"https://{os.environ.get('CODESPACE_NAME')}-{active_port}.{os.environ.get('GITHUB_CODESPACES_PORT_FORWARDING_DOMAIN')}"
    else:
        url_preamble = f"http://localhost:{active_port}"
    
    SECONDS = 2

    # Does this figure have annotations
    if ignore_annotations or not view_path.with_suffix('.xml').exists():
        # Don't worry about annotations so just open the SVG in a browser
        file_rel_path = os.path.relpath(view_path, home_dir)
        url = f'{url_preamble}/{file_rel_path}'
        log.info(f'Opening webpage in {SECONDS} seconds at {url}')
        time.sleep(SECONDS)
        webbrowser.open(url)
    else:
        # There are annotations so we'll open with diagcess
        file_rel_path = os.path.relpath(view_path, diagcess_dir)
        file_rel_path = file_rel_path[:-4]
        diagcess_rel_path = os.path.relpath(diagcess_file, home_dir)

        ## TODO:  what is diagcess_rel_path is empty?
        url = f'{url_preamble}/{diagcess_rel_path}?mole={file_rel_path}'
        SECONDS = 2
        log.info(f'Opening diagcess in {SECONDS} seconds at {url}')
        time.sleep(SECONDS)
        webbrowser.open(url)
    log.setLevel(log_level)

        
def find_active_server(port, restart):
    for proc in psutil.process_iter():
        if not proc.name().startswith('python'):
            continue
        try:
            connections = proc.connections()
            if len(connections) > 0:
                active_port = str(connections[0].laddr.port)
                if restart or active_port != port:
                    log.info('Restarting server')
                    proc.terminate()
                    return None
                return active_port
        except:
            pass
    log.warning("Unable to restart server")
    return None

@main.command(
    help="Validate a PreFigure XML source file against the schema"
)
@click.argument(
    "xml_file",
    type=click.Path()
)
def validate(xml_file):
    engine.validate_source(xml_file)


if __name__ == '__main__':
    main()
