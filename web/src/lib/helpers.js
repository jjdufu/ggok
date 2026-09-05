export const t = (key, vars) => (window.I18n && window.I18n.t ? window.I18n.t(key, vars) : key);

export function setDynI18n(el, key, vars) {
  if (!el) return;
  el.setAttribute("data-i18n-dyn", key);
  if (vars && Object.keys(vars).length) el.setAttribute("data-i18n-vars", JSON.stringify(vars));
  else el.removeAttribute("data-i18n-vars");
  el.textContent = t(key, vars);
}

export function relocalizeDyn(root) {
  const el = root || document;
  if (!el.querySelectorAll) return;
  el.querySelectorAll("[data-i18n-dyn]").forEach((n) => {
    const key = n.getAttribute("data-i18n-dyn");
    let vars;
    const raw = n.getAttribute("data-i18n-vars");
    if (raw) {
      try {
        vars = JSON.parse(raw);
      } catch (err) {
        vars = undefined;
      }
    }
    n.textContent = t(key, vars);
  });
}

export function setTip(el, text) {
  if (window.I18n && typeof window.I18n.setTip === "function") window.I18n.setTip(el, text);
  else if (el) {
    if (text) el.setAttribute("data-tip", String(text));
    else el.removeAttribute("data-tip");
    el.removeAttribute("title");
  }
}

export function hideTip(armSkip) {
  if (window.I18n && typeof window.I18n.hideTip === "function") window.I18n.hideTip(armSkip);
}

export function setPressed(btn, on) {
  if (!btn) return;
  btn.classList.toggle("on", on);
  btn.setAttribute("aria-pressed", on ? "true" : "false");
}

export function relTime(iso) {
  if (!iso) return "";
  const ts = Date.parse(iso);
  if (Number.isNaN(ts)) return iso;
  const sec = Math.round((Date.now() - ts) / 1000);
  if (sec < 60) return t("justNow");
  if (sec < 3600) return t("minutesAgo", { n: Math.floor(sec / 60) });
  if (sec < 86400) return t("hoursAgo", { n: Math.floor(sec / 3600) });
  if (sec < 86400 * 30) return t("daysAgo", { n: Math.floor(sec / 86400) });
  return iso.slice(0, 10);
}

export function shortCwd(cwd) {
  const parts = String(cwd || "").replace(/\/+$/, "").split("/").filter(Boolean);
  if (!parts.length) return cwd || "";
  const tail = parts.length <= 2 ? parts : parts.slice(-2);
  const label = tail.join("/");
  return label.charAt(0).toUpperCase() + label.slice(1);
}

export function parentOf(path) {
  const n = String(path || "").replace(/\/+$/, "");
  if (!n || n === "/") return "";
  const i = n.lastIndexOf("/");
  if (i <= 0) return "";
  return n.slice(0, i);
}

export function filenameFromDisposition(h) {
  if (!h) return "";
  const star = /filename\*\s*=\s*UTF-8''([^;]+)/i.exec(h);
  if (star) {
    try {
      return decodeURIComponent(star[1]);
    } catch (err) {
      return star[1];
    }
  }
  const quoted = /filename\s*=\s*"([^"]+)"/i.exec(h);
  if (quoted) return quoted[1];
  const plain = /filename\s*=\s*([^;]+)/i.exec(h);
  return plain ? plain[1].trim() : "";
}

export function fileName(path) {
  const s = String(path || "");
  if (!s) return "";
  const i = s.lastIndexOf("/");
  return i >= 0 ? s.slice(i + 1) : s;
}

export function fileNameOf(f) {
  return (f && (f.name || f.rel)) || String((f && f.path) || "").split("/").pop() || "file";
}

export function uploadUrl(f) {
  if (!f) return "";
  const direct = String(f.url || "");
  if (direct.startsWith("/api/uploads")) return direct;
  const path = String(f.path || f.rel || "");
  if (!path) return "";
  if (path.startsWith("/api/uploads")) return path;
  return "/api/uploads?path=" + encodeURIComponent(path);
}

export function fileViewSrc(f) {
  const url = uploadUrl(f);
  if (url) return url;
  const preview = String((f && f.preview) || "");
  return preview;
}

export function isImageAttach(f) {
  const mime = String((f && f.mime) || "").toLowerCase();
  if (mime.startsWith("image/")) return true;
  const name = String((f && (f.name || f.rel || f.path)) || "").toLowerCase();
  return /\.(png|jpe?g|gif|webp|bmp|svg)$/.test(name);
}

export function revokePreview(f) {
  if (f && f.preview && String(f.preview).startsWith("blob:")) {
    try {
      URL.revokeObjectURL(f.preview);
    } catch (e) {
    }
    f.preview = "";
  }
}

export function langFromPath(path) {
  const base = fileName(path);
  const i = base.lastIndexOf(".");
  if (i < 0) return "text";
  const ext = base.slice(i + 1).toLowerCase();
  const map = {
    js: "javascript",
    mjs: "javascript",
    cjs: "javascript",
    ts: "typescript",
    tsx: "tsx",
    jsx: "jsx",
    rs: "rust",
    py: "python",
    go: "go",
    rb: "ruby",
    css: "css",
    scss: "scss",
    html: "html",
    htm: "html",
    md: "markdown",
    json: "json",
    toml: "toml",
    yml: "yaml",
    yaml: "yaml",
    sh: "bash",
    bash: "bash",
    zsh: "bash",
    c: "c",
    h: "c",
    cc: "cpp",
    cpp: "cpp",
    java: "java",
    sql: "sql",
    xml: "xml",
    svg: "xml",
    vue: "vue"
  };
  return map[ext] || ext || "text";
}

export function fmtNum(n) {
  return Number(n || 0).toLocaleString(window.I18n && window.I18n.locale ? window.I18n.locale() : "en-US");
}

export function fmtDur(ms) {
  const n = Number(ms || 0);
  if (n < 1000) return n + "ms";
  const s = Math.round(n / 1000);
  if (s < 60) return s + "s";
  const m = Math.floor(s / 60);
  const r = s % 60;
  if (m < 60) return r ? m + "m " + r + "s" : m + "m";
  const h = Math.floor(m / 60);
  const rm = m % 60;
  return rm ? h + "h " + rm + "m" : h + "h";
}

export function fmtCost(ticks) {
  return "$" + (Number(ticks || 0) / 1e10).toFixed(4);
}

export function fmtBytes(n) {
  const x = Number(n || 0);
  if (x < 1024) return x + " B";
  if (x < 1024 * 1024) return (x / 1024).toFixed(1) + " KiB";
  if (x < 1024 * 1024 * 1024) return (x / (1024 * 1024)).toFixed(1) + " MiB";
  return (x / (1024 * 1024 * 1024)).toFixed(1) + " GiB";
}

export function fmtTok(n) {
  const x = Number(n || 0);
  if (x >= 1000000) return Math.round(x / 1000000) + "M";
  if (x >= 1000) return Math.round(x / 1000) + "k";
  return String(x);
}

export function periodLabel(p) {
  if (p === "weekly") return t("weeklyLimit");
  if (p === "monthly") return t("monthlyLimit");
  if (p === "daily") return t("dailyLimit");
  return t("usageLimit");
}

export function fmtReset(iso) {
  const at = Date.parse(iso);
  if (!at) return "";
  const ms = at - Date.now();
  if (ms <= 0) return t("resetsSoon");
  const m = Math.floor(ms / 60000);
  const h = Math.floor(m / 60);
  const d = Math.floor(h / 24);
  if (d >= 1) return t("resetsInDh", { d: d, h: h % 24 });
  if (h >= 1) return t("resetsInHm", { h: h, m: m % 60 });
  return t("resetsInM", { m: Math.max(1, m) });
}

export function fmtResetDate(iso) {
  const at = Date.parse(iso);
  if (!at) return "";
  const d = new Date(at);
  const locale = window.I18n && typeof window.I18n.locale === "function" ? window.I18n.locale() : undefined;
  const opts = { year: "numeric", month: "short", day: "numeric" };
  if (d.getHours() || d.getMinutes()) {
    opts.hour = "2-digit";
    opts.minute = "2-digit";
  }
  return d.toLocaleString(locale, opts);
}

export function i18nOr(key, fallback) {
  const tr = t(key);
  return tr !== key ? tr : fallback;
}

export function isMac() {
  return /Mac|iPhone|iPad/.test(navigator.platform || "");
}

export function oneLinePreview(text) {
  const line = String(text || "")
    .split(/\r?\n/)
    .map((s) => s.trim())
    .find(Boolean);
  return line ? line.replace(/\s+/g, " ") : "";
}

export function focusKeyThought(promptId, idx) {
  return "thought:" + (promptId || "") + ":" + idx;
}

export function focusKeyTool(id) {
  return "tool:" + id;
}

export function formatError(err, fallbackKey) {
  const fallback = t(fallbackKey || "requestFailed");
  let raw = "";
  if (err == null || err === "") return fallback;
  if (typeof err === "string") raw = err;
  else if (typeof err.message === "string") raw = err.message;
  else raw = String(err);
  raw = String(raw).replace(/\u001b\[[0-9;]*m/g, "").trim();
  if (!raw) return fallback;
  if (/^<!DOCTYPE|^<html/i.test(raw)) return fallback;
  if (raw.charAt(0) === "{" || raw.charAt(0) === "[") {
    try {
      const v = JSON.parse(raw);
      if (v && typeof v === "object") {
        raw = v.message || v.error || v.msg || v.text || raw;
        if (typeof raw !== "string") raw = JSON.stringify(raw);
      }
    } catch (e) {
    }
  }
  const lines = raw.split(/\r?\n/).map((s) => s.trim()).filter(Boolean);
  const useful = lines.filter(
    (l) => !/^at\s|^stack backtrace|^note:|^#\d+\s/i.test(l) && !/^Error$/i.test(l)
  );
  let msg = useful[0] || lines[0] || fallback;
  msg = msg.replace(/^Error:\s*/i, "").trim();
  if (!msg) return fallback;
  if (msg.length > 220) msg = msg.slice(0, 217) + "...";
  return msg;
}
