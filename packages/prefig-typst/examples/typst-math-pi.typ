// Generated math, rendered by Typst via mitex.
//
// typst-math.typ shows *authored* math ($…$ → xmlit sentinels) rendered by
// Typst. This example shows the other half: math PreFigure generates *itself*.
// A pi-format x-axis (`h-pi-format="yes"`) makes PreFigure emit LaTeX tick
// labels — `\pi`, `-\frac{\pi}{2}`, `\frac{\pi}{2}` — that never came from the
// author. prefigure() enumerates them and converts them with mitex, so they
// render in the document's math font (New Computer Modern here) exactly like the
// authored `$sin x$` label.
//
//   typst compile --root <repo-root> examples/typst-math-pi.typ out.pdf

#import "../src/lib.typ": prefigure, tags

#set page(width: 6in, height: auto, margin: 1cm)

#let authored = {
  import tags: *
  diagram(dimensions: (340, 210), margins: 5, coordinates(
    bbox: (-3.15, -1.5, 3.15, 1.5),
    grid-axes(..(xlabel: "x", ylabel: "y", "h-pi-format": true)),
    graph(function: "f(x) = sin(x)"),
    label(p: (1.6, calc.sin(1.6)), alignment: "north")[$sin x$],
  ))
}

#align(center, prefigure(authored, width: 9cm))

The $pi$-format tick labels ($-pi$, $-pi/2$, $pi/2$, $pi$) are Typst math in the document font —
PreFigure generated them as LaTeX and mitex converted them — sitting at the PreFigure-computed
positions, right alongside the authored $sin x$ label.
