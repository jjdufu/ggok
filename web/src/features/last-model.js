import { post } from "../lib/api.js";

const KEY = "ggok-last-model";

function readStored() {
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return null;
    const v = JSON.parse(raw);
    if (!v || typeof v.model !== "string" || !v.model.trim()) return null;
    return { model: v.model, effort: typeof v.effort === "string" ? v.effort : "" };
  } catch (e) {
    return null;
  }
}

function writeStored(model, effort) {
  try {
    localStorage.setItem(KEY, JSON.stringify({ model, effort: effort || "", ts: Date.now() }));
  } catch (e) {
    /* quota / private mode */
  }
}

export function bindLastModel(ctx) {
  const stored = readStored();
  if (stored) {
    ctx.selectedModel = stored.model;
    if (stored.effort) ctx.selectedEffort = stored.effort;
  }

  ctx.applyRuntimeLastModel = (rt) => {
    if (readStored()) return;
    if (!rt || !rt.last_model) return;
    ctx.selectedModel = rt.last_model;
    if (rt.last_effort) ctx.selectedEffort = rt.last_effort;
  };

  ctx.persistLastModel = (model, effort) => {
    const id = String(model || "").trim();
    if (!id) return;
    const next = String(effort || "");
    writeStored(id, next);
    post("/api/prefs/model", { model: id, effort: next || undefined }).catch(() => {});
  };
}
