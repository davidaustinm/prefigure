// Demo: render bundled PreFigure examples from Typst.
//   typst compile --root <repo-root> examples/demo.typ examples/demo.pdf
#import "../src/lib.typ": prefigure

#set page(width: 16cm, height: auto, margin: 1.5cm)
#set text(font: "New Computer Modern", size: 11pt)

= PreFigure in Typst

A tangent line, with a math label and math-valued axis labels:

#align(center, prefigure(read("figures/tangent.xml"), width: 9cm))

Roots of unity:

#align(center, prefigure(read("figures/roots_of_unity.xml"), width: 9cm))
