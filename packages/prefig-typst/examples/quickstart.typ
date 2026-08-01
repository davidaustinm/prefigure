// The smallest useful program: import the package and render a diagram. The
// math axis labels and the tangent-line label are typeset by Typst in the
// document font, on top of geometry drawn by the plugin.
//
//   typst compile --root <repo-root> -f png examples/quickstart.typ quickstart.png
#import "../src/lib.typ": prefigure

#set page(width: auto, height: auto, margin: 10pt, fill: white)
#set text(font: "New Computer Modern", size: 11pt)

#prefigure(read("figures/tangent.xml"), width: 8cm)
