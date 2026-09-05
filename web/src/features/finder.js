import { t, setTip, isMac, relTime } from "../lib/helpers.js";
import { iconAct } from "../lib/dom.js";
import { beginInlineRename } from "../lib/rename.js";
import { svgUse } from "../lib/svg.js";
import { api } from "../lib/api.js";
import { renderMarkdown } from "../lib/markdown.js";
import { bindCodeCopy } from "../lib/clipboard.js";
import { openOverlay, closeOverlay } from "../lib/overlay.js";

export function bindFinder(ctx) {
  const { FINDER_PREVIEW_KEY } = ctx;
  const searchBtn = document.getElementById("search-btn");
  const finderEl = document.getElementById("finder");
  const finderScrim = document.getElementById("finder-scrim");
  const finderQ = document.getElementById("finder-q");
  const finderList = document.getElementById("finder-list");
  const finderPreview = document.getElementById("finder-preview");
  const finderResize = document.getElementById("finder-resize");

  let finderPreviewTimer = 0;
  let finderComposing = false;

  function finderShortcut() {
    return isMac() ? "⌘K" : "Ctrl+K";
  }

  function syncSearchTip() {
    if (!searchBtn) return;
    const tip = t("search") + " " + finderShortcut();
    setTip(searchBtn, tip);
    searchBtn.setAttribute("aria-label", tip);
  }

  function finderOpen() {
    return finderEl && !finderEl.hidden;
  }

  function finderPreviewShortcut() {
    return isMac() ? "⌘⇧P" : "Ctrl+Shift+P";
  }

  function loadFinderPreview() {
    try {
      const raw = localStorage.getItem(FINDER_PREVIEW_KEY);
      if (raw === "0") ctx.finderPreviewOn = false;
      else if (raw === "1") ctx.finderPreviewOn = true;
    } catch (e) {
    }
    syncFinderPreview();
  }

  function saveFinderPreview() {
    try {
      localStorage.setItem(FINDER_PREVIEW_KEY, ctx.finderPreviewOn ? "1" : "0");
    } catch (e) {
    }
  }

  function syncFinderPreview() {
    if (finderEl) finderEl.classList.toggle("preview", !!ctx.finderPreviewOn);
    if (!finderResize) return;
    const tip = (ctx.finderPreviewOn ? t("finderHidePreview") : t("finderShowPreview")) + " " + finderPreviewShortcut();
    setTip(finderResize, tip);
    finderResize.setAttribute("aria-label", tip);
    const use = finderResize.querySelector("use");
    if (use) use.setAttribute("href", ctx.finderPreviewOn ? "#i-collapse" : "#i-resize");
  }

  function toggleFinderPreview() {
    ctx.finderPreviewOn = !ctx.finderPreviewOn;
    saveFinderPreview();
    syncFinderPreview();
  }

  function syncFinderKeys() {
    const edit = document.getElementById("finder-kbd-edit");
    const delK = document.getElementById("finder-kbd-delete");
    const mac = isMac();
    if (edit) edit.textContent = mac ? "⌘⇧E" : "Ctrl+Shift+E";
    if (delK) delK.textContent = mac ? "⌘⇧D" : "Ctrl+Shift+D";
  }

  function finderBucket(iso) {
    const ts = Date.parse(iso);
    if (Number.isNaN(ts)) return "older";
    const startOf = (d) => {
      const x = new Date(d);
      x.setHours(0, 0, 0, 0);
      return x.getTime();
    };
    const diff = (startOf(Date.now()) - startOf(ts)) / 86400000;
    if (diff <= 0) return "today";
    if (diff === 1) return "yesterday";
    if (diff < 7) return "week";
    return "older";
  }

  function finderFiltered() {
    const q = String((finderQ && finderQ.value) || "").trim().toLowerCase();
    const rows = (ctx.sessions || []).slice().sort((a, b) => String(b.updated_at || "").localeCompare(String(a.updated_at || "")));
    if (!q) return rows;
    return rows.filter((s) => {
      const title = String(s.title || s.id || "").toLowerCase();
      const cwd = String(s.cwd || "").toLowerCase();
      return title.includes(q) || cwd.includes(q) || String(s.id || "").toLowerCase().includes(q);
    });
  }

  function closeFinder() {
    ctx.finderRenaming = "";
    closeOverlay(finderScrim, { panel: finderEl });
  }

  function openFinder() {
    if (ctx.closeDirModal) ctx.closeDirModal();
    loadFinderPreview();
    openOverlay(finderScrim, finderEl);
    if (finderQ) {
      finderQ.placeholder = t("search") + "...";
      finderQ.value = "";
      finderQ.focus();
    }
    ctx.finderSel = ctx.currentId || "__new__";
    renderFinder();
  }

  function finderPick(id) {
    closeFinder();
    if (id === "__new__") {
      if (ctx.startNewChat) ctx.startNewChat();
    } else if (id) {
      if (ctx.openSession) ctx.openSession(id);
    }
  }

  function renderFinderPreviewEmpty() {
    if (!finderPreview) return;
    finderPreview.replaceChildren();
    const empty = document.createElement("div");
    empty.className = "finder-preview-empty";
    empty.textContent = t("finderPreview");
    finderPreview.appendChild(empty);
  }

  function renderFinderPreviewFrom(detail) {
    if (!finderPreview) return;
    finderPreview.replaceChildren();
    const blocks = (detail && detail.blocks) || [];
    let n = 0;
    blocks.forEach((b) => {
      if (n >= 16) return;
      if (b.type !== "user" && b.type !== "assistant") return;
      const text = String(b.text || "").trim();
      if (!text) return;
      const wrap = document.createElement("div");
      wrap.className = "finder-preview-msg" + (b.type === "user" ? " user" : "");
      const body = document.createElement("div");
      body.className = "md";
      const src = text.length > 8000 ? text.slice(0, 8000) + "\n…" : text;
      body.innerHTML = renderMarkdown(src);
      wrap.appendChild(body);
      finderPreview.appendChild(wrap);
      n += 1;
    });
    if (!n) renderFinderPreviewEmpty();
    else bindCodeCopy(finderPreview);
  }

  function scheduleFinderPreview(id) {
    clearTimeout(finderPreviewTimer);
    if (!id || id === "__new__") {
      renderFinderPreviewEmpty();
      return;
    }
    if (ctx.current && ctx.current.id === id) {
      renderFinderPreviewFrom(ctx.current);
      return;
    }
    if (ctx.finderCache && ctx.finderCache[id]) {
      renderFinderPreviewFrom(ctx.finderCache[id]);
      return;
    }
    renderFinderPreviewEmpty();
    finderPreviewTimer = setTimeout(async () => {
      try {
        const detail = await api("/api/sessions/" + encodeURIComponent(id));
        if (!ctx.finderCache) ctx.finderCache = {};
        ctx.finderCache[id] = detail;
        if (ctx.finderSel === id && finderOpen()) renderFinderPreviewFrom(detail);
      } catch (e) {
        if (ctx.finderSel === id && finderOpen()) renderFinderPreviewEmpty();
      }
    }, 180);
  }

  function renderFinder() {
    if (!finderList || !finderOpen()) return;
    finderList.replaceChildren();
    const q = String((finderQ && finderQ.value) || "").trim().toLowerCase();
    ctx.finderItems = [];
    const addRow = (id, opts) => {
      const b = document.createElement("div");
      b.className = "finder-row" + (ctx.finderSel === id ? " on" : "");
      b.dataset.id = id;
      b.setAttribute("role", "button");
      b.tabIndex = -1;
      if (opts.icon) b.appendChild(svgUse(opts.icon));
      const name = document.createElement("span");
      name.className = "finder-row-name";
      name.textContent = opts.name;
      b.appendChild(name);
      if (opts.badge) {
        const badge = document.createElement("span");
        badge.className = "finder-badge";
        badge.textContent = opts.badge;
        b.appendChild(badge);
      }
      if (opts.when) {
        const when = document.createElement("span");
        when.className = "finder-row-when";
        when.textContent = opts.when;
        b.appendChild(when);
      }
      if (opts.actions) {
        const actions = document.createElement("span");
        actions.className = "finder-row-actions";
        const mk = (icon, label, cls, onClick) => {
          actions.appendChild(
            iconAct({
              icon,
              className: "icon-btn" + (cls ? " " + cls : ""),
              tip: label,
              onClick
            })
          );
        };
        mk("i-edit", t("rename"), "", () => startFinderRename(id));
        mk("i-trash", t("delete"), "danger", () => {
          if (ctx.askDeleteSession) ctx.askDeleteSession(id);
        });
        b.appendChild(actions);
      }
      b.addEventListener("mouseenter", () => {
        if (ctx.finderRenaming) return;
        ctx.finderSel = id;
        finderList.querySelectorAll(".finder-row").forEach((el) => el.classList.toggle("on", el.dataset.id === id));
        scheduleFinderPreview(id);
      });
      b.addEventListener("click", () => {
        if (ctx.finderRenaming === id) return;
        finderPick(id);
      });
      finderList.appendChild(b);
      ctx.finderItems.push(id);
    };
    const showNew = !q || t("finderNew").toLowerCase().includes(q) || "new".includes(q);
    if (showNew) {
      const sec = document.createElement("div");
      sec.className = "finder-sec";
      sec.textContent = t("finderOps");
      finderList.appendChild(sec);
      addRow("__new__", { icon: "i-plus", name: t("finderNew") });
    }
    const rows = finderFiltered();
    const groups = [
      ["today", t("finderToday")],
      ["yesterday", t("finderYesterday")],
      ["week", t("finderWeek")],
      ["older", t("finderOlder")]
    ];
    const by = { today: [], yesterday: [], week: [], older: [] };
    rows.forEach((s) => by[finderBucket(s.updated_at)].push(s));
    groups.forEach(([key, lab]) => {
      const list = by[key];
      if (!list.length) return;
      const sec = document.createElement("div");
      sec.className = "finder-sec";
      sec.textContent = lab;
      finderList.appendChild(sec);
      list.forEach((s) => {
        addRow(s.id, {
          name: s.title || s.id,
          when: relTime(s.updated_at),
          badge: s.id === ctx.currentId ? t("finderCurrent") : "",
          actions: true
        });
      });
    });
    if (!ctx.finderItems.length) {
      const empty = document.createElement("div");
      empty.className = "finder-sec";
      empty.textContent = t("noSessions");
      finderList.appendChild(empty);
    }
    if (!ctx.finderItems.includes(ctx.finderSel)) ctx.finderSel = ctx.finderItems[0] || "";
    finderList.querySelectorAll(".finder-row").forEach((el) => el.classList.toggle("on", el.dataset.id === ctx.finderSel));
    const on = finderList.querySelector(".finder-row.on");
    if (on) on.scrollIntoView({ block: "nearest" });
    scheduleFinderPreview(ctx.finderSel);
  }

  function finderMove(dir) {
    if (!ctx.finderItems || !ctx.finderItems.length) return;
    const i = Math.max(0, ctx.finderItems.indexOf(ctx.finderSel));
    const next = ctx.finderItems[(i + dir + ctx.finderItems.length) % ctx.finderItems.length];
    ctx.finderSel = next;
    renderFinder();
  }

  function startFinderRename(id) {
    if (!id || id === "__new__") return;
    const row = finderList && finderList.querySelector('.finder-row[data-id="' + CSS.escape(id) + '"]');
    const name = row && row.querySelector(".finder-row-name");
    if (!name) return;
    const s = ctx.sessionById ? ctx.sessionById(id) : null;
    ctx.finderRenaming = id;
    ctx.finderSel = id;
    beginInlineRename(name, {
      value: (s && s.title) || id,
      inputClass: "finder-rename",
      onCommit: (v) => ctx.saveSessionTitle && ctx.saveSessionTitle(id, v),
      onRestore: () => {
        ctx.finderRenaming = "";
        if (finderOpen()) renderFinder();
      }
    });
  }

  if (searchBtn) {
    searchBtn.addEventListener("click", () => {
      if (finderOpen()) closeFinder();
      else openFinder();
    });
  }

  if (finderScrim) {
    finderScrim.addEventListener("click", closeFinder);
  }

  if (finderResize) {
    finderResize.addEventListener("click", toggleFinderPreview);
  }

  if (finderQ) {
    finderQ.addEventListener("input", renderFinder);
    finderQ.addEventListener("compositionstart", () => {
      finderComposing = true;
    });
    finderQ.addEventListener("compositionend", () => {
      finderComposing = false;
      renderFinder();
    });
  }

  syncSearchTip();
  syncFinderKeys();

  ctx.openFinder = openFinder;
  ctx.closeFinder = closeFinder;
  ctx.finderOpen = finderOpen;
  ctx.renderFinder = renderFinder;
  ctx.finderFiltered = finderFiltered;
  ctx.finderBucket = finderBucket;
  ctx.finderPick = finderPick;
  ctx.finderMove = finderMove;
  ctx.startFinderRename = startFinderRename;
  ctx.loadFinderPreview = loadFinderPreview;
  ctx.saveFinderPreview = saveFinderPreview;
  ctx.syncFinderPreview = syncFinderPreview;
  ctx.toggleFinderPreview = toggleFinderPreview;
  ctx.scheduleFinderPreview = scheduleFinderPreview;
  ctx.renderFinderPreviewFrom = renderFinderPreviewFrom;
  ctx.renderFinderPreviewEmpty = renderFinderPreviewEmpty;
  ctx.syncFinderKeys = syncFinderKeys;
  ctx.finderShortcut = finderShortcut;
  ctx.finderPreviewShortcut = finderPreviewShortcut;
  ctx.syncSearchTip = syncSearchTip;
  ctx.isFinderComposing = () => finderComposing;
}
