const t = (key, vars) => (window.I18n && window.I18n.t ? window.I18n.t(key, vars) : key);

export function escapeHtml(s) {
  return String(s)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function mdTableCells(row) {
  let s = String(row || "").trim();
  if (s.startsWith("|")) s = s.slice(1);
  if (s.endsWith("|")) s = s.slice(0, -1);
  return s.split("|").map((c) => c.trim());
}

function mdTableSepAligns(line) {
  const cells = mdTableCells(line);
  if (!cells.length) return null;
  const aligns = [];
  for (const c of cells) {
    if (!/^:?-{1,}:?$/.test(c) || !c.includes("-")) return null;
    const left = c.startsWith(":");
    const right = c.endsWith(":");
    aligns.push(left && right ? "center" : right ? "right" : "left");
  }
  return aligns;
}

function isMdTableRow(line) {
  const s = String(line || "").trim();
  return s.includes("|") && !mdTableSepAligns(s);
}

function mdAlignAttr(align) {
  return align && align !== "left" ? ' style="text-align:' + align + '"' : "";
}

function mdListItem(raw) {
  const task = String(raw || "").match(/^\[( |x|X)\]\s+(.*)$/);
  if (task) {
    const on = task[1] !== " ";
    return (
      '<li class="task"><input type="checkbox" disabled' +
      (on ? " checked" : "") +
      "> " +
      inlineMd(task[2]) +
      "</li>"
    );
  }
  return "<li>" + inlineMd(raw) + "</li>";
}

export function inlineMd(s) {
  let t = escapeHtml(s);
  t = t.replace(/`([^`]+)`/g, "<code>$1</code>");
  t = t.replace(/!\[([^\]]*)\]\((https?:[^)\s]+)\)/g, '<img src="$2" alt="$1">');
  t = t.replace(/\[([^\]]+)\]\((https?:[^)\s]+)\)/g, '<a href="$2" rel="noreferrer" target="_blank">$1</a>');
  t = t.replace(/~~([^~]+)~~/g, "<del>$1</del>");
  t = t.replace(/\*\*\*([^*]+)\*\*\*/g, "<strong><em>$1</em></strong>");
  t = t.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
  t = t.replace(/__([^_]+)__/g, "<strong>$1</strong>");
  t = t.replace(/(^|[^\w*])\*([^*\n]+)\*(?!\*)/g, "$1<em>$2</em>");
  t = t.replace(/(^|[^\w_])_([^_\n]+)_(?!_)/g, "$1<em>$2</em>");
  return t;
}

export function mdTable(rows) {
  const aligns = mdTableSepAligns(rows[1]) || [];
  const head = mdTableCells(rows[0]);
  let html = '<div class="md-table-wrap"><table><thead><tr>';
  for (let c = 0; c < head.length; c++) {
    html += "<th" + mdAlignAttr(aligns[c]) + ">" + inlineMd(head[c]) + "</th>";
  }
  html += "</tr></thead><tbody>";
  for (let r = 2; r < rows.length; r++) {
    const cols = mdTableCells(rows[r]);
    html += "<tr>";
    const n = Math.max(cols.length, head.length);
    for (let c = 0; c < n; c++) {
      html += "<td" + mdAlignAttr(aligns[c]) + ">" + inlineMd(cols[c] || "") + "</td>";
    }
    html += "</tr>";
  }
  return html + "</tbody></table></div>";
}

export function renderMdBlock(text) {
  const lines = text.split("\n");
  let out = "";
  let i = 0;
  while (i < lines.length) {
    const line = lines[i];
    const trimmed = line.trim();
    if (isMdTableRow(line) && i + 1 < lines.length && mdTableSepAligns(lines[i + 1])) {
      const rows = [line, lines[i + 1]];
      i += 2;
      while (i < lines.length && isMdTableRow(lines[i])) {
        rows.push(lines[i]);
        i++;
      }
      out += mdTable(rows);
      continue;
    }
    const heading = /^(#{1,6})\s+(.*)$/.exec(trimmed);
    if (heading) {
      const n = heading[1].length;
      out += "<h" + n + ">" + inlineMd(heading[2]) + "</h" + n + ">";
      i++;
      continue;
    }
    if (/^>\s?/.test(line)) {
      const quoted = [];
      while (i < lines.length && /^>\s?/.test(lines[i])) {
        quoted.push(lines[i].replace(/^>\s?/, ""));
        i++;
      }
      out += "<blockquote>" + renderMdBlock(quoted.join("\n")) + "</blockquote>";
      continue;
    }
    if (/^(\*\s*){3,}$|^(-\s*){3,}$|^(_\s*){3,}$/.test(trimmed)) {
      out += "<hr>";
      i++;
      continue;
    }
    if (/^[-*+] /.test(trimmed)) {
      out += "<ul>";
      while (i < lines.length && /^[-*+] /.test(lines[i].trim())) {
        out += mdListItem(lines[i].trim().slice(2));
        i++;
      }
      out += "</ul>";
      continue;
    }
    if (/^\d+\. /.test(trimmed)) {
      out += "<ol>";
      while (i < lines.length && /^\d+\. /.test(lines[i].trim())) {
        out += mdListItem(lines[i].trim().replace(/^\d+\. /, ""));
        i++;
      }
      out += "</ol>";
      continue;
    }
    if (trimmed === "") {
      i++;
      continue;
    }
    const para = [line.replace(/ {2}$/, "")];
    let hard = / {2}$/.test(line);
    i++;
    while (
      i < lines.length &&
      lines[i].trim() !== "" &&
      !isMdTableRow(lines[i]) &&
      !mdTableSepAligns(lines[i]) &&
      !/^(#{1,6})\s+/.test(lines[i].trim()) &&
      !/^>\s?/.test(lines[i]) &&
      !/^(\*\s*){3,}$|^(-\s*){3,}$|^(_\s*){3,}$/.test(lines[i].trim()) &&
      !/^[-*+] /.test(lines[i].trim()) &&
      !/^\d+\. /.test(lines[i].trim())
    ) {
      para.push(lines[i].replace(/ {2}$/, ""));
      hard = hard || / {2}$/.test(lines[i]);
      i++;
    }
    const joined = para.map(inlineMd).join(hard ? "<br>" : " ");
    out += "<p>" + joined + "</p>";
  }
  return out;
}

export function codeCardHtml(label, code) {
  return (
    '<div class="code-card"><div class="code-head"><span>' +
    escapeHtml(label || "text") +
    '</span><button type="button" class="copy-code" data-i18n="copyCode">' +
    escapeHtml(t("copyCode")) +
    "</button></div><pre><code>" +
    escapeHtml(String(code == null ? "" : code).replace(/\n$/, "")) +
    "</code></pre></div>"
  );
}

export function renderMarkdown(src) {
  const parts = String(src).split(/```/);
  let html = "";
  for (let i = 0; i < parts.length; i++) {
    if (i % 2 === 1) {
      const nl = parts[i].indexOf("\n");
      let lang = "text";
      let code = parts[i];
      if (nl >= 0) {
        lang = parts[i].slice(0, nl).trim() || "text";
        code = parts[i].slice(nl + 1);
      }
      html += codeCardHtml(lang, code);
      continue;
    }
    html += renderMdBlock(parts[i]);
  }
  return html;
}
