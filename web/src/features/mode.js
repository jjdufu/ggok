import { t, setTip, isSpectatingSource } from "../lib/helpers.js";
import { api, post } from "../lib/api.js";
import { toast, bindCodeCopy } from "../lib/clipboard.js";
import { openOverlay, closeOverlay } from "../lib/overlay.js";
import { placePopover } from "../lib/popover.js";
import { renderMarkdown } from "../lib/markdown.js";

const MODES = [
  ["ask", "modeAsk"],
  ["plan", "modePlan"],
  ["auto", "modeAuto"],
  ["always-approve", "modeAlways"]
];

function modeLabel(mode) {
  const hit = MODES.find(([id]) => id === mode);
  return hit ? t(hit[1]) : mode || t("modeAsk");
}

function todoMark(status) {
  if (status === "completed") return "✓";
  if (status === "in_progress") return "…";
  return "○";
}

function todoCurrent(items) {
  return (
    items.find((x) => x.status === "in_progress") ||
    items.find((x) => x.status !== "completed") ||
    items[items.length - 1] ||
    null
  );
}

export function bindMode(ctx) {
  const modeBtn = document.getElementById("mode-btn");
  const modeMenu = document.getElementById("mode-menu");

  function closeModeMenu() {
    if (modeMenu) modeMenu.hidden = true;
    if (modeBtn) modeBtn.setAttribute("aria-expanded", "false");
  }

  function pinModeMenu() {
    if (!modeBtn || !modeMenu || modeMenu.hidden) return;
    placePopover(modeMenu, modeBtn, {
      gap: 8,
      pad: 12,
      minH: 80,
      width: 168,
      align: "right",
      zIndex: 40
    });
  }

  function renderModeMenu() {
    if (!modeMenu) return;
    const cur = (ctx.current && ctx.current.mode) || ctx.mode || "ask";
    modeMenu.replaceChildren();
    MODES.forEach(([id, key]) => {
      const b = document.createElement("button");
      b.type = "button";
      b.setAttribute("role", "menuitem");
      b.className = "menu-item" + (cur === id ? " on" : "");
      b.textContent = t(key);
      b.addEventListener("click", (e) => {
        e.stopPropagation();
        closeModeMenu();
        setMode(id);
      });
      modeMenu.appendChild(b);
    });
    modeMenu.hidden = false;
    if (modeBtn) modeBtn.setAttribute("aria-expanded", "true");
    pinModeMenu();
  }

  function toggleModeMenu() {
    if (!ctx.currentId) return;
    if (modeMenu && !modeMenu.hidden) {
      closeModeMenu();
      return;
    }
    const modelMenu = document.getElementById("model-menu");
    if (modelMenu) modelMenu.hidden = true;
    renderModeMenu();
  }

  function syncModeBtn() {
    if (!modeBtn) return;
    const mode = (ctx.current && ctx.current.mode) || ctx.mode || "ask";
    ctx.mode = mode;
    modeBtn.textContent = modeLabel(mode);
    setTip(modeBtn, t("modeTip"));
    const spectating = isSpectatingSource(ctx.source);
    modeBtn.hidden = !ctx.currentId;
    modeBtn.disabled = !ctx.currentId || spectating || ctx.writable !== true;
    if (!ctx.currentId || modeBtn.disabled) closeModeMenu();
  }

  async function setMode(mode) {
    if (!ctx.currentId) return;
    try {
      const out = await post("/api/sessions/" + encodeURIComponent(ctx.currentId) + "/mode", { mode });
      const next = (out && out.mode) || mode;
      ctx.mode = next;
      if (ctx.current) ctx.current.mode = next;
      syncModeBtn();
      if (modeMenu && !modeMenu.hidden) renderModeMenu();
      if (next === "plan" && ctx.openPlan) ctx.openPlan();
    } catch (e) {
      toast(String(e.message || e));
    }
  }

  function renderTodos(todos) {
    const bar = document.getElementById("todos-bar");
    if (!bar) return;
    const items = Array.isArray(todos) ? todos : [];
    ctx.todos = items;
    if (!ctx.currentId || !items.length) {
      ctx.todosOpen = false;
      bar.hidden = true;
      bar.classList.remove("open");
      bar.replaceChildren();
      return;
    }
    const done = items.filter((x) => x.status === "completed").length;
    const current = todoCurrent(items);
    bar.hidden = false;
    bar.classList.toggle("open", !!ctx.todosOpen);
    bar.replaceChildren();

    const toggle = document.createElement("button");
    toggle.type = "button";
    toggle.className = "todos-toggle";
    toggle.setAttribute("aria-expanded", ctx.todosOpen ? "true" : "false");
    toggle.setAttribute("aria-haspopup", "true");
    const title = document.createElement("span");
    title.className = "todos-title";
    title.textContent = t("todosProgress", { done, total: items.length });
    toggle.appendChild(title);
    if (current && current.content) {
      const preview = document.createElement("span");
      preview.className = "todos-preview";
      preview.textContent = current.content;
      toggle.appendChild(preview);
    }
    toggle.addEventListener("click", (e) => {
      e.stopPropagation();
      ctx.todosOpen = !ctx.todosOpen;
      renderTodos(ctx.todos);
    });
    bar.appendChild(toggle);

    const list = document.createElement("ul");
    list.className = "todos-list";
    items.forEach((item) => {
      const li = document.createElement("li");
      li.className = "todo-item " + String(item.status || "pending");
      const mark = document.createElement("span");
      mark.className = "todo-mark";
      mark.textContent = todoMark(item.status);
      const text = document.createElement("span");
      text.className = "todo-text";
      text.textContent = item.content || "";
      li.append(mark, text);
      list.appendChild(li);
    });
    bar.appendChild(list);
  }

  async function openPlan() {
    if (!ctx.currentId) return;
    try {
      const data = await api("/api/sessions/" + encodeURIComponent(ctx.currentId) + "/plan");
      const scrim = document.getElementById("plan-scrim");
      const panel = document.getElementById("plan-panel");
      const body = document.getElementById("plan-md");
      if (body) {
        body.innerHTML = renderMarkdown(String((data && data.markdown) || t("planEmpty")));
        bindCodeCopy(body);
      }
      openOverlay(scrim, panel);
    } catch (e) {
      toast(String(e.message || e));
    }
  }

  function closePlan() {
    const scrim = document.getElementById("plan-scrim");
    const panel = document.getElementById("plan-panel");
    closeOverlay(scrim, { panel });
  }

  async function approvePlan() {
    const perms = ctx.pendingPerms || {};
    const exit = Object.values(perms).find((p) => {
      const hay = String((p && (p.title || "")) + " " + JSON.stringify(p.options || [])).toLowerCase();
      return hay.includes("exit_plan_mode") || hay.includes("plan");
    });
    if (exit && exit.options && exit.options.length) {
      const allow = exit.options.find((o) => String(o.kind || "").includes("allow")) || exit.options[0];
      try {
        await post(
          "/api/sessions/" + encodeURIComponent(ctx.currentId) + "/permissions/" + encodeURIComponent(exit.req),
          { option_id: allow.id }
        );
        closePlan();
        return;
      } catch (e) {
        toast(String(e.message || e));
        return;
      }
    }
    await setMode("ask");
    closePlan();
  }

  function revisePlan() {
    closePlan();
    if (ctx.focusPrompt) ctx.focusPrompt();
    toast(t("planReviseHint"));
  }

  if (modeBtn) {
    modeBtn.addEventListener("click", (e) => {
      e.stopPropagation();
      toggleModeMenu();
    });
  }
  function closeTodos() {
    if (!ctx.todosOpen) return;
    ctx.todosOpen = false;
    renderTodos(ctx.todos);
  }

  document.addEventListener("click", (e) => {
    if (modeMenu && !modeMenu.hidden && !modeMenu.contains(e.target) && (!modeBtn || !modeBtn.contains(e.target))) {
      closeModeMenu();
    }
    const bar = document.getElementById("todos-bar");
    if (ctx.todosOpen && bar && !bar.contains(e.target)) closeTodos();
  });
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") closeTodos();
  });
  window.addEventListener("resize", pinModeMenu);
  window.addEventListener("scroll", pinModeMenu, true);

  const scrim = document.getElementById("plan-scrim");
  if (scrim) scrim.addEventListener("click", closePlan);
  const closeBtn = document.getElementById("plan-close");
  if (closeBtn) closeBtn.addEventListener("click", closePlan);
  const approveBtn = document.getElementById("plan-approve");
  if (approveBtn) approveBtn.addEventListener("click", approvePlan);
  const reviseBtn = document.getElementById("plan-revise");
  if (reviseBtn) reviseBtn.addEventListener("click", revisePlan);
  const exitBtn = document.getElementById("plan-exit");
  if (exitBtn) {
    exitBtn.addEventListener("click", () => {
      setMode("ask");
      closePlan();
    });
  }

  ctx.syncModeBtn = syncModeBtn;
  ctx.setMode = setMode;
  ctx.renderTodos = renderTodos;
  ctx.openPlan = openPlan;
  ctx.closePlan = closePlan;
  ctx.closeModeMenu = closeModeMenu;
  ctx.renderModeMenu = renderModeMenu;
  ctx.modeLabel = modeLabel;
}
