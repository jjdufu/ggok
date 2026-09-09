import { api } from "../lib/api.js";

function fmtVer(ver) {
  return String(ver || "").trim().replace(/^[vV]/, "");
}

export function bindVersion(ctx) {
  const curEl = document.getElementById("quota-ver-cur");
  const latestEl = document.getElementById("quota-ver-latest");

  function paint(st) {
    const ver = st && st.version;
    const cur = ver ? fmtVer(ver) : "";
    if (cur && curEl) curEl.textContent = cur;
    const latestRaw = st && st.latest;
    const latest = latestRaw ? fmtVer(latestRaw) : "";
    if (latestEl) {
      latestEl.textContent = latest || cur;
      latestEl.classList.toggle("new", !!(st && st.update_available));
    }
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
