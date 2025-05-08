import { mathjax } from "mathjax-full/js/mathjax";
import { TeX } from "mathjax-full/js/input/tex";
import { liteAdaptor } from "mathjax-full/js/adaptors/liteAdaptor";
import { STATE } from "mathjax-full/js/core/MathItem";
import { HTMLDocument } from "mathjax-full/js/handlers/html/HTMLDocument";
import { RegisterHTMLHandler } from "mathjax-full/js/handlers/html";
import { SerializedMmlVisitor } from "mathjax-full/js/core/MmlTree/SerializedMmlVisitor";
import { MmlNode } from "mathjax-full/js/core/MmlTree/MmlNode";
import { Sre } from "mathjax-full/js/a11y/sre";
import { toSpeech } from "speech-rule-engine/js/common/system";

console.log("MathJax Braille worker loaded", mathjax, Sre);

// Roughly following https://github.com/mathjax/MathJax-demos-node/blob/master/direct/tex2mml
const tex = new TeX();

const html = new HTMLDocument("", liteAdaptor(), { InputJax: tex });

const visitor = new SerializedMmlVisitor();
const toMathML = (node: MmlNode) => visitor.visitTree(node);

const mml = toMathML(
    html.convert("x^2", { display: false, end: STATE.CONVERT }),
);

console.log(mml);

const speech = toSpeech(mml);
console.log("speech", speech);

{
    const tex = new TeX();
    const adaptor = liteAdaptor();
    RegisterHTMLHandler(adaptor);

    const mj = mathjax.document("", {
        InputJax: tex,
    });

    const mathNode = mj.convert("x^2", { display: true });

    //const html = new HTMLDocument("", liteAdaptor(), { InputJax: tex });

    const visitor = new SerializedMmlVisitor();
    const mml = visitor.visitTree(mathNode);
    console.log("other version", mml);
}
