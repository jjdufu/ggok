import { api } from "../lib/api.js";

function fmtVer(ver) {
  return String(ver || "").trim().replace(/^[vV]/, "");
}

export function bindVersion(ctx) {
  const curEl = document.getElementById("quota-ver-cur");
  const latestEl = document.getElementById("quota-ver-latest");

  function paint(st) {
    const ver = st && st.version;
    if (ver && curEl) curEl.textContent = fmtVer(ver);
    const latest = st && st.latest;
    if (latest && latestEl) latestEl.textContent = fmtVer(latest);
    if (latestEl) latestEl.classList.toggle("new", !!(st && st.update_available));
    const pop = document.getElementById("quota-pop");
    if (pop && !pop.hidden && ctx.placeQuotaPop) ctx.placeQuotaPop();
  }

  async function refreshVersion() {
    try {
      paint(await api("/api/version"));
    } catch {
      /* keep whatever is already on screen */
    }
  }

  ctx.refreshVersion = refreshVersion;
  refreshVersion();
}
