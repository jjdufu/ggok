import { t, isSpectatingSource } from "../lib/helpers.js";
import { toast } from "../lib/clipboard.js";

const STALE_MS = 12000;
const CONNECTING_MS = 8000;
const WATCH_MS = 2000;
const RECONNECT_GAP_MS = 2000;

export function bindSse(ctx) {
  function noteSseLive() {
    ctx.esLastLive = Date.now();
    ctx.esConnectingSince = 0;
  }

  function stopWatchdog() {
    if (ctx.esWatch) {
      clearInterval(ctx.esWatch);
      ctx.esWatch = 0;
    }
  }

  function closeStream() {
    if (ctx.esRetry) {
      clearTimeout(ctx.esRetry);
      ctx.esRetry = 0;
    }
    if (ctx.es) {
      ctx.es.close();
      ctx.es = null;
    }
  }

  function closeEvents() {
    stopWatchdog();
    closeStream();
  }

  function reconnectIfNeeded(id) {
    if (!id || id !== ctx.currentId) return;
    if (ctx.es && ctx.es.readyState === EventSource.OPEN) return;
    connectEvents(id);
  }

  function forceReconnect(id) {
    if (!id || id !== ctx.currentId) return;
    const now = Date.now();
    if (now - (ctx.esReconnectAt || 0) < RECONNECT_GAP_MS) return;
    ctx.esReconnectAt = now;
    connectEvents(id);
  }

  function armWatchdog() {
    if (ctx.esWatch) return;
    ctx.esWatch = setInterval(() => {
      const id = ctx.currentId;
      if (!id) return;
      const es = ctx.es;
      if (!es) {
        connectEvents(id);
        return;
      }
      const now = Date.now();
      if (es.readyState === EventSource.CONNECTING) {
        const since = ctx.esConnectingSince || ctx.esReconnectAt || now;
        if (now - since > CONNECTING_MS) forceReconnect(id);
        return;
      }
      if (es.readyState === EventSource.CLOSED) {
        forceReconnect(id);
        return;
      }
      if (!(ctx.running || ctx.awaitingAgent)) return;
      if (now - (ctx.esLastLive || 0) > STALE_MS) forceReconnect(id);
    }, WATCH_MS);
  }

  function connectEvents(id) {
    closeStream();
    ctx.esConnectingSince = Date.now();
    ctx.esLastLive = Date.now();
    ctx.esReconnectAt = Date.now();
    ctx.es = new EventSource("/api/sessions/" + encodeURIComponent(id) + "/events");
    const es = ctx.es;
    armWatchdog();

    const opened = new Promise((resolve) => {
      let done = false;
      const finish = () => {
        if (done) return;
        done = true;
        resolve();
      };
      es.addEventListener("open", () => {
        noteSseLive();
        finish();
        if (id === ctx.currentId && ctx.pullSession) ctx.pullSession(id).catch(() => {});
      });
      setTimeout(finish, 2500);
    });

    const on = (name, fn) =>
      es.addEventListener(name, (e) => {
        noteSseLive();
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

    on("ping", () => {});

    on("live", (ev) => {
      if (!ev || typeof ev !== "object") return;
      const prev = ctx.source;
      if (ev.source) ctx.source = ev.source;
      ctx.writable = ev.writable === true;
      if (typeof ev.running === "boolean") {
        if (isSpectatingSource(ctx.source)) {
          ctx.running = ev.running;
        } else if (ev.running) {
          ctx.running = true;
          ctx.awaitingAgent = false;
        } else if (!ctx.awaitingAgent) {
          ctx.running = false;
        }
      }
      if (ctx.current) {
        ctx.current.source = ctx.source;
        ctx.current.writable = ctx.writable;
      }
      if (ctx.running) {
        if (ctx.armWorkWatch) ctx.armWorkWatch();
      } else if (ctx.stopWorkWatch) {
        ctx.stopWorkWatch();
      }
      if (ctx.syncSendBtn) ctx.syncSendBtn();
      if (ctx.scheduleRender) ctx.scheduleRender();
      if (prev !== "attached" && ev.source === "attached" && id === ctx.currentId) {
        connectEvents(id);
      }
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
      if (ctx.refreshAccount) ctx.refreshAccount();
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
      if (es.readyState === EventSource.CONNECTING) {
        if (!ctx.esConnectingSince) ctx.esConnectingSince = Date.now();
        return;
      }
      if (es.readyState !== EventSource.CLOSED) return;
      if (ctx.esRetry) clearTimeout(ctx.esRetry);
      ctx.esRetry = setTimeout(() => {
        ctx.esRetry = 0;
        if (ctx.currentId === id) forceReconnect(id);
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

    on("questions", (list) => {
      if (ctx.applyQuestions) ctx.applyQuestions(Array.isArray(list) ? list : []);
    });

    on("rewind", () => {
      if (id === ctx.currentId && ctx.pullSession) ctx.pullSession(id).catch(() => {});
    });

    on("compact", (ev) => {
      ctx.compactPhase = ev && ev.phase;
      if (ctx.syncCompact) ctx.syncCompact(ev);
      if (ev && ev.phase === "done" && ctx.pullSession) ctx.pullSession(id).catch(() => {});
      if (ev && ev.phase === "error" && ev.message) toast(String(ev.message));
    });

    on("mode", (ev) => {
      if (ev && ev.mode) {
        ctx.mode = ev.mode;
        if (ctx.current) ctx.current.mode = ev.mode;
        if (ctx.syncModeBtn) ctx.syncModeBtn();
      }
    });

    on("todos", (list) => {
      if (ctx.renderTodos) ctx.renderTodos(Array.isArray(list) ? list : []);
    });

    on("tasks", () => {
      if (ctx.loadTasks) ctx.loadTasks().then(() => ctx.renderTasks && ctx.renderTasks());
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
