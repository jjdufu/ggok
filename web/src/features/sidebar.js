import { t, hideTip, setTip, shortCwd, fmtNum, fmtTok } from "../lib/helpers.js";
import { menuButton } from "../lib/dom.js";
import { beginInlineRename } from "../lib/rename.js";
import { svgUse } from "../lib/svg.js";
import { api, patch, del } from "../lib/api.js";
import { toast } from "../lib/clipboard.js";
import { openOverlay, closeOverlay } from "../lib/overlay.js";

export function bindSidebar(ctx) {
  const { SIDE_KEY } = ctx;
  const tree = document.getElementById("tree");
  const app = document.getElementById("app");
  const scrim = document.getElementById("scrim");
  const sessMenu = document.getElementById("sess-menu");
  const ctxBar = document.getElementById("ctx-bar");
  const ctxFill = document.getElementById("ctx-fill");

  function setSidebarCollapsed(on) {
    document.documentElement.dataset.sidebar = on ? "collapsed" : "";
    localStorage.setItem(SIDE_KEY, on ? "collapsed" : "open");
  }

  function closeMobile() {
    if (app) app.classList.remove("mobile-open");
    closeOverlay(scrim);
  }

  function openMobile() {
    if (app) app.classList.add("mobile-open");
    openOverlay(scrim);
  }

  const collapseSideBtn = document.getElementById("collapse-side");
  function syncCollapseTip() {
    if (!collapseSideBtn) return;
    const collapsed = document.documentElement.dataset.sidebar === "collapsed";
    const key = collapsed ? "expandSidebar" : "collapseSidebar";
    collapseSideBtn.setAttribute("data-i18n-title", key);
    setTip(collapseSideBtn, t(key));
  }
  if (collapseSideBtn) {
    collapseSideBtn.addEventListener("click", () => {
      if (window.matchMedia("(max-width: 900px)").matches) {
        closeMobile();
        return;
      }
      setSidebarCollapsed(document.documentElement.dataset.sidebar !== "collapsed");
      syncCollapseTip();
    });
  }
  syncCollapseTip();

  const openSideBtn = document.getElementById("open-side");
  if (openSideBtn) {
    openSideBtn.addEventListener("click", openMobile);
  }

  if (scrim) {
    scrim.addEventListener("click", closeMobile);
  }

  function groupTree(list) {
    const byCwd = new Map();
    for (const s of list) {
      if (!byCwd.has(s.cwd)) byCwd.set(s.cwd, []);
      byCwd.get(s.cwd).push(s);
    }
    const projects = [...byCwd.entries()].sort((a, b) => {
      const ua = a[1].reduce((m, s) => (s.updated_at > m ? s.updated_at : m), "");
      const ub = b[1].reduce((m, s) => (s.updated_at > m ? s.updated_at : m), "");
      return ub.localeCompare(ua);
    });
    return projects.map(([cwd, items]) => {
      const sess = [...items].sort((a, b) => b.updated_at.localeCompare(a.updated_at));
      return { cwd, sessions: sess };
    });
  }

  function closeSessMenu() {
    if (sessMenu) sessMenu.hidden = true;
    if (tree) {
      tree.querySelectorAll(".sess.menu-on").forEach((el) => el.classList.remove("menu-on"));
    }
  }

  function applySessionTitle(id, title) {
    const row = ctx.sessions.find((s) => s.id === id);
    if (row) row.title = title;
    if (ctx.current && ctx.current.id === id) {
      ctx.current.title = title;
      if (ctx.setPageTitle) ctx.setPageTitle(title);
      else document.title = title + " · GGOK";
    }
    if (ctx.finderCache && ctx.finderCache[id]) ctx.finderCache[id].title = title;
    renderTree();
    if (ctx.finderOpen && ctx.finderOpen()) ctx.renderFinder();
  }

  function applySessionPinned(id, pinned) {
    const row = ctx.sessions.find((s) => s.id === id);
    if (row) row.pinned = !!pinned;
    renderTree();
    if (ctx.finderOpen && ctx.finderOpen()) ctx.renderFinder();
  }

  async function saveSessionTitle(id, title) {
    const next = String(title || "").trim();
    if (!next) return false;
    const row = ctx.sessions.find((s) => s.id === id);
    if (row && row.title === next) return true;
    try {
      const out = await patch("/api/sessions/" + encodeURIComponent(id), { title: next });
      applySessionTitle(id, (out && out.title) || next);
      return true;
    } catch (e) {
      toast(String(e.message || e));
      return false;
    }
  }

  async function toggleSessionPin(id) {
    const row = ctx.sessions.find((s) => s.id === id);
    const next = !(row && row.pinned);
    try {
      const out = await patch("/api/sessions/" + encodeURIComponent(id), { pinned: next });
      applySessionPinned(id, out && out.pinned);
    } catch (e) {
      toast(String(e.message || e));
    }
  }

  async function deleteSession(id) {
    try {
      await del("/api/sessions/" + encodeURIComponent(id));
      if (ctx.finderCache) delete ctx.finderCache[id];
      if (ctx.currentId === id && ctx.leaveSession) ctx.leaveSession();
      if (ctx.loadList) await ctx.loadList();
      if (ctx.finderOpen && ctx.finderOpen()) {
        if (ctx.finderSel === id) ctx.finderSel = ctx.currentId || "__new__";
        ctx.renderFinder();
      }
    } catch (e) {
      toast(String(e.message || e));
    }
  }

  function askDeleteSession(id) {
    const row = ctx.sessions.find((s) => s.id === id);
    if (!row) return;
    if (ctx.openConfirm) {
      ctx.openConfirm({
        title: t("deleteChatTitle"),
        body: t("deleteChatBody"),
        ok: t("confirmDelete"),
        danger: true,
        onOk: () => deleteSession(id)
      });
    }
  }

  function sessionById(id) {
    return ctx.sessions.find((s) => s.id === id);
  }

  function openSessMenu(btn, s) {
    if (!sessMenu || !s) return;
    const already = !sessMenu.hidden && sessMenu.dataset.id === s.id;
    closeSessMenu();
    if (already) return;
    sessMenu.dataset.id = s.id;
    sessMenu.replaceChildren();
    const item = (icon, label, cls, onClick) => {
      sessMenu.appendChild(
        menuButton({
          icon,
          label,
          className: cls,
          onClick: () => {
            closeSessMenu();
            onClick();
          }
        })
      );
    };
    item("i-edit", t("rename"), "", () => {
      const row = tree.querySelector('.sess[data-id="' + CSS.escape(s.id) + '"]');
      const name = row && row.querySelector(".name");
      if (!name) return;
      ctx.sessRenaming = s.id;
      beginInlineRename(name, {
        value: s.title || s.id,
        inputClass: "sess-rename",
        onCommit: (v) => saveSessionTitle(s.id, v),
        onRestore: () => {
          ctx.sessRenaming = "";
          renderTree();
        }
      });
    });
    item(s.pinned ? "i-pin" : "i-pin", s.pinned ? t("unpin") : t("pin"), "", () => toggleSessionPin(s.id));
    item("i-trash", t("delete"), "danger", () => askDeleteSession(s.id));
    sessMenu.hidden = false;
    btn.closest(".sess") && btn.closest(".sess").classList.add("menu-on");
    const r = btn.getBoundingClientRect();
    const sidebar = btn.closest("#sidebar") || document.getElementById("sidebar");
    const sidebarRight = sidebar ? sidebar.getBoundingClientRect().right : r.right;
    const mw = sessMenu.offsetWidth || 184;
    const mh = sessMenu.offsetHeight || 128;
    let left = sidebarRight + 6;
    let top = Math.max(8, r.top - 4);
    if (left + mw > window.innerWidth - 8) {
      left = Math.max(8, r.left - mw - 6);
    }
    if (top + mh > window.innerHeight - 8) {
      top = Math.max(8, window.innerHeight - 8 - mh);
    }
    sessMenu.style.left = left + "px";
    sessMenu.style.top = top + "px";
  }

  function renderTree() {
    if (!tree) return;
    hideTip(false);
    closeSessMenu();
    tree.innerHTML = "";
    const addSess = (listEl, s) => {
      const b = document.createElement("div");
      b.className = "sess" + (s.id === ctx.currentId ? " active" : "") + (s.running ? " running" : "");
      b.dataset.id = s.id;
      b.setAttribute("role", "button");
      b.tabIndex = 0;
      const label = s.title || s.id;
      b.dataset.tip = label;
      b.setAttribute("aria-label", label);
      const name = document.createElement("span");
      name.className = "name";
      name.textContent = label;
      b.appendChild(name);
      if (s.source === "tui") {
        const tag = document.createElement("span");
        tag.className = "sess-tui";
        tag.textContent = "TUI";
        b.appendChild(tag);
      }
      const more = document.createElement("button");
      more.type = "button";
      more.className = "sess-more";
      more.setAttribute("aria-label", t("sessionMenu"));
      more.appendChild(svgUse("i-dots"));
      more.addEventListener("click", (e) => {
        e.preventDefault();
        e.stopPropagation();
        hideTip(false);
        openSessMenu(more, s);
      });
      b.appendChild(more);
      b.addEventListener("click", () => {
        if (ctx.sessRenaming === s.id) return;
        hideTip(false);
        closeSessMenu();
        closeMobile();
        if (ctx.openSession) ctx.openSession(s.id);
      });
      listEl.appendChild(b);
    };
    const addGroup = (title, tip, items, i18nKey) => {
      if (!items.length) return;
      const wrap = document.createElement("div");
      wrap.className = "proj";
      const tog = document.createElement("button");
      tog.type = "button";
      tog.className = "proj-toggle";
      tog.textContent = title;
      if (i18nKey) tog.setAttribute("data-i18n", i18nKey);
      if (tip) tog.dataset.tip = tip;
      const list = document.createElement("div");
      tog.addEventListener("click", () => {
        list.hidden = !list.hidden;
        wrap.classList.toggle("closed", list.hidden);
      });
      wrap.appendChild(tog);
      wrap.appendChild(list);
      items.forEach((s) => addSess(list, s));
      tree.appendChild(wrap);
    };
    const pinned = ctx.sessions.filter((s) => s.pinned);
    addGroup(t("pinnedGroup"), "", pinned, "pinnedGroup");
    const rest = ctx.sessions.filter((s) => !s.pinned);
    const grouped = groupTree(rest);
    for (const g of grouped) addGroup(shortCwd(g.cwd), g.cwd, g.sessions);
    if (!pinned.length && !grouped.length) {
      const empty = document.createElement("p");
      empty.className = "muted";
      empty.style.padding = "12px 16px";
      empty.textContent = t("noSessions");
      empty.setAttribute("data-i18n", "noSessions");
      tree.appendChild(empty);
    }
  }

  async function loadList() {
    const data = await api("/api/sessions");
    ctx.sessions = Array.isArray(data) ? data : [];
    renderTree();
    if (ctx.finderOpen && ctx.finderOpen()) ctx.renderFinder();
  }

  document.addEventListener("click", (e) => {
    if (sessMenu && !sessMenu.hidden && !sessMenu.contains(e.target)) {
      closeSessMenu();
    }
  });

  function contextOf(src) {
    const c = src || (ctx.current && ctx.current.context) || {};
    const used = Number(c.used || 0);
    let window = Number(c.window || 0);
    if (!window) {
      const id = (ctx.current && ctx.current.model) || ctx.selectedModel || "";
      const m = ctx.modelById ? ctx.modelById(id) : null;
      window = Number((m && m.context_window) || 0) || 500000;
    }
    return { used, window };
  }

  function applyContext(src) {
    const { used, window } = contextOf(src);
    if (ctx.current) ctx.current.context = { used, window };
    const pct = window > 0 ? Math.min(100, Math.round((used / window) * 100)) : 0;
    const shouldShow = Boolean(ctx.currentId) && used > 0;
    if (ctxBar) {
      ctxBar.hidden = !shouldShow;
      ctxBar.classList.toggle("warn", pct >= 60 && pct < 80);
      ctxBar.classList.toggle("hot", pct >= 80);
      const usedK = Math.round(used / 1000);
      const winK = Math.round(window / 1000);
      const ctxTip = pct + "% · " + usedK + "/" + winK + "K";
      ctxBar.removeAttribute("data-tip");
      ctxBar.removeAttribute("data-i18n-title");
      ctxBar.setAttribute("aria-label", ctxTip);
      ctxBar.style.removeProperty("--ctx-pct");
      const label = document.getElementById("ctx-label");
      if (label) label.textContent = ctxTip;
    }
    if (ctxFill) {
      ctxFill.style.width = pct + "%";
      let hue;
      if (pct <= 50) {
        hue = 142 - (pct / 50) * (142 - 45);
      } else {
        hue = 45 - ((pct - 50) / 50) * (45 - 10);
      }
      ctxFill.style.backgroundColor = `hsl(${Math.round(hue)}, 85%, 48%)`;
    }
  }

  ctx.setSidebarCollapsed = setSidebarCollapsed;
  ctx.closeMobile = closeMobile;
  ctx.openMobile = openMobile;
  ctx.groupTree = groupTree;
  ctx.renderTree = renderTree;
  ctx.loadList = loadList;
  ctx.sessionById = sessionById;
  ctx.applySessionTitle = applySessionTitle;
  ctx.applySessionPinned = applySessionPinned;
  ctx.saveSessionTitle = saveSessionTitle;
  ctx.toggleSessionPin = toggleSessionPin;
  ctx.deleteSession = deleteSession;
  ctx.askDeleteSession = askDeleteSession;
  ctx.openSessMenu = openSessMenu;
  ctx.closeSessMenu = closeSessMenu;
  ctx.applyContext = applyContext;
  ctx.contextOf = contextOf;
}
