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

    # windows debug
    log.info(os.listdir())
    log.info(f"npm is at {shutil.which("npm")}")
    log.info(f"node is at {shutil.which("node")}")
        
        
    try:
        subprocess.run(["npm", "install"]) #, f"--prefix={destination}"])
    except subprocess.CalledProcessError as e:
        log.error("MathJax installation failed.  Is npm installed on your system?")
        log.error("Error:", e)
        log.error("Return code:", e.returncode)
        log.error("Standard output:", e.stdout)
        log.error("Standard error:", e.stderr)
        return False
    
    os.chdir(wd)
    return True
        
if __name__ == "__main__":
    main()
