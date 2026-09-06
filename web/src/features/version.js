import { setTip } from "../lib/helpers.js";
import { api } from "../lib/api.js";

const LATEST_RELEASE = "https://github.com/jjdufu/ggok/releases/latest";

export function bindVersion(ctx) {
  const num = document.getElementById("quota-ver-num");
  const btn = document.getElementById("quota-ver-upd");
  const quotaBtn = document.getElementById("quota-btn");
  let lastVersion = "";

  function paint(st) {
    const ver = (st && st.version) || "";
    if (ver) lastVersion = ver;
    if (num) num.textContent = ver;
    const show = !!(st && st.update_available === true && ver);
    if (btn) {
      btn.hidden = !show;
      if (show) {
        setTip(btn, ver);
        btn.setAttribute("aria-label", ver);
      } else {
        setTip(btn, "");
        btn.setAttribute("aria-label", "");
      }
    }
    const pop = document.getElementById("quota-pop");
    if (pop && !pop.hidden && ctx.placeQuotaPop) ctx.placeQuotaPop();
  }

  async function refreshVersion() {
    try {
      paint(await api("/api/version"));
    } catch {
      paint({ version: lastVersion || "", update_available: false });
    }
  }

  if (btn) {
    btn.addEventListener("click", (e) => {
      e.preventDefault();
      e.stopPropagation();
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
