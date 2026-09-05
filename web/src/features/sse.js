import { t } from "../lib/helpers.js";
import { toast } from "../lib/clipboard.js";

export function bindSse(ctx) {
  function closeEvents() {
    if (ctx.esRetry) {
      clearTimeout(ctx.esRetry);
      ctx.esRetry = 0;
    }
    if (ctx.es) {
      ctx.es.close();
      ctx.es = null;
    }
  }

  function reconnectIfNeeded(id) {
    if (!id || id !== ctx.currentId) return;
    if (ctx.es && ctx.es.readyState === EventSource.OPEN) return;
    connectEvents(id);
  }

  function connectEvents(id) {
    closeEvents();
    ctx.es = new EventSource("/api/sessions/" + encodeURIComponent(id) + "/events");
    const es = ctx.es;

    const opened = new Promise((resolve) => {
      let done = false;
      const finish = () => {
        if (done) return;
        done = true;
        resolve();
      };
      es.addEventListener("open", () => {
        finish();
        if (id === ctx.currentId && ctx.pullSession) ctx.pullSession(id).catch(() => {});
      });
      setTimeout(finish, 2500);
    });

    const on = (name, fn) =>
      es.addEventListener(name, (e) => {
        if (e.data == null || e.data === "") return;
        try {
          fn(JSON.parse(e.data));
        } catch (err) {
          fn(e.data);
        }
      });

    on("block", (block) => {
      if (ctx.upsertBlock) ctx.upsertBlock(block);
      if (ctx.scheduleRender) ctx.scheduleRender();
    });

    on("queue", (list) => {
      if (ctx.applyQueue) ctx.applyQueue(list);
    });

    on("live", (ev) => {
      if (!ev || typeof ev !== "object") return;
      if (ev.source) ctx.source = ev.source;
      ctx.writable = ev.writable !== false;
      if (ev.source === "agent") {
        if (ev.running) {
          ctx.running = true;
          ctx.awaitingAgent = false;
        } else if (!ctx.awaitingAgent) {
          ctx.running = false;
        }
      } else if (ev.source === "cli") {
        if (!ctx.awaitingAgent) ctx.running = false;
      }
      if (ctx.current) {
        ctx.current.source = ctx.source;
        ctx.current.writable = ctx.writable;
      }
      if (ctx.syncSendBtn) ctx.syncSendBtn();
      if (ctx.scheduleRender) ctx.scheduleRender();
    });

    on("resync", () => {
      if (id === ctx.currentId && ctx.pullSession) ctx.pullSession(id).catch(() => {});
    });

    on("done", () => {
      ctx.awaitingAgent = false;
      ctx.running = false;
      if (ctx.stopWorkWatch) ctx.stopWorkWatch();
      if (ctx.syncSendBtn) ctx.syncSendBtn();
      if (ctx.loadList) ctx.loadList();
      if (ctx.refreshSessionUsage) ctx.refreshSessionUsage();
      if (ctx.scheduleRender) ctx.scheduleRender();
    });

    on("error", (ev) => {
      if (!ev || (typeof ev === "string" && !String(ev).trim())) return;
      ctx.awaitingAgent = false;
      ctx.running = false;
      if (ctx.stopWorkWatch) ctx.stopWorkWatch();
      if (ctx.syncSendBtn) ctx.syncSendBtn();
      toast((ev && ev.message) || t("agentError"));
      if (ctx.scheduleRender) ctx.scheduleRender();
    });

    es.addEventListener("error", () => {
      if (es !== ctx.es || ctx.currentId !== id) return;
      if (es.readyState !== EventSource.CLOSED) return;
      if (ctx.esRetry) clearTimeout(ctx.esRetry);
      ctx.esRetry = setTimeout(() => {
        ctx.esRetry = 0;
        if (ctx.currentId === id) connectEvents(id);
      }, 1500);
    });

    on("usage", (usage) => {
      if (ctx.current) {
        const prev = ctx.current.usage;
        if (prev && prev.recorded && prev.total_tokens && usage && !usage.total_tokens) {
          return;
        }
        ctx.current.usage = usage;
      }
      if (ctx.renderUsage) ctx.renderUsage((ctx.current && ctx.current.usage) || usage);
    });

    on("context", (c) => {
      if (ctx.applyContext) ctx.applyContext(c);
    });

    on("model", (ev) => {
      if (ev && ev.model) {
        ctx.selectedModel = ev.model;
        if (ev.effort) ctx.selectedEffort = ev.effort;
        if (ctx.current) {
          ctx.current.model = ev.model;
          if (ev.effort) ctx.current.effort = ev.effort;
        }
        if (ctx.fillModels) ctx.fillModels();
      }
    });

    on("commands", (list) => {
      if (Array.isArray(list) && ctx.runtime) ctx.runtime.commands = list;
      if (ctx.renderSlash) ctx.renderSlash();
    });

    on("title", (ev) => {
      if (ev && ev.title && ctx.current) {
        ctx.current.title = ev.title;
        if (ctx.setPageTitle) ctx.setPageTitle(ev.title);
        const row = ctx.sessions && ctx.sessions.find((s) => s.id === ctx.currentId);
        if (row) row.title = ev.title;
        if (ctx.renderTree) ctx.renderTree();
      }
    });

    on("permission", (ev) => {
      if (ev && ev.tool_id) {
        if (!ctx.pendingPerms) ctx.pendingPerms = {};
        ctx.pendingPerms[ev.tool_id] = ev;
      }
      if (ctx.scheduleRender) ctx.scheduleRender();
    });

    return opened;
  }

  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState !== "visible" || !ctx.currentId) return;
    reconnectIfNeeded(ctx.currentId);
    if (ctx.pullSession) ctx.pullSession(ctx.currentId).catch(() => {});
  });
  window.addEventListener("pageshow", () => {
    if (!ctx.currentId) return;
    reconnectIfNeeded(ctx.currentId);
    if (ctx.pullSession) ctx.pullSession(ctx.currentId).catch(() => {});
  });

  ctx.connectEvents = connectEvents;
  ctx.closeEvents = closeEvents;
}
