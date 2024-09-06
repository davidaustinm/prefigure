import subprocess
import sys
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

    print("Installing MathJax libraries with npm")
    try:
        subprocess.run(["npm", "install", f"--prefix={destination}"])
    except:
        print("MathJax installation failed.  Is npm installed on your system?")
        sys.exit()

        
if __name__ == "__main__":
    main()
