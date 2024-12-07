import subprocess
import sys
import os
import logging
from pathlib import Path
import shutil

log = logging.getLogger('prefigure')

def main():
    prefig_root = Path(__file__).parent.parent
    destination = prefig_root / "core" / "mj_sre"

    shutil.copytree(
        prefig_root / "resources" / "js",
        destination,
        dirs_exist_ok = True
    )

    shutil.rmtree(destination / "node_modules", ignore_errors=True)

    # We need to change into mj_sre directory for installation
    # on Windows and linux
    wd = os.getcwd()
    os.chdir(destination)
    log.info(f"Installing MathJax libraries in {destination}")

    # we'll try to find the path to npm, which is needed for windows testing
    npm_cmd = shutil.which("npm")
    if npm_cmd is None:
        npm_cmd = "npm"

    try:
        subprocess.run([npm_cmd, "install"])
    except Exception as e:
        log.error("MathJax installation failed.  Is npm installed on your system?")
        # windows debug
        log.info(f"npm_cmd is at {npm_cmd}")
#        log.exception("Stack trace")
        return False
    
    os.chdir(wd)
    return True
        
if __name__ == "__main__":
    main()
