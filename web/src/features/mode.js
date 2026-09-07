import { t, setTip } from "../lib/helpers.js";
import { api, post } from "../lib/api.js";
import { toast, bindCodeCopy } from "../lib/clipboard.js";
import { openOverlay, closeOverlay } from "../lib/overlay.js";
import { renderMarkdown } from "../lib/markdown.js";

const CYCLE = ["ask", "plan", "auto", "always-approve"];

function modeLabel(mode) {
  const key = {
    ask: "modeAsk",
    plan: "modePlan",
    auto: "modeAuto",
    "always-approve": "modeAlways"
  }[mode];
  return key ? t(key) : mode || t("modeAsk");
}

export function bindMode(ctx) {
  function syncModeBtn() {
    const btn = document.getElementById("mode-btn");
    if (!btn) return;
    const mode = (ctx.current && ctx.current.mode) || ctx.mode || "ask";
    ctx.mode = mode;
    btn.textContent = modeLabel(mode);
    setTip(btn, t("modeTip"));
    btn.hidden = !ctx.currentId;
  }

  async function setMode(mode) {
    if (!ctx.currentId) return;
    try {
      const out = await post("/api/sessions/" + encodeURIComponent(ctx.currentId) + "/mode", { mode });
      const next = (out && out.mode) || mode;
      ctx.mode = next;
      if (ctx.current) ctx.current.mode = next;
      syncModeBtn();
      if (next === "plan" && ctx.openPlan) ctx.openPlan();
    } catch (e) {
      toast(String(e.message || e));
    }
  }

  function cycleMode() {
    const cur = (ctx.current && ctx.current.mode) || ctx.mode || "ask";
    const i = CYCLE.indexOf(cur);
    const next = CYCLE[(i + 1) % CYCLE.length];
    setMode(next);
  }

  function renderTodos(todos) {
    const bar = document.getElementById("todos-bar");
    if (!bar) return;
    const items = Array.isArray(todos) ? todos : [];
    ctx.todos = items;
    if (!items.length) {
      bar.hidden = true;
      bar.replaceChildren();
      return;
    }
    bar.hidden = false;
    bar.replaceChildren();
    const title = document.createElement("div");
    title.className = "todos-title";
    title.textContent = t("todosTitle");
    bar.appendChild(title);
    const list = document.createElement("ul");
    list.className = "todos-list";
    items.forEach((item) => {
      const li = document.createElement("li");
      li.className = "todo-item " + String(item.status || "pending");
      const mark = document.createElement("span");
      mark.className = "todo-mark";
      mark.textContent = item.status === "completed" ? "✓" : item.status === "in_progress" ? "…" : "○";
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
      if (data && data.todos) renderTodos(data.todos);
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

  const modeBtn = document.getElementById("mode-btn");
  if (modeBtn) modeBtn.addEventListener("click", cycleMode);
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
  ctx.cycleMode = cycleMode;
  ctx.renderTodos = renderTodos;
  ctx.openPlan = openPlan;
  ctx.closePlan = closePlan;
  ctx.modeLabel = modeLabel;
}
