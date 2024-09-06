# Adapted from the PreTeXt CLI 

from pathlib import Path
from urllib.request import urlopen
import json
from datetime import datetime
import fileinput


def main() -> None:
    # Get the date from the most recent commit
    url = f"https://api.github.com/repos/davidaustinm/prefigure/commits"
    response = urlopen(url)
    data = json.loads(response.read())
    lastcommit = datetime.strptime(
        data[0]["commit"]["committer"]["date"], "%Y-%m-%dT%H:%M:%SZ"
    )

    # If there's not a new commit, there's nothing to do
    if (datetime.now() - lastcommit).days > 1:
        print("No recent commit:", lastcommit)
        return

    # Update version in runner's pyproject.toml:
    for line in fileinput.input(
        Path(__file__).parent.parent / "pyproject.toml", inplace=True
    ):
        if line.startswith("version"):
            version = str(line.split('"')[1])
            newversion = version + ".dev" + datetime.now().strftime("%Y%m%d%H%M%S")
            print(line.replace(line, f'version = "{newversion}"'.rstrip()))
        else:
            print(line.rstrip())

    print("OK to deploy")


if __name__ == "__main__":
    main()
