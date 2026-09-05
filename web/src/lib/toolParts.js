import { fileName, langFromPath } from "./helpers.js";

export function shortToolName(block) {
  const t = String((block && block.title) || "tool").trim();
  const m = t.match(/^[A-Za-z0-9_.:-]+/);
  return m ? m[0] : t.slice(0, 16);
}

export function pathFromTool(block, raw) {
  if (raw && typeof raw === "object") {
    const fc = raw.FileContent || (raw.EditsApplied ? { absolute_path: raw.EditsApplied.absolute_path } : null);
    if (fc && fc.absolute_path) return fc.absolute_path;
    if (raw.Content && raw.Content.absolute_root_path) return raw.Content.absolute_root_path;
  }
  const s = String((block && block.input_preview) || "");
  const m = s.match(/(?:target_file|target_directory|path|file)\s*:\s*(\S+)/i);
  return m ? m[1] : "";
}

export function toolCardLabel(block, raw, fallback) {
  const path = pathFromTool(block, raw);
  const name = fileName(path);
  if (name) return name;
  const n = String((block && shortToolName(block)) || "").toLowerCase();
  if (n === "execute" || n === "bash") return "bash";
  if (n === "read") return langFromPath(path);
  return fallback || n || "text";
}

export function asToolText(v) {
  if (v == null) return "";
  if (typeof v === "string") return v;
  if (typeof v === "number" || typeof v === "boolean") return String(v);
  if (Array.isArray(v)) {
    if (!v.length) return "";
    if (v.every((x) => typeof x === "string")) return v.join("");
    if (v.length >= 8 && v.every((x) => Number.isInteger(x) && x >= 0 && x <= 255)) {
      try {
        return new TextDecoder("utf-8").decode(Uint8Array.from(v));
      } catch (e) {
        return "";
      }
    }
  }
  if (typeof v === "object" && typeof v.text === "string") return v.text;
  return "";
}

export function collectAcpParts(content, block, raw) {
  const parts = [];
  if (!Array.isArray(content)) return parts;
  for (const item of content) {
    if (!item || typeof item !== "object") continue;
    if (item.type === "diff") {
      const path = item.path || pathFromTool(block, raw);
      const neu = item.newText || item.new_text || "";
      const old = item.oldText || item.old_text || "";
      const text = old && neu ? "--- old\n" + old + "\n+++ new\n" + neu : neu || old;
      parts.push({ label: fileName(path) || langFromPath(path) || "diff", text });
      continue;
    }
    const inner = item.content;
    if (inner && typeof inner === "object" && typeof inner.text === "string") {
      parts.push({
        label: toolCardLabel(block, raw, "text"),
        text: inner.text
      });
    }
  }
  return parts;
}

export function grepMatchesText(raw) {
  const rows = raw && raw.file_matches;
  if (!Array.isArray(rows) || !rows.length) return asToolText(raw && raw.stdout);
  const lines = [];
  for (const row of rows) {
    if (!row || typeof row !== "object") continue;
    const path = row.path || "";
    const matches = Array.isArray(row.matches) ? row.matches : [];
    if (!matches.length) {
      if (path) lines.push(path);
      continue;
    }
    for (const m of matches) {
      const n = m && m.line_number != null ? m.line_number : "";
      const c = m && m.content != null ? String(m.content) : "";
      lines.push(path + (n !== "" ? ":" + n : "") + (c ? ": " + c : ""));
    }
  }
  return lines.join("\n");
}

export function bashDetailText(raw, block, log) {
  const cmd = String((raw && (raw.command || (raw.Result && raw.Result.command))) || (block && block.input_preview) || "").trim();
  const body = String(log || "").trim()
    || asToolText(raw && raw.output_for_prompt)
    || asToolText(raw && raw.output)
    || asToolText(raw && raw.Result && raw.Result.output)
    || asToolText(raw && raw.stdout);
  const exit = raw && raw.Result && raw.Result.exit_code != null
    ? raw.Result.exit_code
    : (raw && raw.exit_code);
  const bits = [];
  if (cmd) bits.push("$ " + cmd);
  if (body) bits.push(body);
  if (exit != null && exit !== "") bits.push("exit: " + exit);
  return bits.join("\n\n");
}

export function extractToolParts(data, block) {
  data = data || {};
  const content = data.content;
  const raw = data.raw_output;
  const log = typeof data.log === "string" ? data.log : "";
  const rawObj = raw && typeof raw === "object" && !Array.isArray(raw) ? raw : {};
  const kind = String(rawObj.type || "").toLowerCase();

  if (log.trim() || kind === "bash" || kind === "backgroundtaskstarted" || kind === "taskoutput" || rawObj.output_for_prompt || (rawObj.Result && (rawObj.Result.output || rawObj.Result.command))) {
    const text = bashDetailText(rawObj, block, log);
    if (text.trim()) return [{ label: "bash", text }];
  }
  if (kind === "grepsearch" || rawObj.file_matches) {
    const text = grepMatchesText(rawObj);
    if (text.trim()) return [{ label: "text", text }];
  }

  const acp = collectAcpParts(content, block, raw);
  if (acp.length) return acp;
  if (content && typeof content === "object" && !Array.isArray(content) && content.FileContent) {
    const fc = content.FileContent;
    const path = fc.absolute_path || pathFromTool(block, raw);
    return [{ label: fileName(path) || "text", text: asToolText(fc.content) }];
  }
  if (typeof raw === "string" && raw.trim()) {
    return [{ label: toolCardLabel(block, raw, "text"), text: raw }];
  }
  if (rawObj && Object.keys(rawObj).length) {
    if (rawObj.FileContent) {
      const fc = rawObj.FileContent;
      const path = fc.absolute_path || pathFromTool(block, raw);
      return [{ label: fileName(path) || langFromPath(path), text: asToolText(fc.content) }];
    }
    if (rawObj.FileNotFound) {
      return [{ label: "text", text: asToolText(rawObj.FileNotFound) }];
    }
    if (rawObj.ImageContent) {
      return [{ label: "image", text: "[image]" }];
    }
    if (rawObj.Content && typeof rawObj.Content === "object") {
      const path = rawObj.Content.absolute_root_path || "";
      return [{ label: fileName(path) || "text", text: asToolText(rawObj.Content.content) }];
    }
    if (rawObj.EditsApplied) {
      const e = rawObj.EditsApplied;
      const path = e.absolute_path || "";
      return [
        {
          label: fileName(path) || "diff",
          text: asToolText(e.tool_output_for_prompt) || asToolText(e.new_string)
        }
      ];
    }
    if (rawObj.TodosUpdated) {
      const s = asToolText(rawObj.TodosUpdated.summary_for_prompt);
      if (s) return [{ label: "text", text: s }];
    }
    const inner = asToolText(rawObj.content) || asToolText(rawObj.message);
    if (inner) return [{ label: toolCardLabel(block, raw, "text"), text: inner }];
  }
  if (typeof content === "string") {
    return [{ label: toolCardLabel(block, raw, "text"), text: content }];
  }
  const preview = String((block && block.input_preview) || "").trim();
  if (preview) return [{ label: toolCardLabel(block, raw, "text"), text: preview }];
  if (content != null && content !== "") return [{ label: "json", text: JSON.stringify(content, null, 2) }];
  if (rawObj && Object.keys(rawObj).length) return [{ label: "json", text: JSON.stringify(rawObj, null, 2) }];
  return [];
}
