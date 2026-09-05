import { t, fmtNum, fmtDur, fmtCost, fmtBytes, setPressed, oneLinePreview, focusKeyThought, focusKeyTool } from "../lib/helpers.js";
import { emptyEl, kv } from "../lib/dom.js";
import { placePopover } from "../lib/popover.js";
import { svgUse } from "../lib/svg.js";
import { api } from "../lib/api.js";
import { renderMarkdown } from "../lib/markdown.js";
import { bindCodeCopy } from "../lib/clipboard.js";
import { shortToolName, extractToolParts } from "../lib/toolParts.js";
import { openOverlay, closeOverlay } from "../lib/overlay.js";

export function bindDrawer(ctx) {
  const drawerEl = document.getElementById("drawer");
  const drawerBody = document.getElementById("drawer-body");
  const drawerStatus = document.getElementById("drawer-status");
  const drawerTitle = document.getElementById("drawer-title");
  const drawerScrim = document.getElementById("drawer-scrim");
  const drawerClose = document.getElementById("drawer-close");
  const usageBody = document.getElementById("usage-body");
  const hostBody = document.getElementById("host-body");
  const infoPop = document.getElementById("info-pop");
  const infoBody = document.getElementById("info-body");
  const usageToggle = document.getElementById("usage-toggle");

  let lastInfoKind = "info";

  function statusDrawerOpen() {
    return drawerEl && !drawerEl.hidden && ctx.drawerMode === "status";
  }

  function turnHasProcess(turn) {
    return (turn.agent || []).some((b) => b.type === "thought" || b.type === "tool");
  }

  function showDrawer() {
    if (drawerEl) drawerEl.hidden = false;
    const main = document.getElementById("main");
    if (main) main.classList.add("drawer-open");
    if (window.matchMedia("(max-width: 900px)").matches) {
      openOverlay(drawerScrim);
    }
  }

  function setDrawerMode(mode) {
    ctx.drawerMode = mode || "";
    const titles = { status: "status", process: "process", files: "workspace" };
    if (drawerTitle) {
      const key = titles[ctx.drawerMode] || "process";
      drawerTitle.setAttribute("data-i18n", key);
      drawerTitle.textContent = t(key);
    }
    if (drawerBody) {
      drawerBody.hidden = ctx.drawerMode !== "process";
      drawerBody.classList.remove("mcp");
    }
    if (drawerStatus) drawerStatus.hidden = ctx.drawerMode !== "status";
    const drawerFiles = document.getElementById("drawer-files");
    if (drawerFiles) drawerFiles.hidden = ctx.drawerMode !== "files";
    if (usageToggle) {
      setPressed(usageToggle, ctx.drawerMode === "status");
      usageToggle.setAttribute("aria-expanded", ctx.drawerMode === "status" ? "true" : "false");
    }
    const wsToggle = document.getElementById("ws-toggle");
    if (wsToggle) {
      setPressed(wsToggle, ctx.drawerMode === "files");
      wsToggle.setAttribute("aria-expanded", ctx.drawerMode === "files" ? "true" : "false");
    }
    if (ctx.drawerMode === "files") stopStatusPoll();
  }

  function openDrawer(promptId, focus) {
    const turns = ctx.current ? (ctx.groupTurns ? ctx.groupTurns(ctx.current.blocks || []) : []) : [];
    const turn = turns.find((t) => t.prompt_id === promptId) || turns[turns.length - 1];
    if (!turn || !turnHasProcess(turn)) return;
    ctx.drawerPromptId = turn.prompt_id || promptId || "";
    if (focus !== undefined) ctx.drawerFocus = focus;
    if (ctx.drawerMode === "status") stopStatusPoll();
    setDrawerMode("process");
    showDrawer();
    renderDrawer();
    if (ctx.drawerFocus) {
      const hit = drawerBody && drawerBody.querySelector('[data-focus="' + String(ctx.drawerFocus).replace(/"/g, "") + '"]');
      if (hit) hit.scrollIntoView({ block: "nearest" });
    } else if (drawerBody) {
      drawerBody.scrollTop = 0;
    }
  }

  function closeDrawer() {
    if (drawerEl) drawerEl.hidden = true;
    closeOverlay(drawerScrim);
    const main = document.getElementById("main");
    if (main) main.classList.remove("drawer-open");
    stopStatusPoll();
    setDrawerMode("");
  }

  function drawerItems(turn) {
    const out = [];
    let ti = 0;
    for (const b of (turn && turn.agent) || []) {
      if (b.type === "thought") {
        out.push({ kind: "thought", block: b, idx: ti, focus: focusKeyThought(turn.prompt_id, ti) });
        ti += 1;
      } else if (b.type === "tool") {
        out.push({ kind: "tool", block: b, focus: focusKeyTool(b.id) });
      }
    }
    return out;
  }

  function accordionChevron(dir) {
    const wrap = document.createElement("span");
    wrap.innerHTML =
      dir === "down"
        ? '<svg width="15" height="15" viewBox="0 0 15 15" fill="none" xmlns="http://www.w3.org/2000/svg" class="drawer-chev drawer-chev-down" aria-hidden="true"><path d="M3.13523 6.15803C3.3241 5.95657 3.64052 5.94637 3.84197 6.13523L7.5 9.56464L11.158 6.13523C11.3595 5.94637 11.6759 5.95657 11.8648 6.15803C12.0536 6.35949 12.0434 6.67591 11.842 6.86477L7.84197 10.6148C7.64964 10.7951 7.35036 10.7951 7.15803 10.6148L3.15803 6.86477C2.95657 6.67591 2.94637 6.35949 3.13523 6.15803Z" fill="currentColor" fill-rule="evenodd" clip-rule="evenodd"></path></svg>'
        : '<svg width="15" height="15" viewBox="0 0 15 15" fill="none" xmlns="http://www.w3.org/2000/svg" class="drawer-chev drawer-chev-right" aria-hidden="true"><path d="M6.1584 3.13508C6.35985 2.94621 6.67627 2.95642 6.86514 3.15788L10.6151 7.15788C10.7954 7.3502 10.7954 7.64949 10.6151 7.84182L6.86514 11.8418C6.67627 12.0433 6.35985 12.0535 6.1584 11.8646C5.95694 11.6757 5.94673 11.3593 6.1356 11.1579L9.565 7.49985L6.1356 3.84182C5.94673 3.64036 5.95694 3.32394 6.1584 3.13508Z" fill="currentColor" fill-rule="evenodd" clip-rule="evenodd"></path></svg>';
    return wrap.firstChild;
  }

  function drawerIcoSlot(kindId) {
    const slot = document.createElement("span");
    slot.className = "drawer-ico-slot";
    const kind = svgUse(kindId);
    kind.classList.add("drawer-ico", "drawer-kind");
    slot.append(kind, accordionChevron("right"));
    return slot;
  }

  function closeDrawerRows() {
    if (!drawerBody) return;
    drawerBody.querySelectorAll(".drawer-row.on").forEach((el) => {
      el.classList.remove("on");
      el.setAttribute("data-state", "closed");
      el.setAttribute("aria-expanded", "false");
      const next = el.nextElementSibling;
      if (next && next.classList.contains("drawer-detail")) next.remove();
    });
  }

  function toolErrMessage(err) {
    return String((err && err.message) || err || "");
  }

  function toolStatusDone(status) {
    const st = String(status || "");
    return st === "completed" || st === "failed";
  }

  function toolCacheStale(cached, block) {
    if (!cached || cached.loading) return false;
    if (cached.error) return false;
    const st = String((block && block.status) || "");
    if (toolStatusDone(st) && cached.status && cached.status !== st) return true;
    return false;
  }

  function renderToolDetail(el, data, block) {
    el.className = "drawer-detail";
    el.replaceChildren();
    const parts = extractToolParts(data, block).filter((p) => p && String(p.text || "").length);
    if (!parts.length) {
      const preview = String((block && (block.input_preview || block.title)) || "").trim();
      el.textContent = preview || t("emptyQueue");
      return;
    }
    const wrap = document.createElement("div");
    wrap.className = "tool-parts";
    for (const part of parts) {
      const card = document.createElement("div");
      card.className = "code-card";
      card.innerHTML =
        '<div class="code-head"><span>' +
        (part.label || "text") +
        '</span><button type="button" class="copy-code" data-i18n="copyCode">' +
        t("copyCode") +
        "</button></div><pre><code>" +
        (part.text || "").replace(/\n$/, "") +
        "</code></pre>";
      bindCodeCopy(card);
      wrap.appendChild(card);
    }
    el.appendChild(wrap);
  }

  function paintToolDetail(el, item, cached) {
    renderToolDetail(el, cached && cached.data, item.block);
    if (cached && cached.error) {
      const err = document.createElement("p");
      err.className = "drawer-detail-err";
      err.textContent = t("loadFailed", { e: cached.error });
      el.appendChild(err);
    } else if (!cached || cached.loading) {
      const wait = document.createElement("p");
      wait.className = "drawer-detail-pending";
      wait.textContent = t("loading");
      el.appendChild(wait);
    }
  }

  function makeDrawerDetail(item) {
    if (item.kind === "thought") {
      const d = document.createElement("div");
      d.className = "drawer-detail";
      const body = document.createElement("div");
      body.className = "block-body md";
      body.innerHTML = renderMarkdown((item.block && item.block.text) || "");
      bindCodeCopy(body);
      d.appendChild(body);
      return d;
    }
    const d = document.createElement("div");
    d.className = "drawer-detail";
    if (!ctx.drawerDetailCache) ctx.drawerDetailCache = {};
    let cached = ctx.drawerDetailCache[item.block.id];
    if (toolCacheStale(cached, item.block)) {
      delete ctx.drawerDetailCache[item.block.id];
      cached = null;
    }
    paintToolDetail(d, item, cached);
    const quiet = !!(cached && cached.data && !cached.loading && !toolStatusDone(item.block.status) && Date.now() - (cached.at || 0) > 800);
    if ((!cached && item.block.id && ctx.currentId) || quiet) {
      if (quiet) cached.loading = true;
      else ctx.drawerDetailCache[item.block.id] = { loading: true, status: item.block.status, at: Date.now() };
      api("/api/sessions/" + encodeURIComponent(ctx.currentId) + "/tools/" + encodeURIComponent(item.block.id))
        .then((data) => {
          ctx.drawerDetailCache[item.block.id] = { data: data || {}, status: item.block.status, at: Date.now() };
          if (ctx.drawerFocus !== item.focus) return;
          const row = drawerBody && drawerBody.querySelector('[data-focus="' + String(item.focus).replace(/"/g, "") + '"]');
          const next = row && row.nextElementSibling;
          if (next && next.classList.contains("drawer-detail")) renderToolDetail(next, data || {}, item.block);
        })
        .catch((err) => {
          ctx.drawerDetailCache[item.block.id] = { error: toolErrMessage(err), status: item.block.status, at: Date.now() };
          if (ctx.drawerFocus !== item.focus) return;
          const row = drawerBody && drawerBody.querySelector('[data-focus="' + String(item.focus).replace(/"/g, "") + '"]');
          const next = row && row.nextElementSibling;
          if (next && next.classList.contains("drawer-detail")) {
            paintToolDetail(next, item, ctx.drawerDetailCache[item.block.id]);
          }
        });
    }
    return d;
  }

  function renderDrawer() {
    if (!drawerBody) return;
    const stick = drawerBody.scrollHeight - drawerBody.scrollTop - drawerBody.clientHeight < 80;
    drawerBody.replaceChildren();
    if (!ctx.current) return;
    const turns = ctx.groupTurns ? ctx.groupTurns(ctx.current.blocks || []) : [];
    const turn = turns.find((t) => t.prompt_id === ctx.drawerPromptId) || turns[turns.length - 1];
    if (!turn) return;
    ctx.drawerPromptId = turn.prompt_id || ctx.drawerPromptId;
    const items = drawerItems(turn);
    for (const item of items) {
      const row = document.createElement("button");
      row.type = "button";
      const open = ctx.drawerFocus === item.focus;
      row.className = "drawer-row " + item.kind + (open ? " on" : "");
      row.dataset.focus = item.focus;
      row.setAttribute("data-state", open ? "open" : "closed");
      row.setAttribute("aria-expanded", open ? "true" : "false");
      if (item.kind === "thought") {
        row.appendChild(drawerIcoSlot("i-bulb"));
        const main = document.createElement("div");
        main.className = "drawer-main";
        const titleLine = document.createElement("div");
        titleLine.className = "drawer-title-line";
        titleLine.textContent = oneLinePreview(item.block.text);
        main.appendChild(titleLine);
        row.appendChild(main);
      } else {
        row.appendChild(drawerIcoSlot("i-wrench"));
        const main = document.createElement("div");
        main.className = "drawer-main";
        const titleLine = document.createElement("div");
        titleLine.className = "drawer-title-line";
        titleLine.textContent = shortToolName(item.block);
        const sub = document.createElement("div");
        sub.className = "drawer-sub";
        sub.textContent = item.block.input_preview || item.block.title || "";
        main.append(titleLine, sub);
        row.appendChild(main);
        if (item.block.result_count) {
          const badge = document.createElement("span");
          badge.className = "drawer-badge";
          badge.textContent = String(item.block.result_count);
          row.appendChild(badge);
        }
      }
      row.addEventListener("click", () => {
        const opening = ctx.drawerFocus !== item.focus;
        closeDrawerRows();
        if (!opening) {
          ctx.drawerFocus = "";
          return;
        }
        const prev = ctx.drawerDetailCache && ctx.drawerDetailCache[item.block.id];
        if (prev && prev.error) delete ctx.drawerDetailCache[item.block.id];
        ctx.drawerFocus = item.focus;
        row.classList.add("on");
        row.setAttribute("data-state", "open");
        row.setAttribute("aria-expanded", "true");
        row.after(makeDrawerDetail(item));
      });
      drawerBody.appendChild(row);
      if (open) drawerBody.appendChild(makeDrawerDetail(item));
    }
    if (stick) drawerBody.scrollTop = drawerBody.scrollHeight;
  }

  function usageCells(parent, label, num, note) {
    const k = document.createElement("span");
    k.className = "usage-k";
    k.textContent = label;
    const n = document.createElement("span");
    n.className = "usage-num";
    n.textContent = num;
    const noteEl = document.createElement("span");
    noteEl.className = "usage-note";
    noteEl.textContent = note || "";
    parent.append(k, noteEl, n);
  }

  function renderUsage(usage) {
    if (!usageBody) return;
    usageBody.replaceChildren();
    if (!usage || !usage.recorded) {
      usageBody.appendChild(emptyEl("usage-empty", t("noModelCalls")));
      return;
    }
    usageCells(usageBody, t("inputTokens"), fmtNum(usage.input_tokens), t("cachedNote", { n: fmtNum(usage.cached_tokens) }));
    usageCells(usageBody, t("outputTokens"), fmtNum(usage.output_tokens), t("reasoningNote", { n: fmtNum(usage.reasoning_tokens) }));
    usageCells(usageBody, t("totalTokens"), fmtNum(usage.total_tokens));
    usageCells(usageBody, t("modelCalls"), fmtNum(usage.model_calls));
    usageCells(usageBody, t("apiTime"), fmtDur(usage.api_duration_ms));
    usageCells(usageBody, t("cost"), fmtCost(usage.cost_usd_ticks));
    const models = usage.models || [];
    if (!models.length) return;
    const box = document.createElement("div");
    box.className = "usage-models";
    for (const m of models) {
      usageCells(box, m.model || t("model"), t("inOut", { inn: fmtNum(m.input_tokens), out: fmtNum(m.output_tokens) }));
    }
    usageBody.appendChild(box);
  }

  function renderHost(st) {
    if (!hostBody) return;
    hostBody.replaceChildren();
    if (!st) return;
    kv(hostBody, "host-k", "host-v", t("user"), st.user);
    kv(hostBody, "host-k", "host-v", t("hostname"), st.hostname);
    kv(hostBody, "host-k", "host-v", t("lanIpv4"), st.ipv4_lan);
    kv(hostBody, "host-k", "host-v", t("wanIpv4"), st.ipv4_wan);
    const cpu = st.cpu || {};
    const pct = cpu.percent == null ? "—" : Number(cpu.percent).toFixed(0) + "%";
    kv(
      hostBody,
      "host-k",
      "host-v",
      t("cpu"),
      pct + " · " + [cpu.load1, cpu.load5, cpu.load15].map((n) => Number(n || 0).toFixed(2)).join(" / ")
    );
    const mem = st.memory || {};
    kv(hostBody, "host-k", "host-v", t("memory"), fmtBytes(mem.used_bytes) + " / " + fmtBytes(mem.total_bytes));
    for (const d of st.disks || []) {
      kv(hostBody, "host-k", "host-v", t("disk", { path: d.path }), fmtBytes(d.used_bytes) + " / " + fmtBytes(d.total_bytes));
    }
  }

  async function refreshHost() {
    try {
      ctx.lastHost = await api("/api/status");
      renderHost(ctx.lastHost);
    } catch (e) {
    }
  }

  async function refreshSessionUsage() {
    if (!ctx.currentId) return;
    try {
      const detail = await api("/api/sessions/" + encodeURIComponent(ctx.currentId));
      if (!detail) return;
      if (detail.usage && detail.usage.recorded) {
        const prev = (ctx.current && ctx.current.usage) || {};
        if (!prev.recorded || (detail.usage.total_tokens || 0) >= (prev.total_tokens || 0)) {
          if (ctx.current) ctx.current.usage = detail.usage;
          renderUsage(detail.usage);
        }
      }
      if (detail.context && ctx.applyContext) ctx.applyContext(detail.context);
    } catch (e) {
    }
  }

  function startStatusPoll() {
    clearInterval(ctx.statusTimer);
    const secs = Math.max(2, Number((ctx.runtime && ctx.runtime.poll_secs) || 5));
    ctx.statusTimer = setInterval(() => {
      refreshHost();
      refreshSessionUsage();
    }, secs * 1000);
  }

  function stopStatusPoll() {
    clearInterval(ctx.statusTimer);
    ctx.statusTimer = 0;
  }

  function setStatusOpen(on) {
    if (!on) {
      if (statusDrawerOpen()) closeDrawer();
      return;
    }
    if (infoPop) infoPop.hidden = true;
    setDrawerMode("status");
    showDrawer();
    renderUsage((ctx.current && ctx.current.usage) || {});
    refreshSessionUsage();
    refreshHost();
    startStatusPoll();
  }

  function setInfoOpen(on, kind) {
    if (!infoPop) return;
    infoPop.hidden = !on;
    if (on) {
      lastInfoKind = kind || lastInfoKind || "info";
      if (statusDrawerOpen()) closeDrawer();
      const rows = [];
      if (kind === "context") {
        const titleEl = document.getElementById("info-title");
        if (titleEl) titleEl.textContent = t("context");
        const { used, window } = ctx.contextOf ? ctx.contextOf() : { used: 0, window: 0 };
        const pct = window > 0 ? Math.min(100, Math.round((used / window) * 100)) : 0;
        const u = (ctx.current && ctx.current.usage) || {};
        rows.push([t("used"), fmtNum(used) + " (" + pct + "%)"]);
        rows.push([t("window"), fmtNum(window)]);
        rows.push([t("remainingLabel"), fmtNum(Math.max(0, window - used))]);
        rows.push([t("input"), fmtNum(u.input_tokens)]);
        rows.push([t("output"), fmtNum(u.output_tokens)]);
        rows.push([t("total"), fmtNum(u.total_tokens)]);
        rows.push([t("cachedLabel"), fmtNum(u.cached_tokens)]);
      } else {
        const titleEl = document.getElementById("info-title");
        if (titleEl) titleEl.textContent = t("session");
        rows.push([t("id"), ctx.currentId || "—"]);
        rows.push([t("cwd"), (ctx.current && ctx.current.cwd) || ctx.selectedCwd || "—"]);
        rows.push([t("model"), (ctx.current && ctx.current.model) || ctx.selectedModel || "—"]);
        rows.push([t("title"), (ctx.current && ctx.current.title) || "—"]);
      }
      if (infoBody) {
        infoBody.replaceChildren();
        for (const [k, v] of rows) kv(infoBody, "info-k", "info-v", k, v);
      }
      pinInfoPop();
    }
  }

  function pinInfoPop() {
    if (!infoPop || infoPop.hidden) return;
    const actions = document.getElementById("actions");
    const anchor = actions || infoPop.parentElement;
    if (!anchor) return;
    placePopover(infoPop, anchor, {
      gap: 8,
      pad: 12,
      minH: 96,
      width: Math.min(420, window.innerWidth * 0.86),
      align: "right",
      zIndex: 25
    });
  }

  if (usageToggle) {
    usageToggle.addEventListener("click", (e) => {
      e.stopPropagation();
      if (ctx.setQuotaOpen) ctx.setQuotaOpen(false);
      setStatusOpen(!statusDrawerOpen());
    });
  }

  if (drawerClose) {
    drawerClose.addEventListener("click", (e) => {
      e.stopPropagation();
      closeDrawer();
    });
  }

  if (drawerScrim) {
    drawerScrim.addEventListener("click", closeDrawer);
  }

  if (drawerEl) {
    drawerEl.addEventListener("click", (e) => e.stopPropagation());
  }

  if (infoPop) {
    infoPop.addEventListener("click", (e) => e.stopPropagation());
  }

  window.addEventListener("resize", pinInfoPop);

  ctx.openDrawer = openDrawer;
  ctx.closeDrawer = closeDrawer;
  ctx.showDrawer = showDrawer;
  ctx.setDrawerMode = setDrawerMode;
  ctx.statusDrawerOpen = statusDrawerOpen;
  ctx.filesDrawerOpen = () => drawerEl && !drawerEl.hidden && ctx.drawerMode === "files";
  ctx.setStatusOpen = setStatusOpen;
  ctx.setInfoOpen = setInfoOpen;
  ctx.renderDrawer = renderDrawer;
  ctx.drawerItems = drawerItems;
  ctx.makeDrawerDetail = makeDrawerDetail;
  ctx.renderToolDetail = renderToolDetail;
  ctx.accordionChevron = accordionChevron;
  ctx.drawerIcoSlot = drawerIcoSlot;
  ctx.closeDrawerRows = closeDrawerRows;
  ctx.renderUsage = renderUsage;
  ctx.renderHost = renderHost;
  ctx.refreshHost = refreshHost;
  ctx.refreshSessionUsage = refreshSessionUsage;
  ctx.startStatusPoll = startStatusPoll;
  ctx.stopStatusPoll = stopStatusPoll;
  ctx.usageCells = usageCells;
  ctx.kv = kv;
}
