// Preserve book numbering for the standard Quarto elements not numbered by DDQ.
#let book-numbering(normal, appendix, n, loc) = {
  let pattern = if state("appendix-state", none).at(loc) == none { normal } else { appendix }
  numbering(pattern, counter(heading).at(loc).first(), n)
}
#let equation-numbering = n => context book-numbering("(1.1)", "(A.1)", n, here())
#let callout-numbering = n => context book-numbering("1.1", "A.1", n, here())
#let subfloat-numbering = (n, i) => context {
  let pattern = if state("appendix-state", none).get() == none { "1.1a" } else { "A.1a" }
  numbering(pattern, counter(heading).get().first(), n, i)
}
#let theorem-inherited-levels = 1
#let theorem-numbering(loc) = if state("appendix-state", none).at(loc) == none { "1.1" } else { "A.1" }
#let theorem-render(prefix: none, title: "", full-title: auto, body) = {
  if full-title != "" and full-title != auto and full-title != none {
    strong[#full-title.]
    h(0.5em)
  }
  body
}
