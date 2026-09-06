import { promptApi } from "../promptApi.js";

const PREFIX = "ggok-draft:";
const CHANNEL = "ggok-draft";
const TAB =
  (typeof crypto !== "undefined" && crypto.randomUUID && crypto.randomUUID()) ||
  `${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
const MAX = 200 * 1024;

function storageGet(key) {
  try {
    const raw = localStorage.getItem(key);
    if (!raw) return { text: "", ts: 0 };
    const v = JSON.parse(raw);
    if (!v || typeof v.text !== "string") return { text: "", ts: 0 };
    return { text: v.text, ts: Number(v.ts) || 0 };
  } catch (e) {
    return { text: "", ts: 0 };
  }
}

function storageSet(key, text, ts) {
  try {
    if (!text) {
      localStorage.removeItem(key);
      return;
    }
    if (text.length > MAX) return;
    localStorage.setItem(key, JSON.stringify({ text, ts }));
  } catch (e) {
    /* quota / private mode */
  }
}

export function bindDraftSync(ctx) {
  let applying = false;
  let timer = 0;
  let lastTs = 0;
  let bc = null;
  try {
    if (typeof BroadcastChannel === "function") bc = new BroadcastChannel(CHANNEL);
  } catch (e) {
    bc = null;
  }

  function keyFor(id, cwd) {
    if (id) return PREFIX + id;
    return PREFIX + "new:" + String(cwd || "");
  }

  function currentKey() {
    const cwd = ctx.selectedCwd || (ctx.current && ctx.current.cwd) || "";
    return keyFor(ctx.currentId, cwd);
  }

  function apply(text, ts) {
    const stamp = Number(ts) || 0;
    if (stamp && stamp < lastTs) return;
    lastTs = stamp || Date.now();
    const next = String(text || "");
    if (promptApi.getText() === next) return;
    applying = true;
    promptApi.setText(next);
    applying = false;
    if (ctx.paintPromptPh) ctx.paintPromptPh(false);
  }

  function publish() {
    if (applying) return;
    const text = promptApi.getText();
    const ts = Date.now();
    lastTs = ts;
    const key = currentKey();
    storageSet(key, text, ts);
    if (bc) {
      try {
        bc.postMessage({ key, text, ts, tab: TAB });
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
  }

  function clear(id) {
    const cwd = ctx.selectedCwd || (ctx.current && ctx.current.cwd) || "";
    storageSet(keyFor(id || ctx.currentId, cwd), "", Date.now());
    storageSet(keyFor("", cwd), "", Date.now());
    lastTs = Date.now();
  }

  function onRemote(key, text, ts, tab) {
    if (tab === TAB) return;
    if (key !== currentKey()) return;
    apply(text, ts);
  }

  if (bc) {
    bc.onmessage = (ev) => {
      const msg = ev && ev.data;
      if (!msg || typeof msg !== "object") return;
      onRemote(msg.key, msg.text, msg.ts, msg.tab);
    };
  }

  window.addEventListener("storage", (ev) => {
    if (!ev.key || ev.key.indexOf(PREFIX) !== 0) return;
    const stored = storageGet(ev.key);
    onRemote(ev.key, stored.text, stored.ts, "");
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
    load();
  };

  window.addEventListener("pagehide", flush);

  return { flush, load, clear, publish };
}
