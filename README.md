# PreFigure

PreFigure is a Python package for authoring mathematical diagrams.  Following the [PreTeXt](https://pretextbook.org/) paradigm, an author creates an XML description of a diagram that PreFigure will convert into a specified output format suitable for including in a textual document.  By default, PreFigure will create an SVG image that can be included in, say, an HTML document. However, PreFigure prioritizes the creation of accessible diagrams so that annotations can be added that enable a screen reader to easily navigate the diagram using the diagcess library.  Tactile diagrams can also be created from the same XML source.

Example XML diagram descriptions are included in the `resources/examples` directory.

## Installation

1. Clone this repository.
2. Install the packages in `requirements.txt`.  If the installation of `pycairo` fails, you probably need to install an [additional library](https://pycairo.readthedocs.io/en/latest/getting_started.html).
3. You will also need to install `liblouis` following [these instructions](https://liblouis.io/downloads/).  
4. PreFigure relies on [MathJax](https://www.mathjax.org/) to create mathematical labels so users need to navigate to the `js` directory of this distribution and execute the bash script `update-sre`.

## Usage

With the PreFigure module `parse.py` in the Python path and `foo.xml` an XML diagram description, image files are created using:

```
prefig foo.xml
prefig -f tactile foo.xml
```
By default, the output appears in `output/foo.svg` and `output/foo.xml`, where the XML output contains the annotations used by a screen reader.

Ordinary SVG images may be examined in a standard web browser.  To explore an image using author-prescribed annotations, you will need to navigate to the `resources/diagcess` directory of this distribution and start a web server (or alternatively, copy `diagcess.js` and `generic.html` from the `diagcess` directory to a more convenient location):

```
python -m http.server
```
In a web browser, the image can be examined at the URL:

```
http://127.0.0.1:8000/generic.html?mole=path_from_generic_to_output_foo/foo
```
## Acknowledgements

[Volker Sorge](https://www.birmingham.ac.uk/staff/profiles/computer-science/academic-staff/sorge-volker) has provided crucial support for this project as well as access to the diagcess library for navigating an image with a screen reader.

The MathJax module `mj-sre-page.js` included with this distribution was created by [Davide Cervone](https://www.math.union.edu/~dpvc/) and Volker Sorge.

Thanks also to the PreTeXt community, and especially [Rob Beezer](http://buzzard.ups.edu/), for support and inspiration.  This project was developed with support from the [UTMOST Project](https://utmost.aimath.org/)

## License

PreFigure is distributed with a GPL license.
