import { setTip, hideTip } from "./helpers.js";
import { svgUse } from "./svg.js";

export function mkEl(tag, cls, text) {
  const el = document.createElement(tag);
  if (cls) el.className = cls;
  if (text != null && text !== "") el.textContent = text;
  return el;
}

export function emptyEl(cls, text) {
  return mkEl("div", cls, text);
}

export function kv(parent, kClass, vClass, k, v) {
  parent.append(
    mkEl("span", kClass, k),
    mkEl("span", vClass, v == null || v === "" ? "—" : String(v))
  );
}

export function iconAct({ icon, className, tip, onClick, clearTip }) {
  const b = mkEl("button", className);
  b.type = "button";
  if (icon) b.appendChild(svgUse(icon));
  if (tip) {
    setTip(b, tip);
    b.setAttribute("aria-label", tip);
  }
  b.addEventListener("click", (e) => {
    e.preventDefault();
    e.stopPropagation();
    if (clearTip) hideTip(false);
    onClick(e);
  });
  return b;
}

export function menuButton({ icon, label, className, onClick }) {
  const b = document.createElement("button");
  b.type = "button";
  b.setAttribute("role", "menuitem");
  if (className) b.className = className;
  if (icon) b.appendChild(svgUse(icon));
  b.appendChild(mkEl("span", "", label));
  b.addEventListener("click", (e) => {
    e.preventDefault();
    e.stopPropagation();
    onClick(e);
  });
  return b;
}
