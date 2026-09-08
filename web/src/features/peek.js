import { t } from "../lib/helpers.js";
import { api } from "../lib/api.js";
import { toast } from "../lib/clipboard.js";
import { openOverlay, closeOverlay } from "../lib/overlay.js";

export function bindPeek(ctx) {
  const scrim = document.getElementById("peek-scrim");
  const panel = document.getElementById("peek-panel");
  const titleEl = document.getElementById("peek-title");
  const host = document.getElementById("peek-timeline");
  const closeBtn = document.getElementById("peek-close");

  function stopPeekPoll() {
    if (ctx.peekTimer) {
      clearInterval(ctx.peekTimer);
      ctx.peekTimer = 0;
    }
  }

  function closePeek() {
    stopPeekPoll();
    ctx.peekId = "";
    document.removeEventListener("keydown", onPeekKey, true);
    closeOverlay(scrim, { panel });
  }

  function onPeekKey(e) {
    if (e.key !== "Escape") return;
    e.preventDefault();
    e.stopPropagation();
    closePeek();
  }

  function renderPeekDetail(detail) {
    if (!host) return;
    host.replaceChildren();
    const turns = ctx.groupTurns ? ctx.groupTurns(detail.blocks || []) : [];
    if (!turns.length) {
      const empty = document.createElement("p");
      empty.className = "muted";
      empty.setAttribute("data-i18n", "peekEmpty");
      empty.textContent = t("peekEmpty");
      host.appendChild(empty);
      return;
    }
    for (const turn of turns) {
      if (ctx.renderTurnArticle) {
        host.appendChild(ctx.renderTurnArticle(turn, false, { readonly: true }));
      }
    }
  }

  async function loadPeek(id) {
    const detail = await api("/api/sessions/" + encodeURIComponent(id));
    if (ctx.peekId !== id) return;
    if (titleEl && !titleEl.dataset.locked) {
      titleEl.textContent = (detail && detail.title) || t("peekTitle");
    }
    renderPeekDetail(detail || { blocks: [] });
    return detail;
  }

  async function openPeek(id, label) {
    const peekId = String(id || "").trim();
    if (!peekId) return;
    ctx.peekId = peekId;
    stopPeekPoll();
    if (titleEl) {
      titleEl.textContent = label || t("peekTitle");
      titleEl.dataset.locked = label ? "1" : "";
      if (!label) delete titleEl.dataset.locked;
    }
    if (host) host.replaceChildren();
    openOverlay(scrim, panel);
    document.addEventListener("keydown", onPeekKey, true);
    try {
      const detail = await loadPeek(peekId);
      if (detail && detail.running) {
        ctx.peekTimer = setInterval(() => {
          loadPeek(peekId).catch(() => {});
        }, 2000);
      }
    } catch (e) {
      toast(t("peekFailed"));
      closePeek();
    }
  }

  if (closeBtn) closeBtn.addEventListener("click", closePeek);
  if (scrim) scrim.addEventListener("click", closePeek);
  if (panel) {
    panel.addEventListener("click", (e) => e.stopPropagation());
  }

  ctx.openPeek = openPeek;
  ctx.closePeek = closePeek;
}
