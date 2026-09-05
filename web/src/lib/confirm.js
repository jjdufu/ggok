import { t } from "./helpers.js";
import { openOverlay, closeOverlay } from "./overlay.js";

export function bindConfirm(ctx) {
  const el = document.getElementById("app-confirm");
  const title = document.getElementById("confirm-title");
  const body = document.getElementById("confirm-body");
  const cancel = document.getElementById("confirm-cancel");
  const okBtn = document.getElementById("confirm-ok");
  const card = el && el.querySelector(".ext-confirm-card");
  const home = el && el.parentNode;
  let onOk = null;
  let dismissOnScrim = false;

  function confirmOpen() {
    return !!(el && !el.hidden);
  }

  function closeConfirm() {
    if (!el) return;
    onOk = null;
    dismissOnScrim = false;
    closeOverlay(el, {
      onDone: () => {
        if (home && el.parentNode !== home) home.appendChild(el);
        el.classList.add("sess-confirm");
        if (okBtn) okBtn.classList.remove("danger");
      }
    });
  }

  function openConfirm(opts) {
    if (!el) return;
    const o = opts || {};
    onOk = typeof o.onOk === "function" ? o.onOk : null;
    dismissOnScrim = !!o.dismissOnScrim;
    if (title) title.textContent = o.title || "";
    if (body) body.textContent = o.body || "";
    if (okBtn) {
      okBtn.textContent = o.ok || t("confirmDelete");
      okBtn.classList.toggle("danger", !!o.danger);
    }
    if (cancel) {
      cancel.setAttribute("data-i18n", "cancel");
      cancel.textContent = t("cancel");
    }
    if (o.host) {
      el.classList.remove("sess-confirm");
      o.host.appendChild(el);
    } else if (home && el.parentNode !== home) {
      el.classList.add("sess-confirm");
      home.appendChild(el);
    } else {
      el.classList.add("sess-confirm");
    }
    openOverlay(el);
  }

  if (el) {
    el.addEventListener("click", (e) => {
      e.stopPropagation();
      if (dismissOnScrim) closeConfirm();
    });
  }
  if (card) {
    card.addEventListener("click", (e) => e.stopPropagation());
  }
  if (cancel) {
    cancel.addEventListener("click", (e) => {
      e.stopPropagation();
      closeConfirm();
    });
  }
  if (okBtn) {
    okBtn.addEventListener("click", (e) => {
      e.stopPropagation();
      const fn = onOk;
      closeConfirm();
      if (typeof fn === "function") fn();
    });
  }

  ctx.confirmOpen = confirmOpen;
  ctx.openConfirm = openConfirm;
  ctx.closeConfirm = closeConfirm;
}
