import { api } from "../lib/api.js";

const LATEST_RELEASE = "https://github.com/jjdufu/ggok/releases/latest";

function fmtVer(ver) {
  return String(ver || "").trim().replace(/^[vV]/, "");
}

export function bindVersion(ctx) {
  const curEl = document.getElementById("quota-ver-cur");
  const latestBtn = document.getElementById("quota-ver-latest");
  const quotaBtn = document.getElementById("quota-btn");
  let lastLatest = "";

  function paint(st) {
    const ver = st && st.version;
    if (ver && curEl) curEl.textContent = fmtVer(ver);
    const latest = st && st.latest;
    if (latest && latestBtn) {
      lastLatest = String(latest);
      latestBtn.textContent = fmtVer(lastLatest);
      latestBtn.classList.add("has");
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

  if (latestBtn) {
    latestBtn.addEventListener("click", (e) => {
      e.preventDefault();
      e.stopPropagation();
      if (!lastLatest) return;
      window.open(LATEST_RELEASE, "_blank", "noopener,noreferrer");
    });
  }
  if (quotaBtn) {
    quotaBtn.addEventListener("click", () => {
      const pop = document.getElementById("quota-pop");
      if (pop && !pop.hidden) refreshVersion();
    });
  }

  refreshVersion();
  setTimeout(refreshVersion, 2000);
}
