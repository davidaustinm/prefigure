import subprocess
import sys
import os
from pathlib import Path
import shutil


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
    print(f"Installing MathJax libraries in {destination}")
    try:
        subprocess.run(["npm", "install"]) #, f"--prefix={destination}"])
    except:
        print("MathJax installation failed.  Is npm installed on your system?")
        sys.exit()
    os.chdir(wd)

        
if __name__ == "__main__":
    main()
