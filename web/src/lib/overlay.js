const pending = new WeakMap();

function overlayDuration() {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches ? 0 : 200;
}

function clearPending(el) {
  const rec = pending.get(el);
  if (!rec) return;
  if (rec.timeout) clearTimeout(rec.timeout);
  if (rec.onEnd) el.removeEventListener("animationend", rec.onEnd);
  pending.delete(el);
}

function show(el) {
  if (!el) return;
  clearPending(el);
  el.setAttribute("data-state", "open");
  el.hidden = false;
}

function hide(el, opts) {
  if (!el) return;
  const onDone = opts && opts.onDone;
  clearPending(el);
  const finish = () => {
    clearPending(el);
    if (el.getAttribute("data-state") === "open") return;
    el.hidden = true;
    el.removeAttribute("data-state");
    if (typeof onDone === "function") onDone();
  };
  el.setAttribute("data-state", "closed");
  if (el.hidden || overlayDuration() === 0) {
    finish();
    return;
  }
  const onEnd = (e) => {
    if (e.target !== el) return;
    if (el.getAttribute("data-state") !== "closed") return;
    finish();
  };
  el.addEventListener("animationend", onEnd);
  pending.set(el, {
    onEnd,
    timeout: setTimeout(finish, overlayDuration() + 80)
  });
}

export function openOverlay(el, panel) {
  show(el);
  if (panel) show(panel);
}

export function closeOverlay(el, opts) {
  const o = opts || {};
  hide(el, { onDone: o.panel ? undefined : o.onDone });
  if (o.panel) hide(o.panel, { onDone: o.onDone });
}
