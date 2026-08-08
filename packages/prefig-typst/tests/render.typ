// Typst-native integration test: exercises the full A–D pipeline against the
// built wasm plugin. Compiling this file *is* the test — every `#assert` below
// runs during layout, and `run.sh` fails the suite if compilation fails.
//
// Run via ../tests/run.sh (which points typst at this file), or directly:
//   typst compile --root <repo> tests/render.typ tests/out.png

#import "../src/lib.typ": prefigure
#import "../src/fonts.typ": resolve-font-map
#import "../src/color.typ": css-color
#import "../src/native.typ": baseline-corner

#let fixtures = "fixtures"
#let plug = plugin("../src/prefig_typst_plugin.wasm")

#set page(width: auto, height: auto, margin: 12pt)

// --- Pass A invariants (pure, no context needed) --------------------------
#let text-src = read(fixtures + "/text_label.xml")
#let extracted = json(plug.extract_measurables(bytes(json.encode(
  (source: text-src, format: "svg", font_map: resolve-font-map(none)),
))))
#let runs = extracted.measurables
#assert(
  runs.map(m => m.text) == ("Hello", "slanted", "and", "bold"),
  message: "Pass A enumerated the wrong runs: " + repr(runs.map(m => m.text)),
)
#assert(
  runs.all(m => m.family == "DejaVu Sans"),
  message: "generic font family was not mapped to a concrete one",
)

// --- Colour strings are read the way resvg reads them ---------------------
// Native labels must land on the same colour the baked `<text>` would, for every
// form PreFigure can emit: the full CSS name set (not just the common dozen),
// hex, and the functional notations — `rgb(r,g,b)` is what PreFigure writes for
// a computed colour (user_namespace.valid_eval).
#let same(a, b) = css-color(a).to-hex() == rgb(b).to-hex()
#assert(
  same("red", "#ff0000") and same("Red", "#ff0000"),
  message: "named colour",
)
#assert(
  same("crimson", "#dc143c") and same("rebeccapurple", "#663399"),
  message: "colour name outside the common set",
)
#assert(
  same("#f00", "#ff0000") and same("#ff000080", "#ff000080"),
  message: "hex colour",
)
#assert(
  same("rgb(255,0,0)", "#ff0000") and same("rgb(100%, 0%, 0%)", "#ff0000"),
  message: "rgb() colour",
)
#assert(
  same("rgb(255 0 0 / 50%)", "#ff000080"),
  message: "CSS 4 space/slash rgb()",
)
#assert(same("rgba(0,128,0,0.5)", "#00800080"), message: "rgba() colour")
#assert(
  same("hsl(0, 100%, 50%)", "#ff0000")
    and same("hsl(120deg 100% 25%)", "#008000"),
  message: "hsl() colour",
)
#assert(
  same("hsl(0.5turn, 100%, 50%)", "#00ffff"),
  message: "hsl() with a turn hue",
)
#assert(
  css-color(none) == black and same("none", "#00000000"),
  message: "missing/none colour",
)

// --- The pivot both label transforms must use ------------------------------
// A transform may move anything except the point the placement targets: the
// start of the baseline. Which corner that is depends on how the box was built,
// so the two kinds must NOT share a pivot — collapsing both to `top + left`
// (the obvious simplification) drops math off the baseline under scale/rotate.
#assert(
  baseline-corner(false) == left + top,
  message: "a text box's baseline is its top edge",
)
#assert(
  baseline-corner(true) == left + bottom,
  message: "a math box's baseline is its bottom edge, so it pivots at the bottom",
)

// --- Full render of every fixture (Passes A–D) ----------------------------
// Only text_only is math-free — `<grid-axes>` generates axis labels and tick
// numbers, which are math — so it alone takes the self-contained-SVG path (baked
// text via resvg, font-map families). Everything below it has math and is
// therefore drawn native, with mitex converting the generated LaTeX.
#let only-src = read(fixtures + "/text_only.xml")
#assert(
  json(plug.extract_measurables(bytes(json.encode(
    (source: only-src, format: "svg", font_map: resolve-font-map(none)),
  ))))
    .at("math", default: ())
    .len()
    == 0,
  message: "text_only.xml is meant to be math-free, so it can exercise labels: \"svg\"",
)

= Text labels (svg-baked)
#prefigure(only-src)

= Font override (svg-baked)
#prefigure(only-src, fonts: (sans-serif: "New Computer Modern"))

= Axis labels and tick numbers (mitex)
#prefigure(text-src, width: 5cm)

= Math label (mitex)
#prefigure(read(fixtures + "/math_label.xml"), width: 5cm)

= No labels (geometry only)
#prefigure(read(fixtures + "/no_labels.xml"), width: 5cm)

// --- Native-label mode: text overlaid as live Typst content ---------------
// Driving the plugin directly, so the generated math needs stand-in dimensions:
// this build embeds no math engine, and `build` refuses to drop an `<m>` it can
// render no other way.
#let dims-for(ex) = (
  ex
    .at("math", default: ())
    .fold((:), (d, body) => {
      d.insert(body, (12.0, 9.0, 3.0))
      d
    })
)
#let native = json(plug.build(bytes(json.encode((
  source: text-src,
  format: "svg",
  labels: "native",
  font_map: resolve-font-map(none),
  metrics: runs.map(m => (..m, width: 20.0, above: 10.0, below: 2.0)),
  math_dims: dims-for(extracted),
)))))
#assert(
  not native.svg.contains("<text"),
  message: "native mode leaked <text> into the SVG",
)
#assert(
  native.labels.filter(l => not l.math).map(l => l.text)
    == ("Hello", "slanted", "and", "bold"),
  message: "native text placements wrong: "
    + repr(native.labels.map(l => (l.text, l.math))),
)
// The generated axis labels and tick numbers come back as math placements for
// the host to draw — none of them may be dropped on the way through the build.
#assert(
  native.labels.filter(l => l.math).map(l => l.text).dedup().sorted()
    == extracted.at("math").sorted(),
  message: "generated math lost between Pass A and the build: "
    + repr(native.labels.filter(l => l.math).map(l => l.text)),
)

= Native labels (Typst text overlaid on the SVG)
#prefigure(text-src, labels: "native")

// --- Rotation invariant: native placements must match the baked SVG's -------
// handedness. `rotatestr` emits `rotate(-theta)`, so a rotate="90" label reads
// bottom-to-top on screen: its first run sits BELOW (larger y) its last run, and
// both share an x. A sign slip in the placement rotation mirrors the label
// (first run above last), which this guards against — see native.typ / label.rs.
#let stress = read(fixtures + "/native_stress.xml")
#let sex = json(plug.extract_measurables(bytes(json.encode(
  (source: stress, format: "svg", font_map: resolve-font-map(none)),
))))
#let snat = json(plug.build(bytes(json.encode((
  source: stress,
  format: "svg",
  labels: "native",
  font_map: resolve-font-map(none),
  metrics: sex.measurables.map(m => (
    ..m,
    width: 30.0,
    above: 10.0,
    below: 3.0,
  )),
  math_dims: dims-for(sex),
)))))
#let rot = snat.labels.filter(l => l.angle != 0)
#assert(
  rot.len() >= 2,
  message: "expected a rotated multi-run label in native_stress",
)
#assert(
  rot.first().text == "Time" and rot.last().text == "(sec)",
  message: "rotated run order changed: " + repr(rot.map(l => l.text)),
)
#assert(
  rot.first().y > rot.last().y,
  message: "rotated label mirrored: first run should sit below last (y "
    + repr(rot.first().y)
    + " vs "
    + repr(rot.last().y)
    + ")",
)
#assert(
  calc.abs(rot.first().x - rot.last().x) < 0.5,
  message: "rotated runs should share an x: " + repr(rot.map(l => l.x)),
)

// The same label's `<m>t</m>` must travel with its text: same baseline line
// (shared x), same angle and scale, and laid out *between* the two text runs in
// row order. Asserting only the text runs above left the math side of this
// fixture uncovered even though it is the case that exercises it.
#let rot-math = rot.filter(l => l.math)
#assert(
  rot-math.len() == 1 and rot-math.first().text == "t",
  message: "expected one math run in the rotated label: "
    + repr(rot.map(l => (l.text, l.math))),
)
#let m = rot-math.first()
#assert(
  m.angle == rot.first().angle and m.scale == rot.first().scale,
  message: "math run did not inherit the label's rotate/scale: "
    + repr((m.angle, m.scale))
    + " vs "
    + repr((rot.first().angle, rot.first().scale)),
)
#assert(
  calc.abs(m.x - rot.first().x) < 0.5,
  message: "math run left the rotated label's baseline: "
    + repr((rot.first().x, m.x)),
)
#assert(
  rot.first().y > m.y and m.y > rot.last().y,
  message: "math run is out of row order (expected Time > t > (sec) in y): "
    + repr(rot.map(l => (l.text, l.y))),
)

// Colours reach the placements verbatim, in whatever CSS form PreFigure wrote —
// the overlay parses them (color.typ), so nothing is normalised on the way out.
#let by-text = snat.labels.fold((:), (d, l) => {
  d.insert(l.text, l)
  d
})
#assert(
  by-text.at("colored").color == "blue"
    and by-text.at("e^x").color == "magenta",
  message: "per-component colour lost: "
    + repr(snat.labels.map(l => (l.text, l.color))),
)
#assert(
  ("name", "rgb", "hsl", "hex").map(t => by-text.at(t).color)
    == ("rebeccapurple", "rgb(30,144,255)", "hsl(120, 60%, 35%)", "#b8860b"),
  message: "exotic colour strings did not survive: "
    + repr(("name", "rgb", "hsl", "hex").map(t => by-text.at(t).color)),
)

= Rotated / scaled / colored stress fixture
#grid(
  columns: 2,
  column-gutter: 12pt,
  prefigure(stress, width: 6cm),
  prefigure(stress, labels: "native", width: 6cm),
)

= Legend with math (native)
#prefigure(read(fixtures + "/native_legend.xml"), labels: "native", width: 6cm)

// --- Generated math is Typst-rendered via mitex -----------------------------
// A pi-format axis makes PreFigure generate LaTeX tick labels (`\frac{\pi}{2}`,
// `\pi`, …). Pass A must surface them so lib.typ can hand them to mitex; assert
// the complex forms are enumerated (offline — no mitex needed for this check).
#let pi-src = read(fixtures + "/pi_ticks.xml")
#let pi-math = json(plug.extract_measurables(bytes(json.encode(
  (source: pi-src, format: "svg", font_map: resolve-font-map(none)),
)))).at("math", default: ())
#assert(
  "\\frac{\\pi}{2}" in pi-math and "\\pi" in pi-math,
  message: "pi-format axis did not generate \\frac/\\pi tick labels: "
    + repr(pi-math),
)

= Generated pi-format ticks rendered by Typst (mitex)
// The generated (non-sentinel) tick bodies enumerated above flow through mitex on
// a plain render — no flags — so compiling this line exercises the conversion end
// to end (\frac{\pi}{2}, \pi, …) alongside the `x`/`y` axis labels.
#prefigure(pi-src, width: 8cm)

// --- Authoring with the re-exported `tags` module + non-string sources -------
// prefigure() accepts an xmlit tree (built from the `tags.*` constructors) and
// raw bytes, not only an XML string. The tree's `tags.*` bake in PreFigure's
// handlers (_…_→<it>, *…*→<b>) and its authored `$…$` auto-extracts to a
// sentinel + math-items — the same product the manual xml-to-string route gives.
#import "../src/lib.typ": tags
#import "@preview/xmlit:0.1.3": xml-to-string
#let authored = tags.diagram(dimensions: "(200,200)", tags.coordinates(
  bbox: "[-3,-3,3,3]",
  tags.grid-axes(xlabel: "x", ylabel: "y"),
  tags.graph(function: "f(x)=x*x - 1"),
  tags.label(p: "(1.2,1)")[the curve $y = x^2$ _here_ *bold*],
))
#let authored-xml = xml-to-string(authored, extract-math: true).xml
#assert(
  authored-xml.contains("<it>here</it>")
    and authored-xml.contains("<b>bold</b>"),
  message: "tags handlers should map _…_→<it> and *…*→<b>: " + authored-xml,
)
#assert(
  authored-xml.contains("⟦math-0⟧"),
  message: "an authored $…$ should extract to a sentinel: " + authored-xml,
)

= Authored from the `tags` module, passed as a tree
#prefigure(authored, width: 5cm)

= XML supplied as bytes (read(…, encoding: none))
#prefigure(read(fixtures + "/text_only.xml", encoding: none), width: 5cm)

// --- Schema coverage: `tags` exports every element in the PreFigure schema -----
// The RELAX NG schema (compact syntax) is the authoritative element set. Every
// `element <name>` it declares must have a constructor in the re-exported `tags`
// module, so a diagram the schema permits can be authored in Typst. (Tags the
// engine also accepts but the schema omits — center, set-eye, the 3d transforms,
// smooth béziers — are intentionally NOT required here.) The schema lives in the
// sibling `prefig` package; this reads it from the repo root (run.sh compiles
// with --root at the repo root).
#let schema = read("/packages/prefig/resources/schema/pf_schema.rnc")
#let schema-matches = schema.matches(
  regex("element\s+([A-Za-z][A-Za-z0-9_-]*)"),
)
#let schema-tags = schema-matches.map(m => m.captures.at(0)).dedup()
// Guard against the regex silently matching nothing (which would make the
// coverage check below pass vacuously): the schema has ~71 distinct elements.
#assert(
  schema-tags.len() > 60,
  message: "only "
    + str(schema-tags.len())
    + " schema elements parsed — did the "
    + "schema path or `element <name>` syntax change?",
)
#let exported = dictionary(tags)
#let missing = schema-tags.filter(t => t not in exported)
#assert(
  missing.len() == 0,
  message: "the `tags` module is missing schema element(s): "
    + repr(missing.sorted()),
)

// --- Validation wiring: the schema accepts a valid diagram and rejects
// an invalid one. Uses xmlit's non-panicking `validate` (so this test never
// aborts) on the SAME schema prefigure() validates against — a guard
// that `validate: true` is actually hooked up and behaving.
#import "@preview/xmlit:0.1.3": create-from-relaxng
#let _val = create-from-relaxng(read("../src/pf_schema.rnc")).utils.validate
#assert(
  _val(read(fixtures + "/no_labels.xml")).valid,
  message: "no_labels.xml should pass schema validation",
)
#let _bad-src = "<diagram dimensions=\"(120,120)\"><coordinates bbox=\"[-2,-2,2,2]\"><grid/><label p=\"(0,0)\">x</label></coordinates></diagram>"
#assert(
  not _val(_bad-src).valid,
  message: "a <label p=…> diagram should fail validation (schema defect D4)",
)

// `validate: true` is non-fatal: an invalid diagram must still render (compiling
// this line is the test — a panic would fail the suite) and carry the callout.
// (Validation is opt-in now, so this passes `validate: true` explicitly.)
= Invalid diagram, validate: true (renders + inline error callout)
#prefigure(_bad-src, width: 4cm, validate: true)
