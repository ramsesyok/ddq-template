// 本物の lib.typ を読み、描画されたテキストをページごとに採取する。
// 内部の採番用 metadata には依存しない。
#import "lib.typ": _tbl-auto, _xref
#set page(width: 180mm, height: 120mm, margin: 10mm)
#set text(size: 10pt)
#counter(heading).update(1)
#show text: t => [#metadata(t.text)<rendered-text>#t]

#let tall(key) = [#set text(size: 8pt)
  #for i in range(100) [#(key + str(i))#linebreak()]]
#let tblc = counter(figure.where(kind: "quarto-float-tbl"))
#let one(name, col: 3, first: true, last: true, numbered: true, rows: 1, breakable: true, header: true, tall-last: false) = {
  if first and numbered {
    tblc.step()
    [#metadata(none)#label("sn-tbl-" + name)]
  }
  _tbl-auto(cap: name, first: first, last: last, numbered: numbered, breakable-rows: breakable)[
    #table(columns: (1fr, 1fr, 1fr, 1fr, 1fr),
      ..if header { (table.header([A], [B], [C], [D], [E]),) } else { () },
      ..range(rows).map(r => range(5).map(c => {
        if c == col and (rows == 1 or (tall-last and r == rows - 1)) { tall("CELL-" + name + "-") }
        else { [#("ROW-" + name + "-" + str(r) + "-" + str(c))] }
      })).flatten())
  ]
}

#let case = sys.inputs.at("case")
#if case == "first" { one("FIRST", col: 0) } else if case == "middle" { one("MIDDLE", col: 3) } else if case == "last" { one("LAST", col: 4) } else if case == "following" {
  one("MIDDLE", col: 3)
  pagebreak()
  one("NEXT", col: 4)
} else if case == "last-row" {
  one("LASTROW", rows: 8, tall-last: true)
} else if case == "same-page" {
  one("SMALL", rows: 2)
  one("MIDDLE", col: 3)
  one("SMALLNEXT", rows: 2)
} else if case == "manual" {
  one("MANUAL", col: 3, last: false)
  pagebreak(weak: true)
  one("MANUAL", col: 4, first: false)
  pagebreak()
  one("NEXT", rows: 2)
} else if case == "ordinary" { one("ORDINARY", rows: 3, breakable: false) } else if case == "many-rows" { one("MANY", rows: 100, breakable: false) } else if case == "no-header" { one("NOHEADER", col: 2, header: false) } else if case == "unnumbered" { one("UNNUMBERED", col: 3, numbered: false) } else if case == "header-only" { one("EMPTY", rows: 0) } else if case == "merged" {
  tblc.step()
  _tbl-auto(cap: "MERGED", breakable-rows: true)[
    #table(columns: (1fr, 3fr, 1fr),
      table.header(table.cell(colspan: 2)[HEADER-MERGED], [C]),
      table.cell(rowspan: 2)[ROWSPAN], tall("CELL-MERGED-"), [short],
      table.cell(colspan: 2)[COLSPAN])
  ]
} else if case == "references" {
  [BEFORE #_xref("tbl-MIDDLE")]
  one("MIDDLE", col: 3)
  [AFTER #_xref("tbl-MIDDLE")]
} else { panic("unknown case: " + case) }

#context [#metadata(query(<rendered-text>).map(m =>
  (page: m.location().page(), text: m.value)))<rendered-pages>]
