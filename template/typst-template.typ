// Typst compiles the assembled book at the project root.
#import "/_extensions/ddq/lib.typ": *

#let quarto-callout = callout
#let callout = with-html-callout-icons(quarto-callout)

$if(highlighting-definitions)$
// The default Skylighting writer emits a block of formatted inline tokens.
// Apply the code style at that entry point for any highlighting theme.
#let quarto-skylighting = Skylighting
#let Skylighting(..args) = ddq-code-block(quarto-skylighting(..args))
$endif$
