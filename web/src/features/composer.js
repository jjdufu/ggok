import { promptApi } from "../promptApi.js";
import { t, setTip, fileNameOf, uploadUrl, fileViewSrc, isImageAttach, revokePreview, isSpectatingSource, occupyMessageKey } from "../lib/helpers.js";
import { bindDraftSync } from "./draft-sync.js";
import { placePopover } from "../lib/popover.js";
import { svgUse } from "../lib/svg.js";
import { api, post } from "../lib/api.js";
import { escapeHtml } from "../lib/markdown.js";
import { toast } from "../lib/clipboard.js";

export function bindComposer(ctx) {
  const { TUI_ONLY_SLASH } = ctx;
  const app = document.getElementById("app");
  const actions = document.getElementById("actions");
  const timeline = document.getElementById("timeline");
  const composer = document.getElementById("composer");
  const sendBtn = document.getElementById("send-btn");
  const sendIcon = document.getElementById("send-icon");
  const modelBtn = document.getElementById("model-btn");
  const dirBtn = document.getElementById("dir-btn");
  const slashMenu = document.getElementById("slash-menu");
  const atMenu = document.getElementById("at-menu");
  const queueEl = document.getElementById("queue");
  const chipsEl = document.getElementById("chips");
  const fileInput = document.getElementById("file-input");
  const stageEl = document.getElementById("stage");
  const ctxBar = document.getElementById("ctx-bar");
  const promptPh = document.getElementById("prompt-ph");

  const promptPhKeys = ["promptPh1", "promptPh2", "promptPh3"];
  let promptPhI = 0;
  const drafts = bindDraftSync(ctx);

  function promptIsEmpty() {
    return !String(promptApi.getText() || "").trim();
  }

  function paintPromptPh(animate) {
    if (!promptPh) return;
    const skill = ctx.activeSkill;
    const hide = !promptIsEmpty() || promptApi.isFocused();
    promptPh.hidden = hide;
    if (hide) return;
    if (skill) promptPh.textContent = skill.hint || t("skillChipPh");
    else promptPh.textContent = t(promptPhKeys[promptPhI % promptPhKeys.length]);
    if (animate) {
      promptPh.classList.remove("swap");
      void promptPh.offsetWidth;
      promptPh.classList.add("swap");
    }
  }

  if (promptPh) {
    paintPromptPh(false);
    promptApi.onChange(() => paintPromptPh(false));
    promptApi.onFocus(() => paintPromptPh(false));
    promptApi.onBlur(() => paintPromptPh(false));
    setInterval(() => {
      if (!promptIsEmpty() || promptApi.isFocused()) return;
      promptPhI += 1;
      paintPromptPh(true);
    }, 8000);
  }

  function fillComposer(text) {
    const next = String(text || "");
    promptApi.setText(next);
    promptApi.focus();
    promptApi.setCaret(next.length);
    drafts.publish();
  }

  function visiblePromptId() {
    if (!ctx.current || !timeline) return "";
    const top = timeline.scrollTop + 64;
    const nodes = [...timeline.querySelectorAll(".turn[data-prompt]")];
    let pid = nodes[0] ? nodes[0].dataset.prompt : "";
    for (const n of nodes) {
      if (n.offsetTop <= top) pid = n.dataset.prompt;
      else break;
    }
    return pid;
  }

  function isSpectating() {
    return isSpectatingSource(ctx.source);
  }

  function syncOccupyBanner() {
    const banner = document.getElementById("occupy-banner");
    if (!banner) return;
    const key = occupyMessageKey(ctx.source);
    if (key) {
      banner.hidden = false;
      banner.textContent = t(key);
    } else {
      banner.hidden = true;
      banner.textContent = "";
    }
  }

  function syncSendBtn() {
    if (!sendBtn) return;
    const spectating = isSpectating();
    const attachedRunning = ctx.source === "attached" && !!ctx.running;
    const canWrite = ctx.writable === true && !spectating;
    sendBtn.disabled = spectating || (!canWrite && !attachedRunning);
    sendBtn.classList.toggle("stopping", attachedRunning);
    const sendTip = attachedRunning ? t("stop") : t("send");
    setTip(sendBtn, sendTip);
    sendBtn.setAttribute("aria-label", sendTip);
    if (sendIcon) sendIcon.dataset.state = attachedRunning ? "b" : "a";
    if (modelBtn) modelBtn.disabled = spectating || ctx.writable !== true;
    const attachBtn = document.getElementById("attach-btn");
    if (attachBtn) attachBtn.disabled = spectating;
    if (composer) {
      const inner = composer.querySelector(".composer-inner");
      if (inner) inner.classList.toggle("spectating", spectating);
    }
    const editor = document.getElementById("prompt-editor");
    if (editor) {
      editor.setAttribute("contenteditable", spectating ? "false" : "true");
      editor.setAttribute("aria-disabled", spectating ? "true" : "false");
    }
    if (queueEl) {
      queueEl.querySelectorAll("button, textarea").forEach((el) => {
        el.disabled = spectating;
      });
    }
    syncOccupyBanner();
  }

  function clearAttachments() {
    for (const f of ctx.attachments || []) revokePreview(f);
    ctx.attachments = [];
  }

  function skillChipLabel(sk) {
    if (!sk) return "";
    const key = "skillQ_" + String(sk.name || "").trim().replace(/-/g, "_");
    const s = t(key);
    if (s && s !== key) return s;
    return sk.label || sk.name || "";
  }

  function renderSkillChip() {
    const sk = ctx.activeSkill;
    if (!sk || !sk.name) return null;
    const chip = document.createElement("span");
    chip.className = "file-chip skill-chip";
    const label = document.createElement("span");
    label.className = "file-chip-name";
    label.textContent = skillChipLabel(sk);
    chip.appendChild(label);
    const x = document.createElement("button");
    x.type = "button";
    x.setAttribute("aria-label", t("close"));
    x.appendChild(svgUse("i-x"));
    x.addEventListener("click", (e) => {
      e.stopPropagation();
      ctx.activeSkill = null;
      renderChips();
      paintPromptPh(false);
    });
    chip.appendChild(x);
    return chip;
  }

  function renderChips() {
    if (!chipsEl) return;
    chipsEl.replaceChildren();
    const skillChip = renderSkillChip();
    const files = ctx.attachments || [];
    chipsEl.hidden = !skillChip && !files.length;
    if (skillChip) chipsEl.appendChild(skillChip);
    for (const f of files) {
      if (ctx.makeFileChip) chipsEl.appendChild(ctx.makeFileChip(f, true));
    }
  }

  function applyActiveSkill(text) {
    const sk = ctx.activeSkill;
    if (!sk || !sk.name) return text;
    if (sk.kind !== "slash") return text;
    const raw = String(text || "").trim();
    if (raw.startsWith("/")) return text;
    return raw ? "/" + sk.name + " " + raw : "/" + sk.name;
  }

  function filesFromDataTransfer(dt) {
    if (!dt) return [];
    const out = [];
    const seen = new Set();
    const add = (file) => {
      if (!file || !file.size) return;
      const key = [file.size, file.type || "", file.name || ""].join("\0");
      if (seen.has(key)) return;
      seen.add(key);
      out.push(file);
    };
    if (dt.files && dt.files.length) {
      for (const file of dt.files) add(file);
      return out;
    }
    if (dt.items && dt.items.length) {
      for (const item of dt.items) {
        if (item.kind === "file") add(item.getAsFile());
      }
    }
    return out;
  }

  function clipboardPlainText(dt) {
    if (!dt) return "";
    try {
      return dt.getData("text/plain") || "";
    } catch (e) {
      return "";
    }
  }

  async function uploadFiles(fileList) {
    const dir = ctx.selectedCwd || (ctx.current && ctx.current.cwd);
    if (!dir) {
      ctx.dirPath = "";
      if (ctx.renderDirModal) ctx.renderDirModal();
      toast(t("pickCwdFirst"));
      return;
    }
    if (!ctx.attachments) ctx.attachments = [];
    for (const file of fileList) {
      const pending = { name: file.name, mime: file.type, processing: true };
      ctx.attachments.push(pending);
      renderChips();
      try {
        const fd = new FormData();
        fd.append("cwd", dir);
        fd.append("file", file, file.name);
        const res = await fetch("/api/uploads", { method: "POST", body: fd, credentials: "same-origin" });
        if (res.status === 401) {
          location.href = "/login";
          return;
        }
        if (!res.ok) throw new Error(await res.text());
        const row = await res.json();
        row.url = uploadUrl(row);
        if (isImageAttach({ mime: row.mime || file.type, name: row.name || file.name })) {
          row.preview = row.url;
        }
        const i = ctx.attachments.indexOf(pending);
        if (i >= 0) ctx.attachments[i] = row;
        else ctx.attachments.push(row);
      } catch (e) {
        ctx.attachments = ctx.attachments.filter((a) => a !== pending);
        toast(String(e.message || e));
      }
      renderChips();
    }
  }

  function applyQueue(list) {
    ctx.queue = Array.isArray(list) ? list : [];
    renderQueue();
  }

  function queueUrl(item) {
    return "/api/sessions/" + encodeURIComponent(ctx.currentId) + "/queue/" + encodeURIComponent(item.id);
  }

  function restoreQueueFiles(item) {
    const files = item && Array.isArray(item.files) ? item.files : [];
    if (!ctx.attachments) ctx.attachments = [];
    for (const f of files) {
      if (!f || !f.path) continue;
      if (ctx.attachments.some((a) => a.path === f.path)) continue;
      const name = String(f.path).split("/").pop() || f.path;
      ctx.attachments.push({
        path: f.path,
        mime: f.mime || "",
        name,
        rel: f.path
      });
    }
    if (files.length) renderChips();
  }

  function isGoneQueueErr(err) {
    return /queue item not found/i.test(String((err && err.message) || err || ""));
  }

  async function sendQueueNow(item) {
    if (isSpectating() || ctx.writable !== true) return;
    if (!ctx.currentId || !item || !item.id) return;
    const snapshot = (ctx.queue || []).slice();
    applyQueue((ctx.queue || []).filter((q) => q.id !== item.id));
    try {
      const out = await post(queueUrl(item) + "/send", {});
      if (Array.isArray(out)) applyQueue(out);
      ctx.running = true;
      syncSendBtn();
    } catch (err) {
      if (isGoneQueueErr(err)) return;
      applyQueue(snapshot);
      toast(String(err.message || err));
    }
  }

  async function dropQueueItem(item) {
    if (!ctx.currentId || !item || !item.id) return null;
    try {
      const out = await api(queueUrl(item), { method: "DELETE" });
      applyQueue(out);
      return true;
    } catch (err) {
      if (isGoneQueueErr(err)) {
        applyQueue((ctx.queue || []).filter((q) => q.id !== item.id));
        return true;
      }
      toast(String(err.message || err));
      return false;
    }
  }

  async function recallQueueItem(item) {
    const ok = await dropQueueItem(item);
    if (!ok) return;
    fillComposer(item.text || "");
    restoreQueueFiles(item);
  }

  function renderQueue() {
    if (!queueEl) return;
    queueEl.replaceChildren();
    queueEl.hidden = !(ctx.queue && ctx.queue.length);
    for (const item of ctx.queue || []) {
      const row = document.createElement("div");
      row.className = "queue-item";
      const head = document.createElement("div");
      head.className = "queue-head";
      const text = document.createElement("div");
      text.className = "queue-text";
      if (item.text) text.textContent = item.text;
      else {
        text.setAttribute("data-i18n", "emptyQueue");
        text.textContent = t("emptyQueue");
      }
      const recall = document.createElement("button");
      recall.type = "button";
      recall.className = "queue-recall";
      recall.setAttribute("data-i18n", "recallQueue");
      recall.textContent = t("recallQueue");
      recall.addEventListener("click", (e) => {
        e.stopPropagation();
        recallQueueItem(item);
      });
      const now = document.createElement("button");
      now.type = "button";
      now.className = "queue-now";
      now.setAttribute("data-i18n", "sendNow");
      now.textContent = t("sendNow");
      now.addEventListener("click", (e) => {
        e.stopPropagation();
        sendQueueNow(item);
      });
      const delBtn = document.createElement("button");
      delBtn.type = "button";
      delBtn.className = "icon-btn";
      delBtn.appendChild(svgUse("i-x"));
      delBtn.addEventListener("click", (e) => {
        e.stopPropagation();
        dropQueueItem(item);
      });
      head.append(text, recall, now, delBtn);
      row.appendChild(head);
      let open = false;
      text.style.cursor = "pointer";
      text.addEventListener("click", () => {
        open = !open;
        let ta = row.querySelector("textarea");
        if (open) {
          if (!ta) {
            ta = document.createElement("textarea");
            ta.value = item.text || "";
            ta.addEventListener("change", async () => {
              try {
                applyQueue(
                  await api(queueUrl(item), {
                    method: "PATCH",
                    headers: { "Content-Type": "application/json" },
                    body: JSON.stringify({ text: ta.value })
                  })
                );
              } catch (err) {
                toast(String(err.message || err));
              }
            });
            row.appendChild(ta);
          }
        } else if (ta) ta.remove();
      });
      queueEl.appendChild(row);
    }
  }

  function commandNames(c) {
    return [c.name, ...((c && c.aliases) || [])].filter(Boolean);
  }

  function findCommand(token) {
    const q = String(token || "").replace(/^\//, "").toLowerCase();
    if (!q) return null;
    return ((ctx.runtime && ctx.runtime.commands) || []).find((c) => commandNames(c).some((n) => String(n).toLowerCase() === q)) || null;
  }

  function filteredCommands(q) {
    const rest = String(q || "").replace(/^\//, "").toLowerCase();
    const cmds = (ctx.runtime && ctx.runtime.commands) || [];
    if (!rest) return cmds.slice(0, 20);
    const prefix = [];
    const includes = [];
    for (const c of cmds) {
      const names = commandNames(c).map((n) => String(n).toLowerCase());
      if (names.some((n) => n.startsWith(rest))) prefix.push(c);
      else if (names.some((n) => n.includes(rest))) includes.push(c);
    }
    return prefix.concat(includes).slice(0, 20);
  }

  function slashToken(text) {
    const raw = String(text || "").trim();
    if (!raw.startsWith("/")) return null;
    const body = raw.slice(1);
    const sp = body.search(/\s/);
    if (sp < 0) return { name: body, args: "" };
    return { name: body.slice(0, sp), args: body.slice(sp + 1).trim() };
  }

  function openModelPicker() {
    if (ctx.openModelMenu) ctx.openModelMenu();
  }

  function applySlashModel(args) {
    const parts = String(args || "").trim().split(/\s+/).filter(Boolean);
    if (!parts.length) {
      openModelPicker();
      return true;
    }
    const models = (ctx.runtime && ctx.runtime.models) || [];
    const raw = parts[0];
    const lower = raw.toLowerCase();
    const m =
      models.find((x) => x.id === raw) ||
      models.find((x) => String(x.id || "").toLowerCase() === lower) ||
      models.find((x) => String(x.name || "").toLowerCase() === lower);
    if (!m) return false;
    let effort = "";
    if (parts[1] && ctx.matchEffort) effort = ctx.matchEffort(m, parts[1]);
    if (!effort) effort = ctx.selectedEffort || m.effort || ((m.efforts || [])[0] && m.efforts[0].id) || "";
    if (ctx.pickModel) ctx.pickModel(m.id, effort);
    return true;
  }

  function applySlashEffort(args) {
    const id = String(args || "").trim().split(/\s+/)[0];
    if (!id || !ctx.selectedModel) return false;
    const m = ctx.modelById ? ctx.modelById(ctx.selectedModel) : null;
    const effort = ctx.matchEffort ? ctx.matchEffort(m, id) : "";
    if (!effort) return false;
    if (ctx.pickModel) ctx.pickModel(ctx.selectedModel, effort);
    return true;
  }

  function slashI18nSlug(name) {
    return String(name || "")
      .trim()
      .replace(/[^a-zA-Z0-9]+/g, "_")
      .replace(/^_+|_+$/g, "");
  }

  function slashI18nLookup(name) {
    const slug = slashI18nSlug(name);
    if (!slug) return "";
    const key = "slash_" + slug;
    const s = t(key);
    if (s && s !== key) return s;
    const qKey = "skillQ_" + slug + "Desc";
    const q = t(qKey);
    if (q && q !== qKey) return q;
    return "";
  }

  function slashI18nDesc(c) {
    const name = String((c && c.name) || "").trim();
    let hit = slashI18nLookup(name);
    if (hit) return hit;
    if (name.includes(":")) {
      hit = slashI18nLookup(name.split(":").pop());
      if (hit) return hit;
    }
    return String((c && (c.description || c.hint)) || "");
  }

  function updateMenuPlacement(menu) {
    if (!menu || menu.hidden) return;
    const wrap = menu.closest(".composer-wrap") || document.querySelector(".composer-wrap");
    if (!wrap) return;
    placePopover(menu, wrap, {
      gap: 8,
      pad: 12,
      minH: 120,
      maxH: 320,
      width: wrap.getBoundingClientRect().width,
      align: "left",
      zIndex: 26
    });
  }

  function renderSlash() {
    if (!slashMenu) return;
    const v = promptApi.getText();
    if (!v.startsWith("/") || v.includes("\n") || v.includes(" ")) {
      slashMenu.hidden = true;
      return;
    }
    const cmds = filteredCommands(v);
    slashMenu.replaceChildren();
    if (!cmds.length) {
      slashMenu.hidden = true;
      return;
    }
    if (ctx.slashIdx >= cmds.length) ctx.slashIdx = 0;
    cmds.forEach((c, i) => {
      const b = document.createElement("button");
      b.type = "button";
      b.className = "menu-item" + (i === ctx.slashIdx ? " active" : "");
      const desc = slashI18nDesc(c);
      const nm = document.createElement("span");
      nm.className = "menu-name";
      nm.textContent = "/" + (c.name || "");
      const ds = document.createElement("span");
      ds.className = "menu-desc";
      ds.textContent = desc;
      ds.setAttribute("data-tip-overflow-target", "");
      b.setAttribute("data-tip-overflow", "");
      b.setAttribute("data-tip-side", "top");
      setTip(b, desc);
      b.append(nm, ds);
      b.addEventListener("click", (e) => {
        e.stopPropagation();
        pickSlash(c, true);
      });
      slashMenu.appendChild(b);
    });
    slashMenu.hidden = false;
    updateMenuPlacement(slashMenu);
    const on = slashMenu.querySelector(".menu-item.active");
    if (on && on.scrollIntoView) on.scrollIntoView({ block: "nearest" });
  }

  function currentSlashChoice() {
    const cmds = filteredCommands(promptApi.getText());
    return cmds[ctx.slashIdx] || cmds[0];
  }

  function pickSlash(cmd, sendIfPlain) {
    const name = cmd.name;
    if (slashMenu) slashMenu.hidden = true;
    if (handleLocalSlash("/" + name)) {
      promptApi.setText("");
      return;
    }
    if (cmd.hint && !sendIfPlain) {
      promptApi.setText("/" + name + " ");
      promptApi.focus();
      return;
    }
    promptApi.setText("/" + name + (cmd.hint ? " " : ""));
    if (!cmd.hint) {
      submitPrompt();
    } else {
      promptApi.focus();
    }
  }

  function handleLocalSlash(text) {
    const tok = slashToken(text);
    if (!tok) return false;
    const cmd = findCommand(tok.name);
    const canon = ((cmd && cmd.name) || tok.name).toLowerCase();
    const names = new Set(
      [canon, tok.name.toLowerCase(), ...commandNames(cmd || { name: canon, aliases: [] }).map((n) => String(n).toLowerCase())]
    );
    const hit = (...list) => list.some((n) => names.has(n));
    if (hit("model", "m")) return applySlashModel(tok.args);
    if (hit("effort") && applySlashEffort(tok.args)) return true;
    if (hit("usage", "cost")) {
      if (ctx.setStatusOpen) ctx.setStatusOpen(true);
      return true;
    }
    if (hit("new", "clear")) {
      startNewChat();
      return true;
    }
    if (hit("session-info", "status", "info")) {
      if (ctx.setInfoOpen) ctx.setInfoOpen(true, "info");
      return true;
    }
    if (hit("context")) {
      if (ctx.setInfoOpen) ctx.setInfoOpen(true, "context");
      return true;
    }
    if (hit("resume")) return true;
    if (hit("mcps", "mcp")) {
      if (ctx.openExtModal) ctx.openExtModal("mcp");
      return true;
    }
    if (hit("plugins", "plugin")) {
      if (ctx.openExtModal) ctx.openExtModal("plugins");
      return true;
    }
    if (hit("marketplace")) {
      if (ctx.openExtModal) ctx.openExtModal("marketplace");
      return true;
    }
    if (hit("skills", "skill")) {
      if (ctx.openExtModal) ctx.openExtModal("skills");
      return true;
    }
    if (TUI_ONLY_SLASH && (TUI_ONLY_SLASH.has(canon) || TUI_ONLY_SLASH.has(tok.name.toLowerCase()))) {
      toast(t("slashTuiOnly"));
      return true;
    }
    return false;
  }

  function parseAtToken(value, caret) {
    const pos = caret == null ? String(value || "").length : caret;
    const left = String(value || "").slice(0, pos);
    const m = left.match(/(^|[\s])(@!?[^\s]*)$/);
    if (!m) return null;
    const raw = m[2];
    const start = pos - raw.length;
    const bang = raw.startsWith("@!");
    let body = bang ? raw.slice(2) : raw.slice(1);
    let range = "";
    const rm = body.match(/^(.*):(\d+(?:-\d+)?)$/);
    if (rm && !body.endsWith("/")) {
      body = rm[1];
      range = rm[2];
    }
    return { start, end: pos, bang, query: body, range, raw };
  }

  function hideAtMenu() {
    if (atMenu) atMenu.hidden = true;
    ctx.atRows = [];
    ctx.atIdx = 0;
  }

  function replaceAtToken(row) {
    const v = promptApi.getText();
    const tok = parseAtToken(v, promptApi.getCaret());
    if (!tok || !row) return;
    const rel = String(row.path || row.name || "").replace(/\\/g, "/");
    let ins = "@" + (tok.bang ? "!" : "") + rel;
    if (row.dir && !ins.endsWith("/")) ins += "/";
    if (tok.range && !row.dir) ins += ":" + tok.range;
    if (!row.dir) ins += " ";
    promptApi.setText(v.slice(0, tok.start) + ins + v.slice(tok.end));
    const pos = tok.start + ins.length;
    promptApi.setCaret(pos);
    promptApi.focus();
    if (row.dir) scheduleAtMenu();
    else hideAtMenu();
  }

  function insertAtRef(row) {
    if (!row) return;
    const rel = String(row.path || row.name || "").replace(/\\/g, "/");
    const hidden = rel.split("/").some((seg) => seg.startsWith("."));
    let ins = (hidden ? "@!" : "@") + rel;
    if (row.dir && !ins.endsWith("/")) ins += "/";
    if (!row.dir) ins += " ";
    const v = promptApi.getText();
    const pos = promptApi.getCaret();
    const before = v.slice(0, pos);
    if (before.length && !/\s$/.test(before)) ins = " " + ins;
    promptApi.setText(before + ins + v.slice(pos));
    promptApi.setCaret(pos + ins.length);
    promptApi.focus();
  }

  function renderAtItems(rows) {
    if (!atMenu) return;
    ctx.atRows = rows || [];
    atMenu.replaceChildren();
    if (!ctx.atRows.length) {
      const empty = document.createElement("div");
      empty.className = "menu-item";
      empty.textContent = t("atNoMatch");
      empty.style.pointerEvents = "none";
      empty.style.color = "var(--muted)";
      atMenu.appendChild(empty);
      atMenu.hidden = false;
      updateMenuPlacement(atMenu);
      return;
    }
    if (ctx.atIdx >= ctx.atRows.length) ctx.atIdx = 0;
    if (ctx.atIdx < 0) ctx.atIdx = 0;
    ctx.atRows.forEach((row, i) => {
      const b = document.createElement("button");
      b.type = "button";
      b.className = "menu-item" + (i === ctx.atIdx ? " active" : "");
      const rel = String(row.path || row.name || "");
      const label = rel + (row.dir && !rel.endsWith("/") ? "/" : "");
      b.innerHTML = "<span class='menu-name'>" + escapeHtml(label) + "</span>";
      b.addEventListener("click", (e) => {
        e.stopPropagation();
        replaceAtToken(row);
      });
      atMenu.appendChild(b);
    });
    atMenu.hidden = false;
    updateMenuPlacement(atMenu);
    const on = atMenu.querySelector(".menu-item.active");
    if (on && on.scrollIntoView) on.scrollIntoView({ block: "nearest" });
  }

  function scheduleAtMenu() {
    clearTimeout(ctx.atTimer);
    const tok = parseAtToken(promptApi.getText(), promptApi.getCaret());
    if (!tok) {
      hideAtMenu();
      return;
    }
    if (slashMenu) slashMenu.hidden = true;
    const cwd = ctx.selectedCwd || (ctx.current && ctx.current.cwd);
    if (!cwd) {
      hideAtMenu();
      return;
    }
    const seq = ++ctx.atSeq;
    const q = (tok.bang ? "!" : "") + tok.query;
    ctx.atTimer = setTimeout(() => {
      api("/api/fs?cwd=" + encodeURIComponent(cwd) + "&q=" + encodeURIComponent(q))
        .then((rows) => {
          if (seq !== ctx.atSeq) return;
          renderAtItems(Array.isArray(rows) ? rows : []);
        })
        .catch((err) => {
          if (seq !== ctx.atSeq) return;
          hideAtMenu();
          toast(String(err.message || err));
        });
    }, 40);
  }

  function startNewChat() {
    leaveSession();
    drafts.load();
    promptApi.focus();
  }

  function stopWorkWatch() {
    if (ctx.workWatch) {
      clearInterval(ctx.workWatch);
      ctx.workWatch = 0;
    }
  }

  function armWorkWatch() {
    stopWorkWatch();
    ctx.workWatch = setInterval(() => {
      if (!ctx.running || !ctx.currentId) {
        stopWorkWatch();
        return;
      }
      pullSession(ctx.currentId).catch(() => {});
      if (ctx.es && ctx.es.readyState === EventSource.CLOSED && ctx.connectEvents) {
        ctx.connectEvents(ctx.currentId);
      }
    }, 1600);
  }

  function leaveSession() {
    drafts.flush();
    if (ctx.closeEvents) ctx.closeEvents();
    ctx.current = null;
    ctx.currentId = null;
    ctx.running = false;
    ctx.awaitingAgent = false;
    ctx.source = "disk";
    ctx.writable = true;
    stopWorkWatch();
    if (ctx.syncWorkTimer) ctx.syncWorkTimer(false);
    ctx.queue = [];
    ctx.pendingPerms = {};
    location.hash = "";
    if (ctx.setPageTitle) ctx.setPageTitle("");
    else document.title = "GGOK";
    if (app) app.classList.remove("has-session");
    if (actions) actions.hidden = true;
    if (timeline) timeline.innerHTML = "";
    if (ctx.applyContext) ctx.applyContext({ used: 0, window: 0 });
    if (ctxBar) ctxBar.hidden = true;
    renderQueue();
    syncSendBtn();
    if (ctx.closeDrawer) ctx.closeDrawer();
    if (ctx.renderTree) ctx.renderTree();
    ctx.selectedCwd = "";
    if (ctx.syncDirLabel) ctx.syncDirLabel();
    if (ctx.syncWsButton) ctx.syncWsButton();
    promptApi.setText("");
    ctx.activeSkill = null;
    renderChips();
  }

  function applyOccupancy(detail) {
    if (!detail) return;
    if (detail.source) ctx.source = detail.source;
    ctx.writable = detail.writable === true;
    if (ctx.current) {
      ctx.current.source = ctx.source;
      ctx.current.writable = ctx.writable;
    }
    if (typeof detail.running === "boolean") {
      if (isSpectatingSource(ctx.source)) {
        ctx.running = detail.running;
      } else if (detail.running) {
        ctx.running = true;
        ctx.awaitingAgent = false;
      } else if (!ctx.awaitingAgent) {
        ctx.running = false;
      }
    }
  }

  async function pullSession(id) {
    if (!id || id !== ctx.currentId) return;
    try {
      const detail = await api("/api/sessions/" + encodeURIComponent(id));
      if (id !== ctx.currentId) return;
      if (!ctx.current) ctx.current = detail;
      else {
        if (Array.isArray(detail.blocks)) {
          const localBlocks = ctx.current.blocks || [];
          const openStart = ctx.openTurnStart ? ctx.openTurnStart(localBlocks) : 0;
          const pendingUsers = [];
          for (let i = openStart; i < localBlocks.length; i++) {
            const b = localBlocks[i];
            if (b.type === "user" && String(b.prompt_id || "").startsWith("pending-")) {
              const serverOpenStart = ctx.openTurnStart ? ctx.openTurnStart(detail.blocks) : 0;
              const exists = detail.blocks.slice(serverOpenStart).some(
                (sb) => sb.type === "user" && String(sb.text || "") === String(b.text || "")
              );
              if (!exists) {
                pendingUsers.push(b);
              }
            }
          }
          ctx.current.blocks = detail.blocks.concat(pendingUsers);
          if (ctx.compactPendingUsers) ctx.compactPendingUsers();
        }
        if (detail.usage) ctx.current.usage = detail.usage;
        if (detail.work_started_ms) ctx.current.work_started_ms = detail.work_started_ms;
      }
      applyOccupancy(detail);
      syncSendBtn();
      if (ctx.scheduleRender) ctx.scheduleRender();
    } catch (e) {}
  }

  async function openSession(id) {
    drafts.flush();
    if (ctx.closeDirModal) ctx.closeDirModal();
    if (ctx.syncWorkTimer) ctx.syncWorkTimer(false);
    ctx.currentId = id;
    location.hash = id;
    ctx.drawerDetailCache = {};
    if (ctx.closeDrawer) ctx.closeDrawer();
    if (ctx.renderTree) ctx.renderTree();
    if (timeline) timeline.innerHTML = "";
    try {
      const detail = await api("/api/sessions/" + encodeURIComponent(id));
      ctx.current = detail;
      ctx.selectedCwd = detail.cwd || ctx.selectedCwd;
      if (ctx.syncDirLabel) ctx.syncDirLabel();
      if (ctx.syncWsButton) ctx.syncWsButton();
      ctx.writable = detail.writable === true;
      ctx.source = detail.source || "disk";
      if (detail.model) ctx.selectedModel = detail.model;
      if (detail.effort) ctx.selectedEffort = detail.effort;
      if (ctx.fillModels) ctx.fillModels();
      if (ctx.setPageTitle) ctx.setPageTitle(detail.title || id);
      else document.title = (detail.title || id) + " · GGOK";
      applyOccupancy(detail);
      syncSendBtn();
      drafts.load();
      if (ctx.renderBlocks) ctx.renderBlocks(detail);
      if (ctx.connectEvents) await ctx.connectEvents(id);
      await pullSession(id);
      try {
        applyQueue(await api("/api/sessions/" + encodeURIComponent(id) + "/queue"));
      } catch (e) {
        applyQueue([]);
      }
    } catch (e) {
      if (app) app.classList.add("has-session");
      if (actions) actions.hidden = true;
      if (timeline) {
        timeline.innerHTML = "";
        const p = document.createElement("p");
        p.className = "empty error";
        p.textContent = t("loadFailed", { e: String(e.message || e) });
        timeline.appendChild(p);
      }
    }
  }

  async function ensureSession() {
    if (ctx.currentId) return ctx.currentId;
    const dir = ctx.selectedCwd || (ctx.current && ctx.current.cwd);
    if (!dir) {
      ctx.dirPath = "";
      if (ctx.renderDirModal) ctx.renderDirModal();
      throw new Error(t("pickCwdFirst"));
    }
    const s = await post("/api/sessions", {
      cwd: dir,
      model: ctx.selectedModel || undefined,
      effort: ctx.selectedEffort || undefined
    });
    ctx.selectedCwd = s.cwd || dir;
    if (ctx.syncDirLabel) ctx.syncDirLabel();
    if (ctx.syncWsButton) ctx.syncWsButton();
    ctx.currentId = s.id;
    const existingBlocks = (ctx.current && ctx.current.blocks) ? ctx.current.blocks : [];
    ctx.current = {
      id: s.id,
      cwd: s.cwd,
      title: s.id,
      model: s.model,
      effort: s.effort,
      blocks: existingBlocks,
      usage: {},
      writable: true,
      source: "attached"
    };
    ctx.writable = true;
    ctx.source = "attached";
    if (s.model) ctx.selectedModel = s.model;
    if (s.effort) ctx.selectedEffort = s.effort;
    if (ctx.fillModels) ctx.fillModels();
    location.hash = s.id;
    if (app) app.classList.add("has-session");
    if (ctx.connectEvents) await ctx.connectEvents(s.id);
    if (ctx.loadList) ctx.loadList().catch(() => {});
    if (ctx.renderBlocks) ctx.renderBlocks(ctx.current);
    pullSession(s.id).catch(() => {});
    return s.id;
  }

  async function submitPrompt() {
    if (isSpectating() || ctx.writable !== true) {
      toast(t(occupyMessageKey(ctx.source) || "sessionBusy"));
      return;
    }
    const text = applyActiveSkill(promptApi.getText());
    if (!text.trim() && !(ctx.attachments && ctx.attachments.length)) {
      if (ctx.running && ctx.queue && ctx.queue.length) {
        await sendQueueNow(ctx.queue[0]);
      }
      return;
    }
    if (handleLocalSlash(text)) {
      promptApi.setText("");
      return;
    }
    const dir = ctx.selectedCwd || (ctx.current && ctx.current.cwd);
    if (!dir && !ctx.currentId) {
      ctx.dirPath = "";
      if (ctx.renderDirModal) ctx.renderDirModal();
      toast(t("pickCwdFirst"));
      return;
    }
    if ((ctx.attachments || []).some((a) => a.processing || !a.path)) {
      toast(t("processing"));
      return;
    }

    const files = (ctx.attachments || []).map((a) => ({ path: a.path, mime: a.mime }));
    const shown = (ctx.attachments || []).map((a) => ({
      path: a.path,
      mime: a.mime,
      name: a.name || a.rel || fileNameOf(a),
      preview: fileViewSrc(a)
    }));

    promptApi.setText("");
    drafts.clear();
    clearAttachments();
    renderChips();
    if (slashMenu) slashMenu.hidden = true;

    if (!ctx.current) {
      ctx.current = {
        id: "",
        cwd: dir || "",
        title: "",
        model: ctx.selectedModel || "grok-4.6",
        effort: ctx.selectedEffort || "high",
        blocks: [],
        usage: {},
        writable: true,
        source: "disk"
      };
    }
    if (app) app.classList.add("has-session");
    if (text.trim() || shown.length) {
      if (ctx.upsertBlock) {
        ctx.upsertBlock({ type: "user", prompt_id: "pending-" + Date.now(), text: text, files: shown });
      }
      if (ctx.scheduleRender) ctx.scheduleRender();
    }
    ctx.awaitingAgent = true;
    ctx.running = true;
    syncSendBtn();
    if (ctx.scheduleRender) ctx.scheduleRender();
    armWorkWatch();

    try {
      const id = await ensureSession();
      if (!ctx.es || ctx.es.readyState === EventSource.CLOSED) {
        if (ctx.connectEvents) await ctx.connectEvents(id);
      }
      const out = await post("/api/sessions/" + encodeURIComponent(id) + "/prompt", { text, files });
      if (out && out.queued) {
        ctx.awaitingAgent = false;
        ctx.queue = out.queue || [];
        renderQueue();
      } else {
        ctx.running = true;
        syncSendBtn();
        if (ctx.scheduleRender) ctx.scheduleRender();
      }
      pullSession(id).catch(() => {});
    } catch (e) {
      ctx.awaitingAgent = false;
      ctx.running = false;
      stopWorkWatch();
      syncSendBtn();
      if (ctx.scheduleRender) ctx.scheduleRender();
      toast(String(e.message || e));
    }
  }

  let composing = false;
  let enterAfterIme = false;

  promptApi.onChange(() => {
    const tok = parseAtToken(promptApi.getText(), promptApi.getCaret());
    if (tok) {
      if (slashMenu) slashMenu.hidden = true;
      scheduleAtMenu();
    } else {
      hideAtMenu();
      renderSlash();
    }
  });

  promptApi.onCompositionStart(() => {
    composing = true;
  });

  promptApi.onCompositionEnd(() => {
    composing = false;
    enterAfterIme = true;
    setTimeout(() => {
      enterAfterIme = false;
    }, 0);
  });

  function imeBlocked(e) {
    return composing || enterAfterIme || e.isComposing || e.keyCode === 229 || e.key === "Process";
  }

  function insertPromptNewline() {
    const sel = promptApi.getSelection();
    const start = sel.start;
    const end = sel.end;
    const v = promptApi.getText();
    promptApi.setText(v.slice(0, start) + "\n" + v.slice(end));
    promptApi.setCaret((start || 0) + 1);
  }

  promptApi.onKeyDown((e) => {
    if (ctx.dirModalOpen && ctx.dirModalOpen()) {
      e.preventDefault();
      return;
    }
    if (atMenu && !atMenu.hidden && (e.key === "ArrowDown" || e.key === "ArrowUp")) {
      e.preventDefault();
      const n = (ctx.atRows || []).length;
      if (!n) return;
      ctx.atIdx = (ctx.atIdx + (e.key === "ArrowDown" ? 1 : n - 1)) % n;
      renderAtItems(ctx.atRows);
      return;
    }
    if (atMenu && !atMenu.hidden && (e.key === "Tab" || (e.key === "Enter" && !e.shiftKey && !e.altKey))) {
      if (e.key === "Enter" && imeBlocked(e)) return;
      e.preventDefault();
      if (!ctx.atRows || !ctx.atRows.length) return;
      replaceAtToken(ctx.atRows[ctx.atIdx] || ctx.atRows[0]);
      return;
    }
    if (slashMenu && !slashMenu.hidden && (e.key === "ArrowDown" || e.key === "ArrowUp")) {
      e.preventDefault();
      const n = slashMenu.children.length;
      ctx.slashIdx = (ctx.slashIdx + (e.key === "ArrowDown" ? 1 : n - 1)) % n;
      renderSlash();
      return;
    }
    if (slashMenu && !slashMenu.hidden && (e.key === "Tab" || (e.key === "Enter" && !e.shiftKey && !e.altKey))) {
      if (e.key === "Enter" && imeBlocked(e)) return;
      e.preventDefault();
      const cmd = currentSlashChoice();
      if (cmd) pickSlash(cmd, e.key === "Enter");
      return;
    }
    if (e.key !== "Enter") return;
    if (imeBlocked(e)) return;
    if (e.shiftKey) return;
    if (e.altKey) {
      e.preventDefault();
      insertPromptNewline();
      return;
    }
    e.preventDefault();
    submitPrompt();
  });

  if (sendBtn) {
    sendBtn.addEventListener("click", (e) => {
      e.stopPropagation();
      if (isSpectating()) return;
      if (ctx.source === "attached" && ctx.running) {
        ctx.running = false;
        ctx.awaitingAgent = false;
        stopWorkWatch();
        if (ctx.current && ctx.current.blocks && ctx.current.blocks.length) {
          const last = ctx.current.blocks[ctx.current.blocks.length - 1];
          last.cancelled = true;
          if (last.prompt_id && ctx.cancelledByUser) ctx.cancelledByUser[last.prompt_id] = true;
        }
        syncSendBtn();
        if (ctx.scheduleRender) ctx.scheduleRender();
        post("/api/sessions/" + encodeURIComponent(ctx.currentId) + "/cancel", {}).catch((err) =>
          toast(String(err.message || err))
        );
        return;
      }
      submitPrompt();
    });
  }

  const attachBtn = document.getElementById("attach-btn");
  if (attachBtn && fileInput) {
    attachBtn.addEventListener("click", (e) => {
      e.stopPropagation();
      fileInput.click();
    });
  }

  if (fileInput) {
    fileInput.addEventListener("change", () => {
      uploadFiles([...fileInput.files]);
      fileInput.value = "";
    });
  }

  let dragDepth = 0;

  function hasFileDrag(e) {
    const types = e.dataTransfer && e.dataTransfer.types;
    if (!types) return false;
    return [...types].includes("Files");
  }

  function setDropping(on) {
    if (stageEl) stageEl.classList.toggle("dropping", on);
    if (composer) composer.classList.toggle("dropping", on);
  }

  function onDragOver(e) {
    if (!hasFileDrag(e)) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = "copy";
  }

  function onDragEnter(e) {
    if (!hasFileDrag(e)) return;
    e.preventDefault();
    dragDepth += 1;
    setDropping(true);
  }

  function onDragLeave(e) {
    if (!hasFileDrag(e)) return;
    dragDepth = Math.max(0, dragDepth - 1);
    if (!dragDepth) setDropping(false);
  }

  function onDrop(e) {
    if (!hasFileDrag(e)) return;
    e.preventDefault();
    dragDepth = 0;
    setDropping(false);
    const files = filesFromDataTransfer(e.dataTransfer);
    if (files.length) uploadFiles(files);
  }

  const dropRoot = stageEl || composer;
  if (dropRoot) {
    dropRoot.addEventListener("dragover", onDragOver);
    dropRoot.addEventListener("dragenter", onDragEnter);
    dropRoot.addEventListener("dragleave", onDragLeave);
    dropRoot.addEventListener("drop", onDrop);
  }

  let lastPasteAt = 0;
  let lastPasteKey = "";
  if (composer) {
    composer.addEventListener("paste", (e) => {
      const files = filesFromDataTransfer(e.clipboardData);
      if (!files.length) return;
      const key = files.map((f) => [f.size, f.type || "", f.name || ""].join("\0")).join("|");
      const now = Date.now();
      if (key && key === lastPasteKey && now - lastPasteAt < 500) return;
      lastPasteKey = key;
      lastPasteAt = now;
      if (!clipboardPlainText(e.clipboardData)) e.preventDefault();
      uploadFiles(files);
    });
  }

  if (slashMenu) slashMenu.addEventListener("click", (e) => e.stopPropagation());
  if (atMenu) atMenu.addEventListener("click", (e) => e.stopPropagation());

  window.addEventListener("resize", () => {
    if (slashMenu && !slashMenu.hidden) updateMenuPlacement(slashMenu);
    if (atMenu && !atMenu.hidden) updateMenuPlacement(atMenu);
  });

  const newSessionBtn = document.getElementById("new-session");
  if (newSessionBtn) {
    newSessionBtn.addEventListener("click", startNewChat);
  }

  ctx.fillComposer = fillComposer;
  ctx.focusPrompt = () => promptApi.focus();
  ctx.submitPrompt = submitPrompt;
  ctx.ensureSession = ensureSession;
  ctx.startNewChat = startNewChat;
  ctx.leaveSession = leaveSession;
  ctx.openSession = openSession;
  ctx.pullSession = pullSession;
  ctx.applyOccupancy = applyOccupancy;
  ctx.stopWorkWatch = stopWorkWatch;
  ctx.armWorkWatch = armWorkWatch;
  ctx.syncSendBtn = syncSendBtn;
  ctx.promptIsEmpty = promptIsEmpty;
  ctx.paintPromptPh = paintPromptPh;
  ctx.uploadFiles = uploadFiles;
  ctx.clearAttachments = clearAttachments;
  ctx.renderChips = renderChips;
  ctx.filesFromDataTransfer = filesFromDataTransfer;
  ctx.clipboardPlainText = clipboardPlainText;
  ctx.hasFileDrag = hasFileDrag;
  ctx.setDropping = setDropping;
  ctx.onDragOver = onDragOver;
  ctx.onDragEnter = onDragEnter;
  ctx.onDragLeave = onDragLeave;
  ctx.onDrop = onDrop;
  ctx.applyQueue = applyQueue;
  ctx.queueUrl = queueUrl;
  ctx.restoreQueueFiles = restoreQueueFiles;
  ctx.isGoneQueueErr = isGoneQueueErr;
  ctx.sendQueueNow = sendQueueNow;
  ctx.dropQueueItem = dropQueueItem;
  ctx.recallQueueItem = recallQueueItem;
  ctx.renderQueue = renderQueue;
  ctx.commandNames = commandNames;
  ctx.findCommand = findCommand;
  ctx.filteredCommands = filteredCommands;
  ctx.slashToken = slashToken;
  ctx.openModelPicker = openModelPicker;
  ctx.applySlashModel = applySlashModel;
  ctx.applySlashEffort = applySlashEffort;
  ctx.slashI18nSlug = slashI18nSlug;
  ctx.slashI18nLookup = slashI18nLookup;
  ctx.slashI18nDesc = slashI18nDesc;
  ctx.renderSlash = renderSlash;
  ctx.currentSlashChoice = currentSlashChoice;
  ctx.pickSlash = pickSlash;
  ctx.handleLocalSlash = handleLocalSlash;
  ctx.parseAtToken = parseAtToken;
  ctx.hideAtMenu = hideAtMenu;
  ctx.replaceAtToken = replaceAtToken;
  ctx.insertAtRef = insertAtRef;
  ctx.renderAtItems = renderAtItems;
  ctx.scheduleAtMenu = scheduleAtMenu;
  ctx.imeBlocked = imeBlocked;
  ctx.insertPromptNewline = insertPromptNewline;
  ctx.visiblePromptId = visiblePromptId;
}
