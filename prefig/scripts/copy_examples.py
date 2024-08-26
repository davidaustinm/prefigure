import os.path
import shutil
import subprocess
from pathlib import Path

def main():
    cwd = Path(os.getcwd())
    prefig_root = Path(__file__).parent.parent
    source = prefig_root / 'resources' / 'examples'
    print("Copying examples into current directory")
    
    shutil.copytree(
        source,
        cwd / 'examples',
        dirs_exist_ok = True
    )

if __name__ == '__main__':
    main()

    
    

