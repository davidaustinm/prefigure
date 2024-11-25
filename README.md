# PreFigure

PreFigure is a Python package for authoring mathematical diagrams.  Following the [PreTeXt](https://pretextbook.org/) paradigm, an author writes an XML description of a diagram that PreFigure converts into an image file suitable for including in a text document.  By default, PreFigure will create an SVG image that can be included in, say, an HTML document. However, PreFigure prioritizes the creation of accessible diagrams so that annotations can be added that enable a screen reader to easily navigate the diagram.  Tactile diagrams can also be created from the same XML source.

PreFigure diagrams can now be authored inside a PreTeXt document.  More information, including detailed documentation, is available from the [PreFigure homepage](https://prefigure.org).

## Using PreFigure

You may author and compile PreFigure diagrams in either of two environments:

1. PreFigure is available in a [GitHub Codespace](https://github.com/davidaustinm/prefigure-codespace).  This is a free, cloud-based platform that takes care of all the installation details and creates a fully configured working environment.  Follow the instructions on that page to create your codespace and then get started authoring diagrams.
2. PreFigure may be installed locally as a Python package following the instructions in the **Local Installation** section below.

## Local Installation

PreFigure may be installed locally as a Python package in the usual way using `pip`.  However, there are a few additional details that require your attention.

1. PreFigure assumes Python version 3.8.5 or higher.  You may check your local Python version with one of the two commands below

    ```
    python -V
    ```

    ```
    python3 -V
    ```

2. You are encouraged to install `liblouis`, which enables the creation of non-mathematical braille labels in tactile diagrams.  PreFigure can still create diagrams without this package installed though you will see a non-fatal warning message when you compile a tactile diagram and any requested labels will not appear in the diagram.

    On a linux machine, use your package manager to install `python3-louis`.  Ubuntu users can use

    ```
    apt get python3-louis
    ```

    while on a Mac, you will want

    ```
    brew install liblouis
    ```

    Alternatively, you can install `liblouis` following [these instructions](https://liblouis.io/downloads/). 

    Within a Python interpreter, you should then be able to `import louis` without an error.

3. You are encouraged to install an [additional library](https://pycairo.readthedocs.io/en/latest/getting_started.html) to support the `pycairo` package.  This may not be essential for your local machine, but there is no harm in performing this step.  The `pycairo` package is needed to produce labels having plain text (rather than mathematics).  If you are not able to install `pycairo`, you will still be able to build PreFigure diagrams, but any labels with plain text will not appear.

4. You are now ready to install PreFigure with

    ```
    pip install prefig[pycairo]
    ```

    If this fails, it is due to the `pycairo` dependency so you can instead install PreFigure without `pycairo` using

    ```
    pip install prefig
    ```

5. You will need a local installation of `node` and `npm` to produce mathematical labels.  (The `node` installation includes `npm`.)  This is a simple process, but you should search to find the instructions for your operating system.  On a Ubuntu machine, it's as easy as 

    ```
    apt install nodejs
    ```

6. For creating tactile graphics to be embossed, you are encouraged to install `rsvg-convert`, which PreFigure uses to convert SVGs into PDFs.  On Ubuntu, you can say

    ```
    apt install librsvg2-bin
    ```

    while Mac users can use

    ```
    brew install librsvg
    ```

7. Once installed, the command `prefig init` will install MathJax and the Braille29 font needed to tactile diagrams.  If you do not perform this step, MathJax will be automatically installed when you first build a diagram with mathematical labels.

## Usage

Once PreFigure is installed, help is available with

```
prefig --help
```

or, say,

```
prefig build --help
```

Details of a requested operation may be obtained using the `-v` and `-vv` flags.  For instance,
```
prefig -vv build foo.xml
```
will print debugging information to the terminal.

Here is a summary of PreFigure commands.
```

1. PreFigure source files can be compiled into SVG images using one of the following two commands, with the first command creating a regular SVG file while the second produces a tactile version of    the diagram.

    ```
    prefig build foo.xml
    ```

    ```
    prefig build -f tactile foo.xml
    ```

    By default, the output appears in `output/foo.svg` and `output/foo.xml`, where the XML output   contains the annotations used by a screen reader.  If PreFigure is called from within a PreTeXt document, then the annotations will appear in `foo-annotations.xml`.

2. To view the resulting diagram, use either

    ```
    prefig view foo
    ```

    ```
    prefig view -i foo
    ```

    The first command will open the diagram in a browser using the `diagcess` library, which enables a reader to explore the annotations interactively.  The second command ignores the annotations and simply opens the SVG diagram in a browser.

3. Once a diagram has been compiled, you may create a PDF using

    ```
    prefig pdf foo
    ```

    Adding the `-b` switch will build the diagram from PreFigure source before the PDF is formed.

4. Similarly,

    ```
    prefig png foo
    ```

    creates a PNG.  Add the `-b` switch to `build` the diagram first.

5. To validate PreFigure source against the PreFigure XML schema, use

    ```
    prefig validate foo.xml
    ```



You may wish to perform the following steps to set up your authoring environment (these are automatically performed in a codespace):

1. To initialize your local installation, use

    ```
    prefig init
    ```

    which will use `npm` to install some MathJax modules.  It will also install the Braille29 font needed for tactile diagrams.  If the MathJax modules are not installed when you attempt to build a diagram, PreFigure will attempt to install them when you build your first diagram.

2. You may install a set of examples for exploration in the current directory using

    ```
    prefig examples
    ```

3. You may initialize a new PreFigure project in the current directory using


    ```
    prefig new
    ```
 
    This copies the `diagcess` tools and a default publication file into the current directory and creates a `source` directory in which to author diagrams.

## Acknowledgements

[Volker Sorge](https://www.birmingham.ac.uk/staff/profiles/computer-science/academic-staff/sorge-volker) has provided crucial support for this project as well as access to the diagcess library for navigating an image with a screen reader.

The MathJax module `mj-sre-page.js` included with this distribution was created by [Davide Cervone](https://www.math.union.edu/~dpvc/) and Volker Sorge.

Thanks also to the PreTeXt community, and especially [Rob Beezer](http://buzzard.ups.edu/), for support and inspiration.  This project was developed with support from the [UTMOST Project](https://utmost.aimath.org/)

## License

PreFigure is distributed with a GPL license.
