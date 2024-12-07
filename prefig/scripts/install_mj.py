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

    npm_cmd = shutil.which("npm")

    try:
        subprocess.run([npm_cmd, "install"])
    except Exception as e:
        log.error("MathJax installation failed.  Is npm installed on your system?")
        # windows debug
        log.info(os.listdir())
        log.info(f"npm is at {shutil.which("npm")}")
        log.info(f"node is at {shutil.which("node")}")
        
        
        log.error(f"package.json exists = {os.path.exists('package.json')}")
        log.error(f"current directory = {os.getcwd()}")
        log.error(f"Listing again: {os.listdir()}")
        log.exception("Stack trace")
        return False
    
    os.chdir(wd)
    return True
        
if __name__ == "__main__":
    main()
