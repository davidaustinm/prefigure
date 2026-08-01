// A compact hero strip for the README: three PreFigure diagrams in a row, each
// built by the same `prefigure(read(...))` call. Geometry is drawn by the wasm
// plugin; the text and math are rendered by Typst in this document's font.
//
//   typst compile --root <repo-root> -f png examples/showcase.typ showcase.png
#import "../src/lib.typ": prefigure

#set page(width: auto, height: auto, margin: 10pt, fill: white)
#set text(font: "New Computer Modern", size: 11pt)

#grid(
  columns: 3,
  column-gutter: 14pt,
  align: bottom,
  prefigure(read("figures/roots_of_unity.xml"), width: 6cm),
  prefigure(read("figures/diffeqs.xml"), width: 6cm),
  prefigure(read("figures/implicit.xml"), width: 6cm),
)
