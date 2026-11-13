// Conversion between .ewm formatted lines ([[{text, bold, italic}]]) and the
// DOM of a contenteditable element.
//
// Each line is a <div>; bold is <b>, italic is <i>. Reading back is tolerant
// of whatever WebKit / Chromium produce while editing (<br>, <strong>, <em>,
// inline font-weight styles, nested divs, &nbsp;).

/** Fills `editable` with one <div> per line. */
export function linesToDom(editable, lines) {
  editable.replaceChildren();
  const list = lines.length ? lines : [[]];
  for (const raw of list) {
    const div = document.createElement("div");
    const [level, line] = splitTabs(raw);
    setIndent(div, level);
    for (const span of line) {
      let node = document.createTextNode(span.text);
      if (span.italic) node = wrap("i", node);
      if (span.bold) node = wrap("b", node);
      div.append(node);
    }
    if (!div.childNodes.length) div.append(document.createElement("br"));
    editable.append(div);
  }
}

/** A line's leading tabs (its indent) and the line without them. */
function splitTabs(spans) {
  let level = 0;
  const out = [];
  for (const s of spans) {
    if (!out.length) {
      const rest = s.text.replace(/^\t+/, "");
      level += s.text.length - rest.length;
      if (rest) out.push({ ...s, text: rest });
    } else out.push(s);
  }
  return [level, out];
}

/** Shows a line div indented `level` steps (kept in data-indent). */
export function setIndent(div, level) {
  if (level > 0) {
    div.dataset.indent = String(level);
    div.style.paddingLeft = `${level * 1.5}em`;
  } else {
    delete div.dataset.indent;
    div.style.paddingLeft = "";
  }
}

function wrap(tag, child) {
  const e = document.createElement(tag);
  e.append(child);
  return e;
}

const BLOCK_TAGS = new Set(["DIV", "P", "LI", "H1", "H2", "H3", "H4", "H5", "H6", "BLOCKQUOTE"]);

function isBold(node) {
  if (node.nodeName === "B" || node.nodeName === "STRONG") return true;
  const w = node.style?.fontWeight;
  return w === "bold" || Number(w) >= 600;
}

function isItalic(node) {
  return node.nodeName === "I" || node.nodeName === "EM" || node.style?.fontStyle === "italic";
}

/** Reads `editable` back into lines of spans. Blank lines are dropped (a blank
 *  line would end the block in .ewm) and each line is trimmed. */
export function domToLines(editable) {
  const lines = [[]];
  const indents = [0];
  const newLine = () => {
    if (lines[lines.length - 1].length) {
      lines.push([]);
      indents.push(0);
    }
  };
  const push = (text, bold, italic) => {
    text = text.replace(/ /g, " ").replace(/[\r\n]+/g, " ");
    if (!text) return;
    const line = lines[lines.length - 1];
    const last = line[line.length - 1];
    if (last && last.bold === bold && last.italic === italic) last.text += text;
    else line.push({ text, bold, italic });
  };
  const walk = (node, bold, italic) => {
    if (node.nodeType === Node.TEXT_NODE) return push(node.nodeValue, bold, italic);
    if (node.nodeType !== Node.ELEMENT_NODE) return;
    if (node.nodeName === "BR") return newLine();
    const block = BLOCK_TAGS.has(node.nodeName);
    if (block) newLine();
    if (block && node.dataset?.indent) indents[lines.length - 1] = Number(node.dataset.indent) || 0;
    const b = bold || isBold(node);
    const i = italic || isItalic(node);
    node.childNodes.forEach((c) => walk(c, b, i));
    if (block) newLine();
  };
  editable.childNodes.forEach((c) => walk(c, false, false));

  return lines
    .map((line, i) => [indents[i] ?? 0, trimLine(line)])
    .filter(([, line]) => line.length)
    .map(([level, line]) => {
      const out = line.map(({ text, bold, italic }) => compact({ text, bold, italic }));
      if (level > 0) out.unshift({ text: "\t".repeat(level) });
      return out;
    });
}

function trimLine(line) {
  const out = line.map((s) => ({ ...s }));
  while (out.length && !(out[0].text = out[0].text.replace(/^\s+/, ""))) out.shift();
  while (out.length && !(out[out.length - 1].text = out[out.length - 1].text.replace(/\s+$/, ""))) out.pop();
  return out;
}

// Match the serde shape (false flags omitted) so documents compare equal.
function compact(span) {
  const s = { text: span.text };
  if (span.bold) s.bold = true;
  if (span.italic) s.italic = true;
  return s;
}

/** Plain text with "\n" between lines (for search / summaries). */
export const linesToText = (lines) => lines.map((l) => l.map((s) => s.text).join("")).join("\n");
