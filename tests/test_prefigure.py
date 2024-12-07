import subprocess
import os

def test_prefigure():
    # copy examples into the current directory
    subprocess.run([
        "python",
        "-m",
        "poetry",
        "run",
        "prefig",
        "examples"
    ])

    # build one of the diagrams and make sure it wrote the output
    result = subprocess.run([
        "python",
        "-m",
        "poetry",
        "run",
        "prefig",
        "-vv",
        "build",
        "examples/de-system.xml"
    ])
    assert result.returncode == 0
    assert os.path.exists("examples/output/de-system.svg")
    
