// The two label modes, side by side, on one math-free diagram:
//   labels: "svg"     text baked into the SVG (self-contained, resvg-rendered)
//   labels: "native"  live Typst text overlaid at the same measured positions
// The baselines coincide exactly; only the native text is selectable and follows
// `#set text`. (A diagram with any math is always drawn native, so this choice
// only matters when there is no math.)
//
//   typst compile --root <repo-root> -f png examples/label-modes.typ label-modes.png
#import "../src/lib.typ": prefigure

#set page(width: auto, height: auto, margin: 10pt, fill: white)
#set text(font: "New Computer Modern", size: 11pt)

#let src = ```
<diagram dimensions="(240,240)" margins="5">
  <coordinates bbox="[-4,-4,4,4]">
    <grid/>
    <graph function="f(x)=0.4*x*x - 2"/>
    <point p="(0,-2)" alignment="south">vertex</point>
    <label p="(0.6,1.3)" alignment="east">opens upward</label>
    <label p="(-3.4,-3.4)" alignment="ne"><it>a parabola</it> with <b>vertex</b> below</label>
    <label p="(-3.6,3.2)" alignment="se">y-intercept below zero</label>
  </coordinates>
</diagram>
```.text

#grid(
  columns: 2,
  column-gutter: 18pt,
  row-gutter: 6pt,
  align: center,
  [*`labels: "svg"`* — baked], [*`labels: "native"`* — live Typst text],
  prefigure(src, width: 6cm), prefigure(src, labels: "native", width: 6cm),
)
