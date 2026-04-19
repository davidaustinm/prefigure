#! /usr/bin/env node

/*************************************************************************
 *
 *  pretext
 *
 *  Uses MathJax v4 to convert all TeX in an HTML document to forms
 *  needed by PreTeXt
 *
 * ----------------------------------------------------------------------
 *
 *  Copyright (c) 2020 The MathJax Consortium
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

// Distributed to the PreTeXt project by Davide Cervone, Volker Sorge
// via https://gist.github.com/dpvc/386e8aac18c010361ef362b9237c71e9
// AIM braille textbook workshop, 2020-08

import { mathjax } from '@mathjax/src/mjs/mathjax.js';
import { TeX } from '@mathjax/src/mjs/input/tex.js';
import { MathML } from '@mathjax/src/mjs/input/mathml.js';
import { SVG } from '@mathjax/src/mjs/output/svg.js';
import { RegisterHTMLHandler } from '@mathjax/src/mjs/handlers/html.js';
import { liteAdaptor } from '@mathjax/src/mjs/adaptors/liteAdaptor.js';
import { SerializedMmlVisitor } from '@mathjax/src/mjs/core/MmlTree/SerializedMmlVisitor.js';
import { STATE, newState } from '@mathjax/src/mjs/core/MathItem.js';
import { EnrichHandler } from '@mathjax/src/mjs/a11y/semantic-enrich.js';
import { AllPackages } from './AllPackages.js';
import fs from 'fs';
import { createRequire } from 'module';
import yargs from 'yargs';
import { hideBin } from 'yargs/helpers';

// SRE's ESM build uses eval('require') to load fs, which fails in a native ESM
// context. Load the CJS build via createRequire so that `require` is available
// inside SRE and it can read locale data files from disk.
const _require = createRequire(import.meta.url);
const { setupEngine, engineReady, toSpeech, toEnriched } = _require('speech-rule-engine');

//
//  Get the command-line arguments
//
const argv = yargs(hideBin(process.argv))
    .demand(0).strict()
    .usage('$0 [options] infile.html > outfile.html')
    .options({
      speech: {
        boolean: true,
        default: false,
        describe: 'produce speech output'
      },
      braille: {
        boolean: true,
        default: false,
        describe: 'produce braille output'
      },
      svg: {
        boolean: true,
        default: false,
        describe: 'produce svg output'
      },
      svgenhanced: {
        boolean: true,
        default: false,
        describe: 'produce speech-enhanced svg output'
      },
      depth: {
        default: 'shallow',
        describe: 'the speech depth for SVG elements'
      },
      mathml: {
        boolean: true,
        default: false,
        describe: 'produce MathML output'
      },
      fontPaths: {
        boolean: true,
        default: false,
        describe: 'use svg paths not cached paths'
      },
      em: {
        default: 16,
        describe: 'em-size in pixels'
      },
      locale: {
        default: 'en',
        describe: 'the locale to use for speech output'
      },
      packages: {
        default: AllPackages.sort().join(', '),
        describe: 'the packages to use, e.g. "base, ams"'
      },
      rules: {
        default: 'mathspeak',
        describe: 'the rule set to use for speech output'
      }
    })
    .argv;

const needsSRE = argv.speech || argv.braille || argv.svgenhanced;

//
//  Read the HTML file
//
const htmlfile = fs.readFileSync(argv._[0], 'utf8');

//
//  Create DOM adaptor and register it for HTML documents
//
const adaptor = liteAdaptor({ fontSize: argv.em });
const handler = needsSRE
  ? EnrichHandler(RegisterHTMLHandler(adaptor), new MathML())
  : RegisterHTMLHandler(adaptor);

//
//  Create a MathML serializer
//
const visitor = new SerializedMmlVisitor();
const toMathML = (node => visitor.visitTree(node, html));

//
//  Create a renderAction that calls a function for each math item
//
function action(state, code, setup = null) {
  return [state, (doc) => {
    const adaptor = doc.adaptor;
    setup && setup();
    for (const math of doc.math) {
      try {
        code(math, doc, adaptor);
      } catch (err) {
        const id = adaptor.getAttribute(adaptor.parent(math.start.node), 'id');
        console.error('Error on item ' + id + ': ' + err.message);
      }
    }
  }];
}

//
//  States for PreTeXt actions
//
newState('PRETEXT', STATE.METRICS + 10);
newState('PRETEXTACTION', STATE.PRETEXT + 10);

//
//  The renderActions to use
//
const renderActions = {
  pretext: action(STATE.PRETEXT, (math, doc, adaptor) => {
    math.outputData.pretext = [adaptor.text('\n')];
    if (needsSRE) {
      math.outputData.mml = toMathML(math.root).toString();
    }
  }),
  typeset: action(STATE.TYPESET, (math, doc, adaptor) => {
    math.typesetRoot = adaptor.node('mjx-data', {}, math.outputData.pretext);
  })
};

//
//  If SVG is requested, add an action to add it to the output
//
if (argv.svg) {
  renderActions.svg = action(STATE.PRETEXTACTION, (math, doc, adaptor) => {
    math.outputData.pretext.push(adaptor.firstChild(doc.outputJax.typeset(math, doc)));
    math.outputData.pretext.push(adaptor.text('\n'));
  });
}

//
//  MathML-input SVG document used for svgenhanced
//
const mmldoc = mathjax.document('', {
  InputJax: new MathML(),
  OutputJax: new SVG({ fontCache: (argv.fontPaths ? 'none' : 'local') }),
});

//
//  If svgenhanced is requested, produce SRE-enriched SVG output
//
if (argv.svgenhanced) {
  renderActions.svg = action(STATE.PRETEXTACTION, (math, doc, adaptor) => {
    const out = mmldoc.convert(toEnriched(math.outputData.mml).toString());
    math.outputData.pretext.push(out);
    math.outputData.pretext.push(adaptor.text('\n'));
  }, () => {
    setupEngine({ speech: argv.depth, modality: 'speech', locale: argv.locale, domain: argv.rules });
  });
}

//
//  If MathML is requested, add an action to add it to the output
//
if (argv.mathml) {
  renderActions.mathml = action(STATE.PRETEXTACTION, (math, doc, adaptor) => {
    const mml = adaptor.firstChild(adaptor.body(adaptor.parse(toMathML(math.root), 'text/html')));
    math.outputData.pretext.push(mml);
    math.outputData.pretext.push(adaptor.text('\n'));
  });
}

//
//  If speech is requested, add an action to add it to the output
//
if (argv.speech) {
  renderActions.speech = action(STATE.PRETEXTACTION, (math, doc, adaptor) => {
    const speech = toSpeech(math.outputData.mml);
    math.outputData.pretext.push(adaptor.node('mjx-speech', {}, [adaptor.text(speech)]));
    math.outputData.pretext.push(adaptor.text('\n'));
  }, () => {
    setupEngine({ modality: 'speech', locale: argv.locale, domain: argv.rules });
  });
}

//
//  If braille is requested, add an action to add it to the output
//
if (argv.braille) {
  renderActions.braille = action(STATE.PRETEXTACTION, (math, doc, adaptor) => {
    const speech = toSpeech(math.outputData.mml);
    math.outputData.pretext.push(adaptor.node('mjx-braille', {}, [adaptor.text(speech)]));
    math.outputData.pretext.push(adaptor.text('\n'));
  }, () => {
    setupEngine({ modality: 'braille', locale: 'nemeth', markup: 'layout', domain: 'default' });
  });
}

//
//  Create an HTML document using the html file and a new TeX input jax
//
const html = mathjax.document(htmlfile, {
  renderActions,
  InputJax: new TeX({ packages: argv.packages.split(/\s*,\s*/) }),
  OutputJax: new SVG({ fontCache: (argv.fontPaths ? 'none' : 'local') }),
});

//
//  Don't add the stylesheet unless SVG output is requested
//
if (!(argv.svg || argv.svgenhanced)) {
  html.addStyleSheet = () => {};
}

(async () => {
  if (needsSRE) {
    await engineReady();
    if (argv.braille) {
      await setupEngine({ locale: 'nemeth', modality: 'braille', markup: 'layout', domain: 'default' });
    }
    if (argv.speech || argv.svgenhanced) {
      await setupEngine({ locale: argv.locale, modality: 'speech', domain: argv.rules });
    }
  }
  await mathjax.handleRetriesFor(() => html.renderPromise());
  console.log(adaptor.outerHTML(adaptor.root(html.document)));
})().catch((err) => console.error(err));
