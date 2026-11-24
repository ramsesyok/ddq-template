-- The built-in orange-book filter is selected only for plain typst in a
-- book. The DDQ format uses the same Quarto file metadata and keeps its
-- own page rules. The appendix divider precedes the first appendix file.
local chapter
local function heading(el)
  local metadata = quarto.doc.file_metadata()
  local file = metadata and metadata.file
  if el.level ~= 1 or not file then return end
  if file.bookItemType == "appendix" then
    local numbered = tostring(PANDOC_WRITER_OPTIONS.number_sections == true)
    return pandoc.RawBlock("typst", '#show: appendices.with("付録", numbered: ' .. numbered .. ')')
  end
  if file.bookItemType == "chapter" and file.bookItemFile ~= chapter then
    chapter = file.bookItemFile
    local number = file.bookItemNumber
    if number and not el.classes:includes("unnumbered") then
      return { pandoc.RawBlock("typst", "#counter(heading).update(" .. (number - 1) .. ")"), el }
    end
  end
end

local function metadata(meta)
  local xref = meta.crossref or {}
  local lang = meta.lang and pandoc.utils.stringify(meta.lang):match("^%a+")
  if lang then meta["ddq-typst-lang"] = pandoc.MetaString(lang:lower()) end
  meta["ddq-table-title"] = xref["tbl-title"]
    or pandoc.Inlines{pandoc.Str(quarto.doc.language["crossref-tbl-title"] or "Table")}
  if xref["ref-hyperlink"] == false then meta["ddq-hide-reference-links"] = true end
  if not PANDOC_WRITER_OPTIONS.number_sections then meta["ddq-hide-heading-numbers"] = true end

  -- Quarto leaves these keys out of the metadata when omitted. The
  -- fixed frame uses its own margins and n / N in every orientation.
  for _, key in ipairs({"papersize", "margin", "page-numbering", "columns"}) do
    local value = meta[key]
    if value ~= nil then
      local text = pandoc.utils.stringify(value)
      if not (key == "papersize" and text:lower() == "a4") and not (key == "columns" and text == "1") then
        io.stderr:write("[DDQ] 警告: " .. key ..
          " は発行 PDF の固定様式（A4・外枠と余白・ページ番号欄）を優先するため反映されません。\n")
      end
    end
  end
  return meta
end

return quarto.utils.combineFilters({
  quarto.utils.file_metadata_filter(),
  { Meta = metadata, Header = heading },
})
