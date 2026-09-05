import { t, periodLabel, fmtReset, fmtResetDate, setTip } from "../lib/helpers.js";
import { emptyEl, kv, mkEl } from "../lib/dom.js";
import { placePopover } from "../lib/popover.js";
import { api } from "../lib/api.js";

const ACCOUNT_KEY = "ggok-account";

function readCachedAccount() {
  try {
    const raw = localStorage.getItem(ACCOUNT_KEY);
    if (!raw) return null;
    const st = JSON.parse(raw);
    if (!st || st.ok === false) return null;
    return st;
  } catch {
    return null;
  }
}

function writeCachedAccount(st) {
  if (!st || st.ok === false || st.used_percent == null) return;
  try {
    localStorage.setItem(ACCOUNT_KEY, JSON.stringify(st));
  } catch {
  }
}

function quotaRed(pct) {
  const n = Math.min(1, Math.max(0, (pct - 90) / 8));
  const r = Math.round(196 + 24 * n);
  const g = Math.round(92 * (1 - n));
  const b = Math.round(36 * (1 - n));
  return "rgb(" + r + ", " + g + ", " + b + ")";
}

export function bindQuota(ctx) {
  const btn = document.getElementById("quota-btn");
  const pop = document.getElementById("quota-pop");
  const body = document.getElementById("quota-pop-body");
  const pctEl = document.getElementById("quota-pct");
  const fill = document.getElementById("quota-fill");

  function placeQuotaPop() {
    if (!pop || !btn || pop.hidden) return;
    const r = btn.getBoundingClientRect();
    const gap = 8;
    const pad = 8;
    const collapsed = document.documentElement.dataset.sidebar === "collapsed";
    const mobile = window.matchMedia("(max-width: 900px)").matches;
    let width;
    let left;
    if (!collapsed || mobile) {
      const row = btn.closest(".foot-quota-row") || btn;
      const rr = row.getBoundingClientRect();
      width = rr.width;
      left = rr.left;
    } else {
      width = 240;
      left = r.right + gap;
      if (left + width + pad > window.innerWidth) {
        left = Math.max(pad, r.left - gap - width);
      }
    }
    placePopover(pop, btn, { gap, pad, minH: 96, width, left, zIndex: 40 });
  }

  function setQuotaOpen(on) {
    if (!pop || !btn) return;
    pop.hidden = !on;
    btn.classList.toggle("on", on);
    btn.setAttribute("aria-expanded", on ? "true" : "false");
    if (on) placeQuotaPop();
  }

  function usedPctOf(st) {
    if (!st || st.used_percent == null || st.used_percent === "") return null;
    const used = Number(st.used_percent);
    if (Number.isNaN(used)) return null;
    return Math.min(100, Math.max(0, used));
  }

  function applyQuotaTone(el, pct, has) {
    if (!el) return;
    el.classList.remove("warn");
    el.classList.toggle("hot", has && pct >= 90);
    el.classList.toggle("pulse", has && pct >= 98);
    if (has && pct >= 90) el.style.setProperty("--quota-color", quotaRed(pct));
    else el.style.removeProperty("--quota-color");
  }

  function syncChip(st) {
    const pct = usedPctOf(st);
    const has = pct != null;
    if (pctEl) pctEl.textContent = has ? Math.round(pct) + "%" : "—";
    if (fill) fill.style.width = has ? pct + "%" : "0%";
    if (!btn) return;
    applyQuotaTone(btn, pct, has);
    btn.style.setProperty("--quota-pct", String(has ? pct : 0));
    let tip = t("weeklyLimit");
    if (has && st) {
      const period = periodLabel(st.period);
      const reset = st.resets_at ? fmtReset(st.resets_at) : "";
      tip = period + " · " + Math.round(pct) + "%";
      if (reset) tip += " · " + reset;
    } else if (st && st.ok === false) {
      tip = st.error || t("couldntLoadUsage");
    }
    setTip(btn, tip);
    btn.setAttribute("aria-label", tip);
  }

  function renderAccount(st) {
    syncChip(st);
    if (!body) return;
    body.replaceChildren();
    if (!st) {
      body.appendChild(emptyEl("account-empty", t("loadingUsage")));
      return;
    }
    if (st.ok === false) {
      body.appendChild(emptyEl("account-empty", st.error || t("couldntLoadUsage")));
      return;
    }
    const plan = st.tier_label || st.tier || "";
    const email = st.email || "";
    if (plan || email) {
      const head = mkEl("div", "quota-head");
      if (plan) head.appendChild(mkEl("div", "quota-plan", plan));
      if (email) head.appendChild(mkEl("div", "quota-email", email));
      body.appendChild(head);
    }
    const rows = mkEl("div", "quota-rows");
    function addRow(k, v) {
      if (v == null || v === "") return;
      const row = mkEl("div", "quota-row");
      kv(row, "quota-row-k", "quota-row-v", k, v);
      rows.appendChild(row);
    }
    const used = usedPctOf(st);
    if (used != null) {
      const left = st.remaining_percent == null ? Math.max(0, 100 - used) : Number(st.remaining_percent);
      addRow(t("used"), Math.round(used) + "%");
      addRow(t("remaining"), Math.round(left) + "%");
    }
    const period = periodLabel(st.period);
    const date = st.resets_at ? fmtResetDate(st.resets_at) : "";
    if (date) addRow(t("resets"), date);
    else if (period) addRow(t("resets"), period);
    const products = st.products || [];
    for (const p of products) {
      addRow(p.product || "product", Math.round(Number(p.used_percent || 0)) + "%");
    }
    if (rows.childNodes.length) body.appendChild(rows);
    else if (!plan && !email) {
      body.appendChild(emptyEl("account-empty", t("loadingUsage")));
    }
    if (pop && !pop.hidden) placeQuotaPop();
  }

  async function refreshAccount() {
    try {
      const acc = await api("/api/account");
      if (ctx.applyAccount) ctx.applyAccount(acc);
      else {
        ctx.lastAccount = acc;
        writeCachedAccount(acc);
        renderAccount(acc);
      }
    } catch (e) {
      if (ctx.lastAccount && ctx.lastAccount.ok !== false && usedPctOf(ctx.lastAccount) != null) {
        renderAccount(ctx.lastAccount);
        return;
      }
      renderAccount({ ok: false, error: String(e.message || e) });
    }
  }

  function startQuotaPoll() {
    clearInterval(ctx.quotaTimer);
    ctx.quotaTimer = setInterval(refreshAccount, 60 * 1000);
  }

  if (btn) {
    btn.addEventListener("click", (e) => {
      e.stopPropagation();
      setQuotaOpen(pop && pop.hidden);
    });
  }
  if (pop) {
    pop.addEventListener("click", (e) => e.stopPropagation());
  }
  document.addEventListener("click", (e) => {
    if (!pop || pop.hidden) return;
    if (pop.contains(e.target) || (btn && btn.contains(e.target))) return;
    setQuotaOpen(false);
  });
  window.addEventListener("resize", () => {
    if (pop && !pop.hidden) placeQuotaPop();
  });

  ctx.setQuotaOpen = setQuotaOpen;
  ctx.renderAccount = renderAccount;
  ctx.refreshAccount = refreshAccount;
  ctx.startQuotaPoll = startQuotaPoll;
  ctx.writeCachedAccount = writeCachedAccount;

  if (!ctx.lastAccount) {
    const cached = readCachedAccount();
    if (cached) ctx.lastAccount = cached;
  }
  renderAccount(ctx.lastAccount);
  refreshAccount();
  startQuotaPoll();
}
