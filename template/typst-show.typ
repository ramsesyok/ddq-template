// Quarto 用テンプレートパーシャル。
// qmd のフロントマター（title / subtitle / author / doc-number / doc-revision /
// spec / company-ja / company-en / cover / page-start / toc / toc-title / toc-depth /
// pagebreak-level）を
// lib.typ の design-doc() に引き渡す。
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
$if(doc-number)$
  doc-number: "$doc-number$",
$endif$
$if(doc-revision)$
  doc-revision: "$doc-revision$",
$endif$
$if(spec)$
  spec: true,
$endif$
$if(company-ja)$
  company-ja: "$company-ja$",
$endif$
$if(company-en)$
  company-en: "$company-en$",
$endif$
$if(cover)$
  cover: true,
$endif$
$if(page-start)$
  page-start: $page-start$,
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
$if(pagebreak-level)$
  pagebreak-level: $pagebreak-level$,
$endif$
)
