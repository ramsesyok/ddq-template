-- At post-quarto, Quarto has indexed the floats and left one span per
-- reference for the book resolver. The resolver replaces its contents and
-- class, so retain the ID in an attribute even with ref-hyperlink: false.
local kinds = { fig = true, tbl = true, lst = true }
local chapter

local function reference(span)
  if span.classes:includes("quarto-unresolved-ref") then
    span.attributes["data-ddq-ref"] = pandoc.utils.stringify(span)
    if span.classes:includes("ref-noprefix") then
      span.attributes["data-ddq-no-prefix"] = "true"
    end
    return span
  end
end

local function float_number(float)
  if float.order and float.order.section then
    chapter = chapter or float.order.section[1]
  end
  if float.classes:includes("ddq-table-anchor") then
    return pandoc.Div({}, pandoc.Attr(float.identifier))
  end
  local id = float.identifier
  if not id or not kinds[id:match("^(%a+)%-")] or not float.order or float.parent_id then return end
  -- The index owns the original order. Give the HTML caption renderer a
  -- placeholder in a new order object, preserving its title and delimiter.
  float.order = { order = "DDQFLOATNUMBER" }
  return float
end

local function document(doc)
  quarto.doc.include_text("in-header", '<meta name="ddq-format" content="html">')
  local crossref = doc.meta.crossref or {}
  if crossref["chapters-alpha"] == true and chapter and chapter > 0 then
    quarto.doc.include_text("in-header", '<meta name="ddq-appendix" content="' .. chapter .. '">')
  end
end

return { Pandoc = document, Span = reference, FloatRefTarget = float_number }
