import abc
import lxml.etree as ET
import tempfile
import logging
import inspect
import os
from pathlib import Path
from pathlib import Path

log = logging.getLogger("prefigure")
ns = {'svg': 'http://www.w3.org/2000/svg'}

class AbstractMathLabels(abc.ABC):
    @abc.abstractmethod
    def add_macros(self, macros):
        pass

    @abc.abstractmethod
    def register_math_label(self, id, text):
        pass

    @abc.abstractmethod
    def process_math_labels(self):
        pass

    @abc.abstractmethod
    def get_math_label(self, id):
        pass


class AbstractTextMeasurements(abc.ABC):
    @abc.abstractmethod
    def measure_text(self, text, font_data):
        pass

class AbstractBrailleTranslator(abc.ABC):
    @abc.abstractmethod
    def initialized(self):
        pass
    
    @abc.abstractmethod
    def translate(self, text, typeform):
        pass

class LocalMathLabels(AbstractMathLabels):
    def __init__(self, format):
        self.format = format

        html = ET.Element('html')
        body = ET.SubElement(html, 'body')
        self.html_tree = html
        self.html_body = body
        self.labels_present = False

    def add_macros(self, macros):
        macros_div = ET.SubElement(self.html_body, 'div')
        macros_div.set('id', 'latex-macros')
        macros_div.text = '\({}\)'.format(macros)

    def register_math_label(self, id, text):
        div = ET.SubElement(self.html_body, 'div')
        div.set('id', id)
        div.text = text
        self.labels_present = True

    def process_math_labels(self):
        if not self.labels_present:
            return
        # prepare the MathJax command
        input_filename = "prefigure-labels.html"
        output_filename = f"prefigure-{self.format}.html"
        working_dir = tempfile.TemporaryDirectory()
        mj_input = os.path.join(working_dir.name, input_filename)
        mj_output = os.path.join(working_dir.name, output_filename)

        # write the HTML file
        with ET.xmlfile(mj_input, encoding='utf-8') as xf:
            xf.write(self.html_tree, pretty_print=True)

        options = ''
        if self.format == 'tactile':
            format = 'braille'
        else:
            options = '--svgenhanced --depth deep'
            format = 'svg'

        # have MathJax process the HTML file and load the resulting
        # SVG labels into label_tree 
        path = Path(os.path.dirname(os.path.abspath(inspect.getfile(inspect.currentframe()))))
        mj_dir = path.absolute() / 'mj_sre'
        mj_dir_str = str(mj_dir)

        if not (mj_dir / 'mj-sre-page.js').exists():
            log.info("MathJax installation not found so we will install it")
            from .. import scripts
            success = scripts.install_mj.main()
            if not success:
                log.error("Cannot create labels without MathJax")
                return

        mj_command = 'node {}/mj-sre-page.js --{} {} {} > {}'.format(mj_dir_str, format, options, mj_input, mj_output)
        log.debug("Using MathJax to produce mathematical labels")
        try:
            os.system(mj_command)
        except:
            log.error("Production of mathematical labels with MathJax was unsuccessful")
            return
        self.label_tree = ET.parse(mj_output)
        working_dir.cleanup()

    def get_math_label(self, id):
        # first we'll retrieve braille math labels
        if self.format == "tactile":
            try:
                path = "//html/body/div[@id = '{}']".format(id)
                div = self.label_tree.xpath(path)[0]
            except:
                log.error("Error retrieving a mathematical label")
                log.error("  Perhaps it was not created due to an earlier error")
                return None

            try:
                return div.xpath('mjx-data/mjx-braille')[0]
            except IndexError:
                log.error(f"Error in processing label, possibly a LaTeX error: {div.text}")                
                return None

        # now we get sighted math labels
        path = "//html/body/div[@id = '{}']".format(id)
        div = self.label_tree.xpath(path)[0]
        try:
            insert = div.xpath('mjx-data/mjx-container/svg:svg',
                               namespaces=ns)[0]
            return insert
        except IndexError:
            log.error(f"Error in processing label, possibly a LaTeX error: {div.text}")
            return None
        

class CairoTextMeasurements(AbstractTextMeasurements):
    def __init__(self):
        self.cairo_loaded = False
        global cairo
        try:
            import cairo
        except:
            log.info('Error importing Python package cairo, which is required for non-mathemaical labels.')
            log.info('See the PreFigure installation instructions at https://prefigure.org')
            log.info('The rest of the diagram will still be built')
            return

        log.info("cairo imported")
        self.cairo_loaded = True
        surface = cairo.SVGSurface(None, 200, 200)
        self.context = cairo.Context(surface)
        self.italics_dict = {True: cairo.FontSlant.ITALIC,
                             False: cairo.FontSlant.NORMAL}
        self.bold_dict = {True: cairo.FontWeight.BOLD,
                          False: cairo.FontWeight.NORMAL}

    def measure_text(self, text, font_data):
        font, font_size, italics, bold = font_data[:4]
        
        if not self.cairo_loaded:
            return None

        self.context.select_font_face(font,
                                      self.italics_dict[italics],
                                      self.bold_dict[bold])
        self.context.set_font_size(font_size)
        extents = self.context.text_extents(text)
        y_bearing = extents[1]
        t_height  = extents[3]
        xadvance  = extents[4]
        return [xadvance, -y_bearing, t_height+y_bearing]                       

class LocalLouisBrailleTranslator(AbstractBrailleTranslator):
    def __init__(self):
        self.louis_loaded = False

        try:
            global louis
            import louis
        except:
            log.info('Failed to import louis so we cannot make braille labels')
            log.info('See the installation instructions at https://prefigure.org')
            log.info('The rest of the diagram will still be built.')
            return

        log.info("louis imported")
        self.louis_loaded = True

    def initialized(self):
        return self.louis_loaded

    def translate(self, text, typeform):
        if not self.louis_loaded:
            return None
        return louis.translateString(
            ["braille-patterns.cti", "en-us-g2.ctb"],
            text,
            typeform=typeform
        )
        
        
class PyodideTextMeasurements(AbstractTextMeasurements):
    def __init__(self):
        self.js_loaded = False
        global js
        try:
            import js
            log.info("js imported")
            self.js_loaded = True
        except:
            from .compat import ErrorOnAccess
            js = ErrorOnAccess("js")

        js_code = '''
        function js_measure_text(text) {
        const canvas = document.createElement("canvas");
        ctx = canvas.getContext("2d");
        ctx.font = "14px sans";
        const tm = ctx.measureText(text);
        return [tm.width, tm.actualBoundingBoxAscent,tm.actualBoundingBoxDescent];
        }
        '''
        try:
            js.eval(js_code)
            log.info("js_code evaluated")
        except:
            log.error("js_code evaluated")

    def measure_text(self, text, font_data):
        import json
        text = json.dumps(text)
        log.info('Called measure text')
        try:
            text_dims = js.eval(f"js_measure_text({text})").to_py()
            log.info(f"text_dims found: {text_dims}")
            log.error(f"type = {type(text_dims)}")
        except Exception as e:
            log.error(str(e))
            log.error("text_dims not found")
        return text_dims
