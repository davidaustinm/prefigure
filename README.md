# PreFigure

PreFigure is a Python package for authoring mathematical diagrams.  Following the [PreTeXt](https://pretextbook.org/) paradigm, an author creates an XML description of a diagram that PreFigure will convert into a specified output format suitable for including in a textual document.  By default, PreFigure will create an SVG image that can be included in, say, an HTML document. However, PreFigure prioritizes the creation of accessible diagrams so that annotations can be added that enable a screen reader to easily navigate the diagram using the diagcess library.  Tactile diagrams can also be created from the same XML source.

Sample XML descriptions are included in the `samples` directory.

## Usage

With the PreFigure module `parse.py` in the Python path and `foo.xml` an XML diagram description, image files are created using:

```
python parse.py foo.xml
python parse.py -f tactile foo.xml
```

By default, the output appears in `output/foo.svg` and `output/foo.xml`, where the XML output contains the annotations used by a screen reader.

Ordinary SVG images may be examined in a standard web browser.  To explore an image using author-prescribed annotations, you will need to navigate to the `js` directory of this distribution and start a web server (or alternatively, copy `diagcess.js` and `generic.html` to a more convenient location):

```
python -m http.server
```

In a web browser, the image can be examined at URL:

```
http://127.0.0.1:8000/generic.html?mole=path_from_generic_to_output_foo/foo
```

## Installation

PreFigure relies on [MathJax](https://www.mathjax.org/) to create mathematical labels so users need to install [MathJax-demos-node](https://github.com/mathjax/MathJax-demos-node).  

**Temporary:** The inclusion of MathJax labels relies on on the node module `mj-sre-page.js` contained in PreFigure's `js` directory.  You will also need to include a Python module
`mj_cmd.py` in the `prefigure` directory of this distribution with a single line that gives the path to `mj-sre-page.js`.  For example,

```
mathjax_path = '/home/david/prefigure/js/
```

## Acknowledgements

[Volker Sorge](https://www.birmingham.ac.uk/staff/profiles/computer-science/academic-staff/sorge-volker) has provided crucial support for this project as well as access to the diagcess library for navigating an image with a screen reader.  

The MathJax module `mj-sre-page.js` was created by [Davide Cervone](https://www.math.union.edu/~dpvc/) and Volker.

Thanks also to the PreTeXt community, and especially [Rob Beezer](http://buzzard.ups.edu/), for support and inspiration.  This project was developed with support from the [UTMOST Project](https://utmost.aimath.org/)

## License

PreFigure is distributed with a GPL license.
