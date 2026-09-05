import { t, setTip, setDynI18n, fileNameOf, fileViewSrc, isImageAttach, revokePreview, focusKeyThought, focusKeyTool } from "../lib/helpers.js";
import { svgUse } from "../lib/svg.js";
import { post } from "../lib/api.js";
import { renderMarkdown } from "../lib/markdown.js";
import { bindCodeCopy, copyText, toast } from "../lib/clipboard.js";
import { shortToolName } from "../lib/toolParts.js";
import { openOverlay, closeOverlay } from "../lib/overlay.js";

export function bindTimeline(ctx) {
  const timeline = document.getElementById("timeline");
  const actions = document.getElementById("actions");
  const app = document.getElementById("app");
  const dl = document.getElementById("dl-md");

  const cancelledByUser = Object.create(null);

  function isPendingPrompt(id) {
    return String(id || "").startsWith("pending-");
  }

  function groupTurns(blocks) {
    const turns = [];
    let cur = { prompt_id: "", user: [], agent: [], duration_ms: 0, ended: false, cancelled: false };
    const flush = () => {
      if (cancelledByUser[cur.prompt_id]) cur.cancelled = true;
      if (cur.user.length || cur.agent.length) turns.push(cur);
      cur = { prompt_id: "", user: [], agent: [], duration_ms: 0, ended: false, cancelled: false };
    };
    for (const b of blocks || []) {
      if (b.type === "turn_end") {
        if (b.prompt_id) cur.prompt_id = b.prompt_id;
        cur.duration_ms = Number(b.duration_ms || 0);
        cur.ended = true;
        cur.cancelled = !!(b.cancelled || cancelledByUser[cur.prompt_id]);
        flush();
        continue;
      }
      if (b.type === "user") {
        if (cur.agent.length && cur.user.length) {
          cur.ended = true;
          flush();
        }
        const prevUser = cur.user[cur.user.length - 1];
        if (prevUser && String(prevUser.text || "") === String(b.text || "")) {
          const prevPend = isPendingPrompt(prevUser.prompt_id);
          const nextPend = isPendingPrompt(b.prompt_id);
          if (prevPend && !nextPend) cur.user[cur.user.length - 1] = b;
          else if (!prevPend && nextPend) {
          } else if (prevPend && nextPend) {
          } else cur.user.push(b);
        } else {
          cur.user.push(b);
        }
        cur.prompt_id = b.prompt_id && !isPendingPrompt(b.prompt_id) ? b.prompt_id : cur.prompt_id || b.prompt_id;
        if (b.cancelled) cur.cancelled = true;
      } else {
        cur.agent.push(b);
        cur.prompt_id = b.prompt_id || cur.prompt_id;
        if (b.cancelled) cur.cancelled = true;
      }
    }
    flush();
    return turns;
  }

  function closeFileLightbox(immediate) {
    const el = document.getElementById("file-lightbox");
    document.removeEventListener("keydown", onFileLightboxKey, true);
    if (!el) return;
    if (immediate) {
      el.remove();
      return;
    }
    closeOverlay(el, { onDone: () => el.remove() });
  }

  function onFileLightboxKey(e) {
    if (e.key === "Escape") {
      e.preventDefault();
      closeFileLightbox();
    }
  }

  function openFileLightbox(src, name) {
    closeFileLightbox(true);
    const overlay = document.createElement("div");
    overlay.id = "file-lightbox";
    overlay.className = "ui-scrim";
    overlay.setAttribute("role", "dialog");
    overlay.setAttribute("aria-label", name || t("attach"));
    const img = document.createElement("img");
    img.src = src;
    img.alt = name || "";
    overlay.appendChild(img);
    overlay.addEventListener("click", () => closeFileLightbox());
    img.addEventListener("click", (e) => e.stopPropagation());
    document.addEventListener("keydown", onFileLightboxKey, true);
    document.body.appendChild(overlay);
    openOverlay(overlay);
  }

  function openFileChip(f) {
    if (!f || f.processing) return;
    const src = fileViewSrc(f);
    if (!src) return;
    if (isImageAttach(f)) openFileLightbox(src, fileNameOf(f));
    else window.open(src, "_blank", "noopener");
  }

  function makeFileChip(f, removable) {
    const chip = document.createElement("span");
    const processing = !!(f && f.processing);
    const src = processing ? "" : fileViewSrc(f);
    const image = !processing && isImageAttach(f) && src;
    chip.className = "file-chip" + (image ? " has-thumb" : "") + (processing ? " processing" : "") + (!processing && src ? " clickable" : "");
    if (processing) {
      const spin = document.createElement("span");
      spin.className = "file-chip-spin";
      chip.appendChild(spin);
    } else if (image) {
      const img = document.createElement("img");
      img.src = src;
      img.alt = fileNameOf(f);
      chip.appendChild(img);
    } else {
      const ico = document.createElement("span");
      ico.className = "file-chip-ico";
      ico.appendChild(svgUse("i-clip"));
      chip.appendChild(ico);
    }
    const label = document.createElement("span");
    label.className = "file-chip-name";
    if (processing) {
      label.setAttribute("data-i18n", "processing");
      label.textContent = t("processing");
    } else {
      label.textContent = fileNameOf(f);
    }
    chip.appendChild(label);
    if (removable) {
      const x = document.createElement("button");
      x.type = "button";
      x.appendChild(svgUse("i-x"));
      x.addEventListener("click", (e) => {
        e.stopPropagation();
        ctx.attachments = (ctx.attachments || []).filter((a) => a !== f);
        revokePreview(f);
        if (ctx.renderChips) ctx.renderChips();
      });
      chip.appendChild(x);
    }
    if (!processing && src) {
      chip.tabIndex = 0;
      chip.setAttribute("role", "button");
      chip.addEventListener("click", () => openFileChip(f));
      chip.addEventListener("keydown", (e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          openFileChip(f);
        }
      });
    }
    return chip;
  }

  function userActionBtn(icon, key, onClick) {
    const b = document.createElement("button");
    b.type = "button";
    b.className = "icon-btn";
    const title = t(key);
    setTip(b, title);
    b.setAttribute("data-i18n-title", key);
    b.setAttribute("aria-label", title);
    b.appendChild(svgUse(icon));
    b.addEventListener("click", (e) => {
      e.stopPropagation();
      onClick();
    });
    return b;
  }

  function renderUser(block, appear) {
    const row = document.createElement("div");
    row.className = "user-row";
    const files = Array.isArray(block.files) ? block.files : [];
    if (files.length) {
      const strip = document.createElement("div");
      strip.className = "user-files";
      for (const f of files) strip.appendChild(makeFileChip(f, false));
      row.appendChild(strip);
    }
    const text = String(block.text || "");
    if (text) {
      const wrap = document.createElement("div");
      wrap.className = "say" + (appear ? " ui-in" : "");
      wrap.dataset.prompt = block.prompt_id || "";
      wrap.dataset.kind = "user";
      const body = document.createElement("div");
      body.className = "block-body plain";
      body.textContent = text;
      wrap.appendChild(body);
      row.appendChild(wrap);
    }
    const actionsEl = document.createElement("div");
    actionsEl.className = "user-actions";
    actionsEl.append(
      userActionBtn("i-edit", "edit", () => {
        if (ctx.fillComposer) ctx.fillComposer(block.text);
      }),
      userActionBtn("i-copy", "copy", () => copyText(block.text))
    );
    row.appendChild(actionsEl);
    return row;
  }

  function isToolDone(tr) {
    const s = String((tr && tr.status) || "").toLowerCase();
    return s === "completed" || s === "failed" || s === "cancelled" || s === "success" || s === "done";
  }

  function splitTrace(agentBlocks, live) {
    const segs = [];
    let items = [];
    let toolGroup = [];
    let nThought = 0;
    let nTool = 0;
    let segThought = 0;
    let segTool = 0;
    const flushTools = () => {
      if (!toolGroup.length) return;
      let i = 0;
      while (i < toolGroup.length) {
        const tr = toolGroup[i];
        const name = shortToolName(tr);
        if (!isToolDone(tr)) {
          items.push({ kind: "tool", tools: [tr], running: !!live, name });
          i += 1;
          continue;
        }
        let j = i + 1;
        while (j < toolGroup.length && shortToolName(toolGroup[j]) === name && isToolDone(toolGroup[j])) {
          j += 1;
        }
        items.push({ kind: "tool", tools: toolGroup.slice(i, j), running: false, name });
        i = j;
      }
      toolGroup = [];
    };
    const flushProcess = () => {
      flushTools();
      if (!items.length) return;
      segs.push({
        kind: "process",
        items,
        nThought: segThought,
        nTool: segTool
      });
      items = [];
      segThought = 0;
      segTool = 0;
    };
    for (const b of agentBlocks || []) {
      if (b.type === "thought") {
        flushTools();
        nThought += 1;
        segThought += 1;
        items.push({ kind: "thought", block: b, live: false, idx: nThought - 1 });
      } else if (b.type === "tool") {
        nTool += 1;
        segTool += 1;
        toolGroup.push(b);
      } else if (b.type === "assistant") {
        flushProcess();
        const text = String(b.text || "").trim();
        if (text) segs.push({ kind: "asst", text });
      }
    }
    flushProcess();
    if (live) {
      for (let i = segs.length - 1; i >= 0; i--) {
        if (segs[i].kind !== "process") continue;
        segs[i].live = true;
        const it = segs[i].items;
        for (let j = it.length - 1; j >= 0; j--) {
          if (it[j].kind === "thought") {
            it[j].live = true;
            break;
          }
          if (it[j].kind === "tool") break;
        }
        break;
      }
    }
    return { segs, nThought, nTool };
  }

  function liveSeconds() {
    if (!ctx.workStarted) ctx.workStarted = Date.now();
    return Math.max(0, Math.round((Date.now() - ctx.workStarted) / 1000));
  }

  function applyWorkStartedMs(ms) {
    const n = Number(ms || 0);
    if (!n || n > Date.now() + 2000) return false;
    ctx.workStarted = n;
    return true;
  }

  function turnSeconds(turn) {
    const ms = Number(turn.duration_ms || 0);
    if (ms < 500) return null;
    return Math.round(ms / 1000);
  }

  function thoughtPreviewLines(text, live) {
    const lines = String(text || "")
      .split(/\r?\n/)
      .map((s) => s.trim())
      .filter(Boolean);
    if (!lines.length) return [""];
    if (!live) return [lines[0]];
    return lines.slice(0, 3);
  }

  function renderTraceThought(item, promptId) {
    const row = document.createElement("button");
    row.type = "button";
    row.className = "trace-row" + (item.live ? " thought-live" : "");
    const ico = svgUse("i-bulb");
    ico.classList.add("trace-ico");
    row.appendChild(ico);
    const text = document.createElement("span");
    text.className = "trace-text";
    const inner = document.createElement("span");
    if (item.live) inner.className = "shimmer-text";
    inner.textContent = thoughtPreviewLines(item.block.text, item.live).join("\n");
    text.appendChild(inner);
    row.appendChild(text);
    row.addEventListener("click", () => {
      if (ctx.openDrawer) ctx.openDrawer(promptId, focusKeyThought(promptId, item.idx));
    });
    return row;
  }

  function compactToolPreview(preview) {
    const s = String(preview || "").trim();
    if (!s) return "";
    const c = s[0];
    if (c === "{" || c === "[") return "";
    return s;
  }

  function renderTraceTool(item, promptId) {
    const wrap = document.createElement("div");
    const row = document.createElement("button");
    row.type = "button";
    row.className = "trace-row" + (item.running ? " tool-live" : "");
    row.appendChild(svgUse("i-wrench"));
    const ico = row.querySelector("svg");
    if (ico) ico.classList.add("trace-ico");
    const text = document.createElement("span");
    text.className = "trace-text" + (item.running ? " shimmer-text" : "");
    const first = item.tools[0] || {};
    const preview = compactToolPreview(first.input_preview);
    const title = String(first.title || "").trim();
    if (item.running) {
      setDynI18n(text, "runningTool", { name: item.name, preview: preview ? "  " + preview : "" });
    } else if (item.tools.length > 1) {
      setDynI18n(text, "ranTools", { n: item.tools.length, name: item.name });
    } else if (preview) {
      text.textContent = item.name + "  " + preview;
    } else {
      text.textContent = title || item.name;
    }
    row.appendChild(text);
    const count = item.tools.reduce((n, tr) => n + (tr.result_count ? Number(tr.result_count) : 0), 0);
    if (count) {
      const c = document.createElement("span");
      c.className = "trace-count";
      c.textContent = String(count);
      row.appendChild(c);
    }
    const focus = focusKeyTool(first.id || "");
    row.addEventListener("click", () => {
      if (ctx.openDrawer) ctx.openDrawer(promptId, focus);
    });
    wrap.appendChild(row);
    return wrap;
  }

  function accordionChevron(dir) {
    if (ctx.accordionChevron) return ctx.accordionChevron(dir);
    const wrap = document.createElement("span");
    wrap.innerHTML =
      dir === "down"
        ? '<svg width="15" height="15" viewBox="0 0 15 15" fill="none" xmlns="http://www.w3.org/2000/svg" class="drawer-chev drawer-chev-down" aria-hidden="true"><path d="M3.13523 6.15803C3.3241 5.95657 3.64052 5.94637 3.84197 6.13523L7.5 9.56464L11.158 6.13523C11.3595 5.94637 11.6759 5.95657 11.8648 6.15803C12.0536 6.35949 12.0434 6.67591 11.842 6.86477L7.84197 10.6148C7.64964 10.7951 7.35036 10.7951 7.15803 10.6148L3.15803 6.86477C2.95657 6.67591 2.94637 6.35949 3.13523 6.15803Z" fill="currentColor" fill-rule="evenodd" clip-rule="evenodd"></path></svg>'
        : '<svg width="15" height="15" viewBox="0 0 15 15" fill="none" xmlns="http://www.w3.org/2000/svg" class="drawer-chev drawer-chev-right" aria-hidden="true"><path d="M6.1584 3.13508C6.35985 2.94621 6.67627 2.95642 6.86514 3.15788L10.6151 7.15788C10.7954 7.3502 10.7954 7.64949 10.6151 7.84182L6.86514 11.8418C6.67627 12.0433 6.35985 12.0535 6.1584 11.8646C5.95694 11.6757 5.94673 11.3593 6.1356 11.1579L9.565 7.49985L6.1356 3.84182C5.94673 3.64036 5.95694 3.32394 6.1584 3.13508Z" fill="currentColor" fill-rule="evenodd" clip-rule="evenodd"></path></svg>';
    return wrap.firstChild;
  }

  function renderProcessChip(nThought, nTool, opts) {
    opts = opts || {};
    const n = nThought + nTool;
    if (!n && !opts.live) return null;
    const chip = document.createElement("button");
    chip.type = "button";
    chip.className = "chip process" + (opts.open ? " open" : "") + (opts.live ? " live" : "") + (opts.appear ? " ui-in" : "");
    if (opts.live) {
      const dot = document.createElement("span");
      dot.className = "trace-dot";
      chip.appendChild(dot);
    }
    const stack = document.createElement("span");
    stack.className = "chip-stack";
    if (nThought) {
      const a = document.createElement("span");
      a.className = "chip-ico";
      a.appendChild(svgUse("i-spark"));
      stack.appendChild(a);
    }
    if (nTool) {
      const a = document.createElement("span");
      a.className = "chip-ico";
      a.appendChild(svgUse("i-wrench"));
      stack.appendChild(a);
    }
    const span = document.createElement("span");
    span.className = "chip-label";
    if (opts.live) {
      if (opts.seconds == null) setDynI18n(span, "working");
      else setDynI18n(span, "workedSeconds", { n: opts.seconds });
    } else if (n === 1) {
      setDynI18n(span, "itemCountOne");
    } else {
      setDynI18n(span, "itemCount", { n });
    }
    chip.append(stack, span);
    if (opts.chevron) chip.appendChild(accordionChevron(opts.open ? "down" : "right"));
    if (opts.onClick) chip.addEventListener("click", opts.onClick);
    return chip;
  }

  function renderProcessSeg(seg, promptId, idx, turnLive, appearChip) {
    const key = (promptId || "") + ":" + idx;
    const live = !!(seg.live && turnLive);
    const wrap = document.createElement("div");
    wrap.className = "turn-trace" + (live ? " live" : "") + (live || (ctx.traceOpen && ctx.traceOpen.has(key)) ? " open" : "");
    wrap.dataset.seg = key;
    const lines = document.createElement("div");
    lines.className = "trace-lines";
    const chip = renderProcessChip(seg.nThought, seg.nTool, {
      live,
      seconds: live ? liveSeconds() : null,
      open: wrap.classList.contains("open"),
      chevron: !!seg.items.length,
      appear: !!(appearChip && live),
      onClick: (e) => {
        e.stopPropagation();
        if (!ctx.traceOpen) ctx.traceOpen = new Set();
        if (wrap.classList.contains("open")) {
          wrap.classList.remove("open");
          chip.classList.remove("open");
          ctx.traceOpen.delete(key);
        } else {
          wrap.classList.add("open");
          chip.classList.add("open");
          ctx.traceOpen.add(key);
          lines.scrollTop = live ? lines.scrollHeight : 0;
        }
      }
    });
    if (chip) wrap.appendChild(chip);
    for (const item of seg.items) {
      if (item.kind !== "tool") continue;
      for (const tr of item.tools) {
        const perm = ctx.pendingPerms && ctx.pendingPerms[tr.id];
        if (perm) wrap.appendChild(renderPerm(perm));
      }
    }
    for (const item of seg.items) {
      if (item.kind === "thought") lines.appendChild(renderTraceThought(item, promptId));
      else if (item.kind === "tool") lines.appendChild(renderTraceTool(item, promptId));
    }
    wrap.appendChild(lines);
    return wrap;
  }

  function renderCapsule(nThought, nTool, promptId) {
    return renderProcessChip(nThought, nTool, {
      onClick: () => {
        if (ctx.openDrawer) ctx.openDrawer(promptId, "");
      }
    });
  }

  function renderAssistantBody(text) {
    const el = document.createElement("article");
    el.className = "block assistant";
    const body = document.createElement("div");
    body.className = "block-body md";
    body.dataset.src = text;
    body.innerHTML = renderMarkdown(text);
    bindCodeCopy(body);
    el.appendChild(body);
    return el;
  }

  function renderFoot(nThought, nTool, promptId, copySrc, duration, appear) {
    const foot = document.createElement("div");
    foot.className = "seg-foot" + (appear ? " ui-in" : "");
    if (duration && duration.key) {
      const dur = document.createElement("div");
      dur.className = "trace-duration";
      setDynI18n(dur, duration.key, duration.vars);
      foot.appendChild(dur);
    }
    const bar = document.createElement("div");
    bar.className = "msg-actions" + (appear ? " ui-in" : "");
    if (copySrc) {
      const copy = document.createElement("button");
      copy.type = "button";
      copy.className = "icon-btn";
      setTip(copy, t("copy"));
      copy.setAttribute("data-i18n-title", "copy");
      copy.appendChild(svgUse("i-copy"));
      copy.addEventListener("click", () => copyText(copySrc));
      bar.appendChild(copy);
    }
    if (bar.childNodes.length) foot.appendChild(bar);
    return foot;
  }

  function renderPerm(perm) {
    const card = document.createElement("div");
    card.className = "perm-card";
    const titleEl = document.createElement("span");
    titleEl.className = "perm-title";
    titleEl.textContent = perm.title || t("allowTool");
    if (!perm.title) titleEl.setAttribute("data-i18n", "allowTool");
    const allow = document.createElement("button");
    allow.type = "button";
    allow.className = "perm-allow";
    allow.setAttribute("data-i18n", "allow");
    allow.textContent = t("allow");
    const deny = document.createElement("button");
    deny.type = "button";
    deny.className = "perm-deny";
    deny.setAttribute("data-i18n", "deny");
    deny.textContent = t("deny");
    allow.addEventListener("click", () => answerPerm(perm, true));
    deny.addEventListener("click", () => answerPerm(perm, false));
    card.append(titleEl, allow, deny);
    return card;
  }

  async function answerPerm(perm, allow) {
    try {
      await post(
        "/api/sessions/" + encodeURIComponent(ctx.currentId) + "/permissions/" + encodeURIComponent(perm.req),
        { allow }
      );
      if (ctx.pendingPerms) delete ctx.pendingPerms[perm.tool_id];
      scheduleRender();
    } catch (e) {
      toast(String(e.message || e));
    }
  }

  function nearBottom() {
    if (!timeline) return true;
    return timeline.scrollHeight - timeline.scrollTop - timeline.clientHeight < 80;
  }

  function turnHasProcess(turn) {
    return (turn && turn.agent || []).some((b) => b.type === "thought" || b.type === "tool");
  }

  function tickWorkSeconds() {
    if (!ctx.workStarted || !timeline) return;
    const n = liveSeconds();
    for (const el of timeline.querySelectorAll(".turn-trace.live .chip.process .chip-label")) {
      setDynI18n(el, "workedSeconds", { n });
    }
  }

  function syncWorkTimer(live) {
    if (live) {
      if (!ctx.workStarted) {
        const fromServer = ctx.current && ctx.current.work_started_ms;
        if (fromServer) {
          applyWorkStartedMs(fromServer);
          ctx.current.work_started_ms = 0;
        }
        if (!ctx.workStarted) ctx.workStarted = Date.now();
      }
      if (!ctx.workTimer) ctx.workTimer = setInterval(tickWorkSeconds, 1000);
    } else {
      ctx.workStarted = 0;
      if (ctx.current) ctx.current.work_started_ms = 0;
      if (ctx.workTimer) {
        clearInterval(ctx.workTimer);
        ctx.workTimer = 0;
      }
    }
  }

  function captureTraceScroll() {
    const map = {};
    if (!timeline) return map;
    for (const el of timeline.querySelectorAll(".turn-trace")) {
      const id = el.dataset.seg || "";
      const lines = el.querySelector(".trace-lines");
      if (!id || !lines) continue;
      const stick = lines.scrollHeight - lines.scrollTop - lines.clientHeight < 24;
      map[id] = { top: lines.scrollTop, stick };
    }
    return map;
  }

  function restoreTraceScroll(map) {
    if (!timeline) return;
    for (const el of timeline.querySelectorAll(".turn-trace.open")) {
      const id = el.dataset.seg || "";
      const lines = el.querySelector(".trace-lines");
      if (!id || !lines) continue;
      const prev = map && map[id];
      const live = el.classList.contains("live");
      if (prev?.stick || (!prev && live)) lines.scrollTop = lines.scrollHeight;
      else if (prev) lines.scrollTop = prev.top;
      else lines.scrollTop = 0;
    }
  }

  function agentStructureKey(split) {
    return split.segs
      .map((s) => {
        if (s.kind === "asst") return "a";
        if (s.kind !== "process") return s.kind;
        return (
          "p" +
          (s.live ? "L" : "") +
          ":" +
          s.items
            .map((it) => {
              if (it.kind === "thought") return it.live ? "T" : "t";
              return (it.running ? "R" : "r") + String((it.tools && it.tools.length) || 0);
            })
            .join(",")
        );
      })
      .join("|");
  }

  function fillAgentCol(right, turn, isLive, opts) {
    opts = opts || {};
    right.replaceChildren();
    const split = splitTrace(turn.agent, isLive);
    let segIdx = 0;
    const hasProcess = split.segs.some((s) => s.kind === "process");
    if (isLive && !hasProcess) {
      right.appendChild(
        renderProcessSeg(
          { kind: "process", items: [], nThought: 0, nTool: 0, live: true },
          turn.prompt_id,
          -1,
          true,
          opts.appearChip
        )
      );
    }
    for (let i = 0; i < split.segs.length; i++) {
      const seg = split.segs[i];
      if (seg.kind === "process") {
        right.appendChild(renderProcessSeg(seg, turn.prompt_id, segIdx, isLive, opts.appearChip));
        segIdx += 1;
      } else if (seg.kind === "asst") {
        right.appendChild(renderAssistantBody(seg.text));
      }
    }
    const copySrc = split.segs
      .filter((s) => s.kind === "asst")
      .map((s) => s.text)
      .join("\n\n");
    if (!isLive && (split.nThought || split.nTool || copySrc)) {
      const sec = turnSeconds(turn);
      const duration = turn.cancelled
        ? { key: "userStopped" }
        : split.nThought || split.nTool
          ? sec == null
            ? { key: "workDone" }
            : { key: "workedSeconds", vars: { n: sec } }
          : null;
      right.appendChild(renderFoot(split.nThought, split.nTool, turn.prompt_id, copySrc, duration, opts.appearFoot));
    }
    return split;
  }

  function renderTurnArticle(turn, isLive, opts) {
    opts = opts || {};
    const row = document.createElement("article");
    row.className = "turn";
    row.dataset.prompt = turn.prompt_id || "";
    row.dataset.kind = "turn";
    for (const u of turn.user) row.appendChild(renderUser(u, opts.appearUser));
    const right = document.createElement("div");
    right.className = "col col-agent";
    const split = fillAgentCol(right, turn, isLive, opts);
    row.dataset.agentFp = agentStructureKey(split);
    if (right.childNodes.length) row.appendChild(right);
    return row;
  }

  function patchLiveTurn(row, turn, split) {
    const items = [];
    for (const s of split.segs) {
      if (s.kind === "process") for (const it of s.items) items.push(it);
    }
    const thought = items.find((it) => it.kind === "thought" && it.live);
    const thoughtEl = row.querySelector(".trace-row.thought-live .shimmer-text") || row.querySelector(".trace-row.thought-live .trace-text");
    if (thought && thoughtEl) {
      const next = thoughtPreviewLines(thought.block.text, true).join("\n");
      if (thoughtEl.textContent !== next) thoughtEl.textContent = next;
    }
    const tool = items.find((it) => it.kind === "tool" && it.running);
    const toolEl = row.querySelector(".trace-row.tool-live .trace-text");
    if (tool && toolEl) {
      const first = tool.tools[0] || {};
      const preview = compactToolPreview(first.input_preview);
      setDynI18n(toolEl, "runningTool", { name: tool.name, preview: preview ? "  " + preview : "" });
    }
    const chipLabel = row.querySelector(".chip.process.live .chip-label");
    if (chipLabel) setDynI18n(chipLabel, "workedSeconds", { n: liveSeconds() });
    const asstSegs = split.segs.filter((s) => s.kind === "asst");
    const bodies = [...row.querySelectorAll(".col-agent .block.assistant .block-body.md")];
    if (asstSegs.length !== bodies.length) return false;
    asstSegs.forEach((seg, i) => {
      if (bodies[i].dataset.src === seg.text) return;
      bodies[i].dataset.src = seg.text;
      bodies[i].innerHTML = renderMarkdown(seg.text);
      bindCodeCopy(bodies[i]);
    });
    return true;
  }

  function syncTurnUsers(row, turn, appear) {
    if (!row || !turn) return;
    if (turn.prompt_id) row.dataset.prompt = turn.prompt_id;
    const users = turn.user || [];
    const existing = [...row.querySelectorAll(":scope > .user-row")];
    const have = existing.map((el) => {
      const body = el.querySelector(".say .block-body");
      return body ? body.textContent : "";
    });
    const texts = users.map((u) => String(u.text || ""));
    if (existing.length === users.length && texts.every((text, i) => have[i] === text)) return;
    const agent = row.querySelector(":scope > .col-agent");
    for (const el of existing) el.remove();
    for (const u of users) {
      const node = renderUser(u, appear);
      row.insertBefore(node, agent);
    }
  }

  function updateLastTurn(row, turn, isLive) {
    syncTurnUsers(row, turn, false);
    const wasLive = !!row.querySelector(".turn-trace.live");
    let col = row.querySelector(".col-agent");
    if (!isLive && !wasLive) return;
    if (!col) {
      col = document.createElement("div");
      col.className = "col col-agent";
      row.appendChild(col);
    }
    if (!isLive && wasLive) {
      const split = fillAgentCol(col, turn, false, { appearFoot: true });
      row.dataset.agentFp = agentStructureKey(split);
      return;
    }
    const split = splitTrace(turn.agent, true);
    const fp = agentStructureKey(split);
    if (row.dataset.agentFp === fp && patchLiveTurn(row, turn, split)) return;
    const hadChip = !!col.querySelector(".chip.process");
    const next = fillAgentCol(col, turn, true, { appearChip: !hadChip });
    row.dataset.agentFp = agentStructureKey(next);
  }

  function renderBlocks(detail) {
    if (!timeline) return;
    const stick = nearBottom();
    const tracePos = captureTraceScroll();
    if (!detail) {
      timeline.innerHTML = "";
      delete timeline.dataset.session;
      return;
    }
    if (actions) actions.hidden = false;
    if (app) app.classList.add("has-session");
    if (dl) {
      dl.href = "/api/sessions/" + encodeURIComponent(detail.id) + ".md";
      dl.setAttribute("download", detail.id + ".md");
    }
    if (ctx.renderUsage) ctx.renderUsage(detail.usage);
    if (ctx.applyContext) ctx.applyContext(detail.context);
    const turns = groupTurns(detail.blocks || []);
    const last = turns[turns.length - 1];
    const live = !!(last && !last.ended && ctx.running);
    syncWorkTimer(live);
    const sid = detail.id || "";
    if (timeline.querySelector(":scope > .empty")) timeline.innerHTML = "";
    const existing = [...timeline.querySelectorAll(":scope > article.turn")];
    const aligned =
      timeline.dataset.session === sid &&
      existing.length <= turns.length &&
      existing.every((el, i) => (el.dataset.prompt || "") === (turns[i].prompt_id || ""));
    if (!turns.length) {
      timeline.innerHTML = "";
      const empty = document.createElement("p");
      empty.className = "empty";
      empty.setAttribute("data-i18n", "emptySession");
      empty.textContent = t("emptySession");
      timeline.appendChild(empty);
    } else if (!aligned) {
      timeline.innerHTML = "";
      for (let idx = 0; idx < turns.length; idx++) {
        timeline.appendChild(renderTurnArticle(turns[idx], live && idx === turns.length - 1, {}));
      }
    } else {
      const lastIdx = turns.length - 1;
      if (existing.length && existing.length < turns.length) {
        const prev = existing[existing.length - 1];
        const prevTurn = turns[existing.length - 1];
        if (prev && prevTurn) updateLastTurn(prev, prevTurn, false);
      }
      for (let i = existing.length; i < turns.length; i++) {
        timeline.appendChild(renderTurnArticle(turns[i], live && i === lastIdx, { appearUser: true }));
      }
      if (existing.length === turns.length) {
        updateLastTurn(existing[lastIdx], turns[lastIdx], live);
      }
    }
    timeline.dataset.session = sid;
    if (ctx.drawerMode === "process" && ctx.drawerPromptId && ctx.renderDrawer) ctx.renderDrawer();
    restoreTraceScroll(tracePos);
    if (stick) timeline.scrollTop = timeline.scrollHeight;
  }

  function scheduleRender() {
    if (ctx.renderTimer) return;
    ctx.renderTimer = setTimeout(() => {
      ctx.renderTimer = 0;
      if (ctx.current) renderBlocks(ctx.current);
    }, 40);
  }

  function openTurnStart(blocks) {
    const list = blocks || [];
    for (let i = list.length - 1; i >= 0; i--) {
      if (list[i].type === "turn_end") return i + 1;
    }
    return 0;
  }

  function compactPendingUsers() {
    if (!ctx.current || !ctx.current.blocks) return;
    const out = [];
    for (const b of ctx.current.blocks) {
      if (b.type === "turn_end") {
        out.push(b);
        continue;
      }
      if (b.type === "user") {
        let prevIdx = -1;
        for (let i = out.length - 1; i >= 0; i--) {
          if (out[i].type === "turn_end") break;
          if (out[i].type === "user") {
            prevIdx = i;
            break;
          }
        }
        if (prevIdx >= 0 && String(out[prevIdx].text || "") === String(b.text || "")) {
          const prev = out[prevIdx];
          const prevPend = isPendingPrompt(prev.prompt_id);
          const nextPend = isPendingPrompt(b.prompt_id);
          if (prevPend && !nextPend) {
            const files = b.files && b.files.length ? b.files : prev.files;
            out[prevIdx] = Object.assign({}, prev, b, files ? { files } : {});
            continue;
          }
          if (!prevPend && nextPend) continue;
          if (prevPend && nextPend) continue;
        }
      }
      out.push(b);
    }
    ctx.current.blocks = out;
  }

  function upsertBlock(block) {
    if (!ctx.current) {
      ctx.current = { id: ctx.currentId, cwd: ctx.selectedCwd, title: ctx.currentId, blocks: [], usage: {} };
    }
    if (!ctx.current.blocks) ctx.current.blocks = [];
    if (block.type === "tool" && block.id) {
      const i = ctx.current.blocks.findIndex((b) => b.type === "tool" && b.id === block.id);
      if (i >= 0) ctx.current.blocks[i] = Object.assign({}, ctx.current.blocks[i], block);
      else ctx.current.blocks.push(block);
      return;
    }
    if (block.type === "user") {
      const openStart = openTurnStart(ctx.current.blocks);
      for (let i = ctx.current.blocks.length - 1; i >= openStart; i--) {
        const b = ctx.current.blocks[i];
        if (b.type !== "user") continue;
        const sameText = String(b.text || "") === String(block.text || "");
        const sameId = block.prompt_id && b.prompt_id === block.prompt_id;
        const pendingHit = (isPendingPrompt(b.prompt_id) || isPendingPrompt(block.prompt_id)) && sameText;
        if (sameId || pendingHit) {
          const prev = ctx.current.blocks[i];
          ctx.current.blocks[i] = Object.assign({}, prev, block);
          if (!(block.files && block.files.length) && prev.files && prev.files.length) {
            ctx.current.blocks[i].files = prev.files;
          }
          if (!isPendingPrompt(block.prompt_id) && isPendingPrompt(prev.prompt_id)) {
            ctx.current.blocks[i].prompt_id = block.prompt_id;
          }
          compactPendingUsers();
          return;
        }
      }
      let openHasUser = false;
      for (let i = openStart; i < ctx.current.blocks.length; i++) {
        if (ctx.current.blocks[i].type === "user") {
          openHasUser = true;
          break;
        }
      }
      const insertAt = openHasUser ? ctx.current.blocks.length : openStart;
      ctx.current.blocks.splice(insertAt, 0, block);
      compactPendingUsers();
      return;
    }
    if (block.type === "thought" || block.type === "assistant") {
      ctx.awaitingAgent = false;
      for (let i = ctx.current.blocks.length - 1; i >= 0; i--) {
        const b = ctx.current.blocks[i];
        if (b.type === block.type && (!block.prompt_id || b.prompt_id === block.prompt_id)) {
          ctx.current.blocks[i] = block;
          return;
        }
        if (b.type === "turn_end" || b.type === "user") break;
      }
      ctx.current.blocks.push(block);
      return;
    }
    if (block.type === "tool") ctx.awaitingAgent = false;
    ctx.current.blocks.push(block);
  }

  const copyTurnBtn = document.getElementById("copy-turn");
  if (copyTurnBtn) {
    copyTurnBtn.addEventListener("click", () => {
      if (!ctx.current) return;
      const pid = ctx.visiblePromptId ? ctx.visiblePromptId() : "";
      const parts = [];
      for (const b of ctx.current.blocks || []) {
        if (b.prompt_id !== pid) continue;
        if (b.type === "user" || b.type === "thought" || b.type === "assistant") parts.push(b.text);
      }
      copyText(parts.join("\n\n"));
    });
  }

  ctx.groupTurns = groupTurns;
  ctx.upsertBlock = upsertBlock;
  ctx.scheduleRender = scheduleRender;
  ctx.renderBlocks = renderBlocks;
  ctx.patchLiveTurn = patchLiveTurn;
  ctx.updateLastTurn = updateLastTurn;
  ctx.renderTurnArticle = renderTurnArticle;
  ctx.fillAgentCol = fillAgentCol;
  ctx.renderUser = renderUser;
  ctx.renderAssistantBody = renderAssistantBody;
  ctx.renderFoot = renderFoot;
  ctx.renderCapsule = renderCapsule;
  ctx.renderProcessChip = renderProcessChip;
  ctx.renderProcessSeg = renderProcessSeg;
  ctx.renderTraceThought = renderTraceThought;
  ctx.renderTraceTool = renderTraceTool;
  ctx.splitTrace = splitTrace;
  ctx.renderPerm = renderPerm;
  ctx.answerPerm = answerPerm;
  ctx.syncWorkTimer = syncWorkTimer;
  ctx.tickWorkSeconds = tickWorkSeconds;
  ctx.liveSeconds = liveSeconds;
  ctx.applyWorkStartedMs = applyWorkStartedMs;
  ctx.turnSeconds = turnSeconds;
  ctx.thoughtPreviewLines = thoughtPreviewLines;
  ctx.nearBottom = nearBottom;
  ctx.userActionBtn = userActionBtn;
  ctx.makeFileChip = makeFileChip;
  ctx.openFileChip = openFileChip;
  ctx.openFileLightbox = openFileLightbox;
  ctx.closeFileLightbox = closeFileLightbox;
  ctx.onFileLightboxKey = onFileLightboxKey;
  ctx.captureTraceScroll = captureTraceScroll;
  ctx.restoreTraceScroll = restoreTraceScroll;
  ctx.cancelledByUser = cancelledByUser;
  ctx.isPendingPrompt = isPendingPrompt;
  ctx.openTurnStart = openTurnStart;
  ctx.compactPendingUsers = compactPendingUsers;
}
