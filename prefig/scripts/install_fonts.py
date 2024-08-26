import os.path
import shutil
import subprocess
from pathlib import Path

def main():
    home = Path(os.path.expanduser('~'))
    prefig_root = Path(__file__).parent.parent
    source = prefig_root / 'resources' / 'fonts'
    shutil.copytree(
        source,
        home / '.fonts',
        dirs_exist_ok = True
    )

    print("Installing the Braille29 font")
    try:
        subprocess.run(['fc-cache', '-f'])
        print("Successfully installed the Braille29 font")
    except:
        print("Unable to install the Braille29 font")

if __name__ == '__main__':
    main()
    

