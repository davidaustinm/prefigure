#import "../src/lib.typ": prefigure, tags, xml-to-string
#set text(font: "Fira Math")
#show math.equation: set text(font: "Fira Math")

#let doc = {
  import tags: *
  diagram(dimensions: (260, 120), {
    show: coordinates.with(bbox: (-4, -4, 4, 4))

    grid-axes(xlabel: "x", ylabel: "y")
    graph(function: "f(x)=0.4*x^2 - 2")
    point(
      p: "(1,f(1))",
      alignment: "southeast",
      $(1, #(0.4 * 1 * 1 - 2))$,
    )
    label(
      p: "(-3, f(-3))",
      alignment: "center",
      clear-background: true,
    )[
      the *curve* $y = 0.4 x^2 - 2$]
  })
}

#prefigure(doc, width: 8cm)

#let doc = {
  import tags: *
  diagram(dimensions: (260, 260), {
    show: coordinates.with(bbox: (-4, -4, 4, 4))
    grid-axes(xlabel: "x", ylabel: "y")
  })
}

#xml-to-string(doc)