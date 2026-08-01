// A larger gallery: a variety of PreFigure diagrams rendered from Typst through
// the wasm plugin — level curves, a slope field, a phase/time plot with a
// legend, a Riemann sum, higher derivatives, vector projection, roots of unity,
// and a tangent line. Every figure is built by the same `prefigure(read(...))`
// call; Typst measures the text and PreFigure draws the geometry, while by
// default Typst also renders the text and the math (via mitex) in the document
// font — so the labels below match this document's type.
//
// Compile with --root at the repo root so the relative read() paths resolve:
//   typst compile --root <repo-root> examples/large-demo.typ large-demo.pdf

#import "../src/lib.typ": prefigure

#set page(paper: "a4", margin: 2cm, numbering: "1")
#set text(font: "New Computer Modern", size: 10.5pt)
#set par(justify: true)
#show heading.where(level: 1): set text(size: 16pt)
#show heading.where(level: 2): set block(above: 1.4em, below: 0.7em)

#align(center)[
  #text(size: 22pt, weight: "bold")[PreFigure in Typst — a gallery]
  #v(0.3em)
  #text(
    fill: luma(90),
  )[Diagrams authored as PreFigure XML; geometry by the wasm plugin, text and math by Typst]
]

#v(1em)

// A figure with a caption, centered.
#let fig(path, caption, width: 11cm) = figure(
  prefigure(read(path), width: width),
  caption: caption,
)

= Multivariable calculus

== Level curves
The level curves $y^2 - x^3 + x = k$ of a function of two variables, one curve
per value of $k$. Each contour is drawn by an `<implicit-curve>`; the fractional
label $2 slash (3 sqrt(3))$ is the figure's own LaTeX, rendered by Typst via mitex.

#fig(
  "figures/implicit.xml",
  [Level curves of $f(x,y) = y^2 - x^3 + x$.],
  width: 9cm,
)

== Vector projection
The projection of $bold(v)$ onto a line $L$, decomposed into $hat(bold(b))$ and
its orthogonal complement $bold(b)^perp$.

#fig(
  "figures/projection.xml",
  [Projecting one vector onto another.],
  width: 9cm,
)

= Differential equations

== Slope field
A first-order slope field with several solution curves threaded through it.

#fig("figures/diffeqs.xml", [A slope field and solutions.], width: 9.5cm)

== A system, over time
A solution of a differential-equation system plotted against time, with a
legend distinguishing $x(t)$ from $x'(t)$. The legend exercises the label
measurement path end-to-end: its box is sized from Typst's own text metrics.

#fig(
  "figures/de-system.xml",
  [A component plot with a measured legend.],
  width: 10cm,
)

= Single-variable calculus

#grid(
  columns: (1fr, 1fr),
  column-gutter: 1em,
  fig("figures/riemann.xml", [A Riemann sum, $A_1 dots A_4$.], width: 7.5cm),
  fig(
    "figures/derivatives.xml",
    [A function with $f'$ and $f''$.],
    width: 7.5cm,
  ),
)

#show math.equation: set text(font: "Fira Math")
== Tangent line
#fig(
  "figures/tangent.xml",
  [The tangent line to a graph at $a = 1$.],
  width: 9cm,
)

= Complex numbers

== Roots of unity
The eighth roots of unity on the unit circle, each labelled with a power of
$omega$ — every label is math, typeset by Typst in the document font.

#fig("figures/roots_of_unity.xml", [The eighth roots of unity.], width: 9cm)
