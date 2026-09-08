import { promptApi } from "../promptApi.js";
import { t, uploadUrl, isImageAttach, revokePreview } from "../lib/helpers.js";
import { toast } from "../lib/clipboard.js";

const PREFIX = "ggok-draft:";
const CHANNEL = "ggok-draft";
const TAB =
  (typeof crypto !== "undefined" && crypto.randomUUID && crypto.randomUUID()) ||
  `${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
const MAX = 200 * 1024;

function emptyDraft() {
  return { text: "", files: [], ts: 0 };
}

function normalizeFiles(raw) {
  if (!Array.isArray(raw)) return [];
  const out = [];
  for (const f of raw) {
    if (!f || typeof f.path !== "string" || !f.path) continue;
    out.push({
      path: f.path,
      mime: typeof f.mime === "string" ? f.mime : "",
      name: typeof f.name === "string" ? f.name : ""
    });
  }
  return out;
}

function storageGet(key) {
  try {
    const raw = localStorage.getItem(key);
    if (!raw) return emptyDraft();
    const v = JSON.parse(raw);
    if (!v || typeof v.text !== "string") return emptyDraft();
    return { text: v.text, files: normalizeFiles(v.files), ts: Number(v.ts) || 0 };
  } catch (e) {
    return emptyDraft();
  }
}

function storageSet(key, text, files, ts) {
  try {
    if (!text && !(files && files.length)) {
      localStorage.removeItem(key);
      return;
    }
    const payload = JSON.stringify({ text, files: files || [], ts });
    if (payload.length > MAX) return;
    localStorage.setItem(key, payload);
  } catch (e) {
    /* quota / private mode */
  }
}

async function uploadExists(f) {
  const url = uploadUrl(f);
  if (!url) return false;
  try {
    const res = await fetch(url, { method: "HEAD", credentials: "same-origin" });
    return res.ok;
  } catch (e) {
    return false;
  }
}

export function bindDraftSync(ctx) {
  let applying = false;
  let timer = 0;
  let lastTs = 0;
  let restoreSeq = 0;
  let bc = null;
  try {
    if (typeof BroadcastChannel === "function") bc = new BroadcastChannel(CHANNEL);
  } catch (e) {
    bc = null;
  }

  function keyFor(id) {
    if (id) return PREFIX + id;
    return PREFIX + "new";
  }

  function currentKey() {
    return keyFor(ctx.currentId);
  }

  function snapshotFiles() {
    return (ctx.attachments || [])
      .filter((a) => a && a.path && !a.processing)
      .map((a) => ({
        path: a.path,
        mime: a.mime || "",
        name: a.name || a.rel || ""
      }));
  }

  function restoreFiles(files) {
    const seq = ++restoreSeq;
    const list = normalizeFiles(files);
    (async () => {
      const next = [];
      let missing = false;
      for (const f of list) {
        const ok = await uploadExists(f);
        if (seq !== restoreSeq) return;
        if (!ok) {
          missing = true;
          continue;
        }
        const row = {
          path: f.path,
          mime: f.mime || "",
          name: f.name || f.path.split("/").pop() || "file",
          rel: f.path
        };
        row.url = uploadUrl(row);
        if (isImageAttach(row)) row.preview = row.url;
        next.push(row);
      }
      if (seq !== restoreSeq) return;
      for (const a of ctx.attachments || []) revokePreview(a);
      ctx.attachments = next;
      if (ctx.renderChips) ctx.renderChips();
      if (ctx.syncSendBtn) ctx.syncSendBtn();
      if (missing) toast(t("draftGone"));
      if (missing) publish();
    })();
  }

  function apply(text, files, ts) {
    const stamp = Number(ts) || 0;
    if (stamp && stamp < lastTs) return;
    lastTs = stamp || Date.now();
    const next = String(text || "");
    applying = true;
    if (promptApi.getText() !== next) promptApi.setText(next);
    restoreFiles(files);
    applying = false;
    if (ctx.paintPromptPh) ctx.paintPromptPh(false);
  }

  function publish() {
    if (applying) return;
    restoreSeq += 1;
    const text = promptApi.getText();
    const files = snapshotFiles();
    const ts = Date.now();
    lastTs = ts;
    const key = currentKey();
    storageSet(key, text, files, ts);
    if (bc) {
      try {
        bc.postMessage({ key, text, files, ts, tab: TAB });
      } catch (e) {
        /* closed */
      }
    }
  }

  function flush() {
    if (timer) {
      clearTimeout(timer);
      timer = 0;
    }
    publish();
  }

  function load() {
    const stored = storageGet(currentKey());
    lastTs = stored.ts || Date.now();
    applying = true;
    promptApi.setText(stored.text || "");
    applying = false;
    if (ctx.paintPromptPh) ctx.paintPromptPh(false);
    restoreFiles(stored.files);
  }

  function clear(id) {
    storageSet(keyFor(id || ctx.currentId), "", [], Date.now());
    storageSet(keyFor(""), "", [], Date.now());
    lastTs = Date.now();
  }

  function onRemote(key, text, files, ts, tab) {
    if (tab === TAB) return;
    if (key !== currentKey()) return;
    apply(text, files, ts);
  }

  if (bc) {
    bc.onmessage = (ev) => {
      const msg = ev && ev.data;
      if (!msg || typeof msg !== "object") return;
      onRemote(msg.key, msg.text, msg.files, msg.ts, msg.tab);
    };
  }

  window.addEventListener("storage", (ev) => {
    if (!ev.key || ev.key.indexOf(PREFIX) !== 0) return;
    const stored = storageGet(ev.key);
    onRemote(ev.key, stored.text, stored.files, stored.ts, "");
  });

  promptApi.onChange(() => {
    if (applying) return;
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => {
      timer = 0;
      publish();
    }, 50);
  });

  ctx.onCwdPicked = (path) => {
    if (ctx.currentId) return;
    flush();
    ctx.selectedCwd = path;
  };

  window.addEventListener("pagehide", flush);

  return { flush, load, clear, publish };
}
