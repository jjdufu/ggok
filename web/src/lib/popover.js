export function placePopover(el, anchor, opts = {}) {
  if (!el || el.hidden || !anchor) return;
  const r = anchor instanceof Element ? anchor.getBoundingClientRect() : anchor;
  const gap = opts.gap ?? 8;
  const pad = opts.pad ?? 12;
  const minH = opts.minH ?? 120;
  const cap = opts.maxH;
  const vw = window.innerWidth;
  const vh = window.innerHeight;

  el.style.position = "fixed";
  if (opts.zIndex != null) el.style.zIndex = String(opts.zIndex);
  el.style.right = "auto";
  el.style.maxHeight = "none";

  let width = opts.width;
  if (width === "anchor") width = r.width;
  if (typeof width === "number" && width > 0) {
    width = Math.min(width, Math.max(0, vw - pad * 2));
    el.style.width = Math.round(width) + "px";
  }

  const boxW = el.offsetWidth;
  let left;
  if (typeof opts.left === "number") left = opts.left;
  else if (opts.align === "right") left = r.right - boxW;
  else left = r.left;
  left = Math.min(Math.max(pad, left), Math.max(pad, vw - pad - boxW));
  el.style.left = Math.round(left) + "px";

  const popH = el.scrollHeight;
  const desired = cap ? Math.min(popH, cap) : popH;
  const spaceAbove = Math.max(0, r.top - pad);
  const spaceBelow = Math.max(0, vh - r.bottom - pad);
  const need = desired + gap;
  const canAbove = need <= spaceAbove;
  const canBelow = need <= spaceBelow;

  let mode;
  if (canAbove || canBelow) {
    if (canAbove && canBelow) mode = spaceAbove >= spaceBelow ? "up" : "down";
    else mode = canBelow ? "down" : "up";
  } else {
    mode = "free";
  }

  if (mode === "free") {
    const maxH = Math.max(minH, vh - pad * 2);
    el.style.maxHeight = Math.round(maxH) + "px";
    el.style.overflowY = popH > maxH ? "auto" : "";
    const h = Math.min(popH, maxH);
    let top = Math.round((vh - h) / 2);
    top = Math.min(Math.max(pad, top), Math.max(pad, vh - pad - h));
    el.style.top = top + "px";
    el.style.bottom = "auto";
    return;
  }

  let sideAvail = (mode === "down" ? spaceBelow : spaceAbove) - gap;
  if (cap) sideAvail = Math.min(sideAvail, cap);
  sideAvail = Math.max(minH, sideAvail);
  if (popH > sideAvail) {
    el.style.maxHeight = Math.round(sideAvail) + "px";
    el.style.overflowY = "auto";
  } else {
    el.style.maxHeight = "";
    el.style.overflowY = "";
  }
  if (mode === "down") {
    el.style.top = Math.round(r.bottom + gap) + "px";
    el.style.bottom = "auto";
  } else {
    el.style.top = "auto";
    el.style.bottom = Math.round(vh - r.top + gap) + "px";
  }
}
