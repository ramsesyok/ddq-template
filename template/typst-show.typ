// Forward the resolved Quarto options and the public DDQ settings.
#show: design-doc.with(
$if(title)$
  title: "$title$",
$endif$
$if(subtitle)$
  subtitle: "$subtitle$",
$endif$
$if(by-author)$
  author: "$for(by-author)$$it.name.literal$$sep$, $endfor$",
$endif$
$if(ddq.doc-number)$
  doc-number: "$ddq.doc-number$",
$endif$
$if(ddq.doc-revision)$
  doc-revision: "$ddq.doc-revision$",
$endif$
$if(ddq.spec)$
  spec: true,
$endif$
$if(ddq.company-ja)$
  company-ja: "$ddq.company-ja$",
$endif$
$if(ddq.company-en)$
  company-en: "$ddq.company-en$",
$endif$
$if(ddq.cover)$
  cover: true,
$endif$
$if(ddq.page-start)$
  page-start: $ddq.page-start$,
$endif$
$if(toc)$
  toc: true,
$endif$
$if(toc-title)$
  toc-title: "$toc-title$",
$endif$
$if(toc-depth)$
  toc-depth: $toc-depth$,
$endif$
$if(toc-placed)$
  toc-placed: true,
$endif$
$if(ddq.pagebreak-level)$
  pagebreak-level: $ddq.pagebreak-level$,
$endif$
$if(ddq-typst-lang)$
  language: "$ddq-typst-lang$",
$endif$
$if(mainfont)$
  body-font: ("$mainfont$",),
$endif$
$if(fontsize)$
  font-size: $fontsize$,
$endif$
$if(linestretch)$
  line-stretch: $linestretch$,
$endif$
$if(codefont)$
  code-font: ($for(codefont)$"$codefont$",$endfor$),
$endif$
$if(ddq-hide-heading-numbers)$
  numbered-headings: false,
$endif$
$if(number-depth)$
  number-depth: $number-depth$,
$endif$
$if(ddq-hide-reference-links)$
  reference-links: false,
$endif$
$if(ddq-table-title)$
  table-title: [$ddq-table-title$],
$endif$
$if(crossref.title-delim)$
  title-delim: [$crossref.title-delim$],
$endif$
)
