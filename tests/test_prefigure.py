import subprocess
import os

def test_prefigure():
    # copy examples into the current directory
    try:
        subprocess.run([
            "prefig",
            "-vv",
            "examples"
        ])
    except:
        subprocess.run([
            "poetry",
            "run",
            "prefig",
            "-vv",
            "examples"
        ])

    # build one of the diagrams and make sure it wrote the output
    result = subprocess.run([
        "prefig",
        "-vv",
        "build",
        "examples/de-system.xml"
    ])
    if result.returncode != 0:
        result = subprocess.run([
            "poetry",
            "run",
            "prefig",
            "-vv",
            "build",
            "examples/de-system.xml"
        ])
        
    assert result.returncode == 0
    assert os.path.exists("examples/output/de-system.svg")
    
