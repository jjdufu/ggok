import { t } from "../lib/helpers.js";
import { api, post } from "../lib/api.js";
import { toast } from "../lib/clipboard.js";

export function bindTasks(ctx) {
  async function loadTasks() {
    if (!ctx.currentId) return [];
    try {
      const data = await api("/api/sessions/" + encodeURIComponent(ctx.currentId) + "/tasks");
      ctx.tasks = (data && data.tasks) || [];
      return ctx.tasks;
    } catch (e) {
      ctx.tasks = [];
      toast(String(e.message || e));
      return [];
    }
  }

  async function killTask(tid) {
    if (!ctx.currentId || !tid) return;
    try {
      await post(
        "/api/sessions/" +
          encodeURIComponent(ctx.currentId) +
          "/tasks/" +
          encodeURIComponent(tid) +
          "/kill",
        {}
      );
      await loadTasks();
      if (ctx.renderTasks) ctx.renderTasks();
    } catch (e) {
      toast(String(e.message || e));
    }
  }

  function renderTasks() {
    const host = document.getElementById("tasks-list");
    if (!host) return;
    host.replaceChildren();
    const rows = ctx.tasks || [];
    if (!rows.length) {
      const empty = document.createElement("p");
      empty.className = "muted";
      empty.textContent = t("tasksEmpty");
      host.appendChild(empty);
      return;
    }
    rows.forEach((row) => {
      const item = document.createElement("div");
      item.className = "task-row";
      const title = document.createElement("div");
      title.className = "task-title";
      title.textContent = row.title || row.id;
      const st = document.createElement("div");
      st.className = "task-status";
      st.textContent = row.status || "";
      const kill = document.createElement("button");
      kill.type = "button";
      kill.className = "task-kill";
      kill.textContent = t("taskKill");
      kill.addEventListener("click", () => killTask(row.id));
      item.append(title, st, kill);
      host.appendChild(item);
    });
  }

  async function openTasks() {
    if (ctx.openDrawer) ctx.openDrawer("", "");
    const pane = document.getElementById("drawer-tasks");
    if (pane) pane.hidden = false;
    await loadTasks();
    renderTasks();
  }

  ctx.loadTasks = loadTasks;
  ctx.killTask = killTask;
  ctx.renderTasks = renderTasks;
  ctx.openTasks = openTasks;
}
