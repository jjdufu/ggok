import { t } from "../lib/helpers.js";
import { api, post } from "../lib/api.js";
import { toast } from "../lib/clipboard.js";
import { openOverlay, closeOverlay } from "../lib/overlay.js";

function userIndex(blocks, promptId) {
  let i = 0;
  for (const b of blocks || []) {
    if (b.type !== "user") continue;
    if (String(b.prompt_id || "") === String(promptId || "")) return i;
    i += 1;
  }
  return -1;
}

export function bindRewind(ctx) {
  function closeRewind() {
    const scrim = document.getElementById("rewind-scrim");
    const panel = document.getElementById("rewind-panel");
    closeOverlay(scrim, { panel });
  }

  function renderPoints(points) {
    const list = document.getElementById("rewind-list");
    if (!list) return;
    list.replaceChildren();
    (points || []).forEach((p) => {
      const btn = document.createElement("button");
      btn.type = "button";
      btn.className = "rewind-item" + (p.disabled ? " disabled" : "");
      btn.disabled = !!p.disabled;
      const idx = document.createElement("span");
      idx.className = "rewind-idx";
      idx.textContent = "#" + p.prompt_index;
      const prev = document.createElement("span");
      prev.className = "rewind-preview";
      prev.textContent = p.preview || "";
      const meta = document.createElement("span");
      meta.className = "rewind-meta";
      meta.textContent = p.disabled && p.reason ? p.reason : p.created_at || "";
      btn.append(idx, prev, meta);
      btn.addEventListener("click", () => {
        if (p.disabled) return;
        confirmRewind(p.prompt_index);
      });
      list.appendChild(btn);
    });
    if (!(points || []).length) {
      const empty = document.createElement("p");
      empty.className = "muted";
      empty.textContent = t("rewindEmpty");
      list.appendChild(empty);
    }
  }

  async function openRewind() {
    if (!ctx.currentId) {
      toast(t("rewindNeedSession"));
      return;
    }
    try {
      const data = await api("/api/sessions/" + encodeURIComponent(ctx.currentId) + "/rewind/points");
      const scrim = document.getElementById("rewind-scrim");
      const panel = document.getElementById("rewind-panel");
      openOverlay(scrim, panel);
      renderPoints(data && data.points);
    } catch (e) {
      toast(String(e.message || e));
    }
  }

  async function confirmRewind(promptIndex) {
    if (!window.confirm(t("rewindConfirm"))) return;
    closeRewind();
    try {
      await post("/api/sessions/" + encodeURIComponent(ctx.currentId) + "/rewind", {
        prompt_index: promptIndex
      });
      toast(t("rewindDone"));
      if (ctx.pullSession) await ctx.pullSession(ctx.currentId);
    } catch (e) {
      toast(String(e.message || e));
    }
  }

  async function rewindToPrompt(promptId) {
    const idx = userIndex(ctx.current && ctx.current.blocks, promptId);
    if (idx < 0) {
      toast(t("rewindMissing"));
      return;
    }
    await confirmRewind(idx);
  }

  async function retryFromPrompt(promptId, text) {
    const idx = userIndex(ctx.current && ctx.current.blocks, promptId);
    if (idx < 0) {
      toast(t("rewindMissing"));
      return;
    }
    if (!window.confirm(t("retryConfirm"))) return;
    ctx.retryPending = null;
    try {
      await post("/api/sessions/" + encodeURIComponent(ctx.currentId) + "/retry", {
        prompt_index: idx,
        text: text || undefined
      });
      if (ctx.pullSession) await ctx.pullSession(ctx.currentId);
    } catch (e) {
      toast(String(e.message || e));
    }
  }

  function editThenRetry(block) {
    ctx.retryPending = {
      prompt_index: userIndex(ctx.current && ctx.current.blocks, block.prompt_id),
      text: String(block.text || "")
    };
    if (ctx.fillComposer) ctx.fillComposer(block.text);
    toast(t("retryEditHint"));
  }

  const scrim = document.getElementById("rewind-scrim");
  if (scrim) scrim.addEventListener("click", closeRewind);
  const closeBtn = document.getElementById("rewind-close");
  if (closeBtn) closeBtn.addEventListener("click", closeRewind);

  ctx.openRewind = openRewind;
  ctx.closeRewind = closeRewind;
  ctx.rewindToPrompt = rewindToPrompt;
  ctx.retryFromPrompt = retryFromPrompt;
  ctx.editThenRetry = editThenRetry;
  ctx.userPromptIndex = userIndex;
}
