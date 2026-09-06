import { bindTheme } from "./features/theme.js";
import { bindConfirm } from "./lib/confirm.js";
import { bindSidebar } from "./features/sidebar.js";
import { bindModelMenu } from "./features/model-menu.js";
import { bindFinder } from "./features/finder.js";
import { bindDirModal } from "./features/dir-modal.js";
import { bindDrawer } from "./features/drawer.js";
import { bindWorkspace } from "./features/workspace.js";
import { bindQuota } from "./features/quota.js";
import { bindVersion } from "./features/version.js";
import { bindExtModal } from "./features/ext-modal.js";
import { bindSse } from "./features/sse.js";
import { bindTimeline } from "./features/timeline.js";
import { bindQuestion } from "./features/question.js";
import { bindComposer } from "./features/composer.js";
import { api } from "./lib/api.js";
import { relocalizeDyn, hideTip, formatError } from "./lib/helpers.js";

export function boot() {
  const tree = document.getElementById("tree");
  if (!tree) return;

  const THEME_KEY = "ggok-theme";
  const SIDE_KEY = "ggok-sidebar";
  const FINDER_PREVIEW_KEY = "ggok-finder-preview";

  const TUI_ONLY_SLASH = new Set([
    "quit",
    "exit",
    "home",
    "welcome",
    "multiline",
    "ml",
    "vim-mode",
    "minimal",
    "fullscreen",
    "theme",
    "timestamps",
    "dashboard",
    "agents-dashboard",
    "edit-prompt",
    "history"
  ]);

  const EFFORT_I18N = {
    low: { label: "effortLow", desc: "effortLowDesc" },
    medium: { label: "effortMedium", desc: "effortMediumDesc" },
    high: { label: "effortHigh", desc: "effortHighDesc" },
    xhigh: { label: "effortXhigh", desc: "effortXhighDesc" }
  };

  const ctx = {
    THEME_KEY,
    SIDE_KEY,
    FINDER_PREVIEW_KEY,
    TUI_ONLY_SLASH,
    EFFORT_I18N,

    sessions: [],
    currentId: null,
    current: null,
    running: false,
    awaitingAgent: false,
    source: "disk",
    writable: true,

    selectedModel: "grok-4.6",
    selectedEffort: "high",
    selectedCwd: "",
    pinned: false,
    hostUser: "",

    mcpData: { servers: [], sources: [] },
    mcpBusy: false,
    pluginData: { plugins: [], sources: [] },
    pluginBusy: false,
    pluginBusyOp: "",
    pluginBusyName: "",
    personalSkills: [],
    skillBusy: false,
    extTab: "mcp",
    extQuery: "",
    extOpen: "",
    extAddOpen: false,
    extConfirm: "",
    mcpConfirm: "",
    pluginConfirm: "",

    queue: [],
    attachments: [],
    pendingPerms: {},
    pendingQuestions: {},
    questionDrafts: {},
    traceOpen: new Set(),

    drawerPromptId: "",
    drawerFocusKey: "",
    drawerMode: "status",
    drawerDetailCache: {},
    lastAccount: null,
    lastHost: null,
    lastInfoKind: "info",
    accountEmail: "",
    pageSessionTitle: "",

    workStarted: 0,
    workTimer: 0,
    workWatch: 0,
    renderTimer: 0,
    statusPoll: 0,
    atTimer: 0,
    atSeq: 0,

    runtime: { models: [], commands: [], workspace_roots: [] },
    runtimeReady: false,

    slashIdx: 0,
    atIdx: 0,
    atRows: []
  };

  ctx.setPageTitle = function setPageTitle(sessionTitle) {
    if (sessionTitle !== undefined) ctx.pageSessionTitle = String(sessionTitle || "");
    const email = String(ctx.accountEmail || "").trim();
    const sess = String(ctx.pageSessionTitle || "").trim();
    if (sess && email) document.title = sess + " · " + email;
    else if (sess) document.title = sess + " · GGOK";
    else if (email) document.title = "GGOK · " + email;
    else document.title = "GGOK";
  };

  function applyAccount(acc) {
    if (!acc) return;
    const prev = ctx.lastAccount;
    if (prev && prev.used_percent != null && acc.used_percent == null) {
      acc = Object.assign({}, prev, acc);
    }
    ctx.lastAccount = acc;
    const email = String((acc && acc.email) || "").trim();
    if (email) {
      ctx.accountEmail = email;
      ctx.setPageTitle();
    }
    if (ctx.writeCachedAccount) ctx.writeCachedAccount(acc);
    if (ctx.renderAccount) ctx.renderAccount(acc);
  }
  ctx.applyAccount = applyAccount;

  bindTheme(ctx);
  bindConfirm(ctx);
  bindSidebar(ctx);
  bindQuota(ctx);
  bindVersion(ctx);
  bindModelMenu(ctx);
  bindFinder(ctx);
  bindDirModal(ctx);
  bindDrawer(ctx);
  bindWorkspace(ctx);
  bindExtModal(ctx);
  bindSse(ctx);
  bindTimeline(ctx);
  bindQuestion(ctx);
  bindComposer(ctx);

  document.addEventListener("keydown", (e) => {
    if ((e.metaKey || e.ctrlKey) && !e.shiftKey && !e.altKey && (e.key === "j" || e.key === "J")) {
      e.preventDefault();
      if (ctx.startNewChat) ctx.startNewChat();
      return;
    }
    if ((e.metaKey || e.ctrlKey) && !e.shiftKey && !e.altKey && (e.key === "k" || e.key === "K")) {
      e.preventDefault();
      if (ctx.finderOpen && ctx.finderOpen()) {
        if (ctx.closeFinder) ctx.closeFinder();
      } else {
        if (ctx.openFinder) ctx.openFinder();
      }
      return;
    }
    if (ctx.finderOpen && ctx.finderOpen()) {
      if ((e.metaKey || e.ctrlKey) && e.shiftKey && !e.altKey && (e.key === "p" || e.key === "P")) {
        e.preventDefault();
        if (ctx.toggleFinderPreview) ctx.toggleFinderPreview();
        return;
      }
      if ((e.metaKey || e.ctrlKey) && e.shiftKey && !e.altKey && (e.key === "e" || e.key === "E")) {
        e.preventDefault();
        if (ctx.finderSel && ctx.finderSel !== "__new__" && ctx.startFinderRename) {
          ctx.startFinderRename(ctx.finderSel);
        }
        return;
      }
      if ((e.metaKey || e.ctrlKey) && e.shiftKey && !e.altKey && (e.key === "d" || e.key === "D")) {
        e.preventDefault();
        if (ctx.finderSel && ctx.finderSel !== "__new__" && ctx.askDeleteSession) {
          ctx.askDeleteSession(ctx.finderSel);
        }
        return;
      }
      if (e.key === "ArrowDown") {
        e.preventDefault();
        if (ctx.finderMove) ctx.finderMove(1);
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        if (ctx.finderMove) ctx.finderMove(-1);
        return;
      }
      if (e.key === "Enter" && !ctx.finderComposing && !ctx.finderRenaming) {
        e.preventDefault();
        if (ctx.finderSel && ctx.finderPick) ctx.finderPick(ctx.finderSel);
        return;
      }
    }
    if (ctx.dirModalOpen && ctx.dirModalOpen()) {
      if (e.key === "Enter") {
        e.preventDefault();
        if (ctx.dirPath) {
          if (ctx.closeDirModal) ctx.closeDirModal();
          if (ctx.pickCwd) ctx.pickCwd(ctx.dirPath);
        }
        return;
      }
    }
    if (e.key === "Escape") {
      if (ctx.setInfoOpen) ctx.setInfoOpen(false);
      if (ctx.setQuotaOpen) ctx.setQuotaOpen(false);
      const slashMenu = document.getElementById("slash-menu");
      const atMenu = document.getElementById("at-menu");
      const modelMenu = document.getElementById("model-menu");
      if (slashMenu) slashMenu.hidden = true;
      if (atMenu) atMenu.hidden = true;
      if (modelMenu) modelMenu.hidden = true;
      hideTip(false);
      if (ctx.dirModalOpen && ctx.dirModalOpen()) {
        if (ctx.closeDirModal) ctx.closeDirModal();
        return;
      }
      if (ctx.confirmOpen && ctx.confirmOpen()) {
        if (ctx.closeConfirm) ctx.closeConfirm();
        return;
      }
      const sessMenu = document.getElementById("sess-menu");
      if (sessMenu && !sessMenu.hidden) {
        if (ctx.closeSessMenu) ctx.closeSessMenu();
        return;
      }
      if (ctx.finderOpen && ctx.finderOpen()) {
        if (ctx.closeFinder) ctx.closeFinder();
        return;
      }
      const extSkillMenu = document.getElementById("ext-skill-menu");
      if (extSkillMenu && !extSkillMenu.hidden) {
        if (ctx.hideSkillMenu) ctx.hideSkillMenu();
        return;
      }
      if (ctx.extAddOpen && ctx.extModalOpen && ctx.extModalOpen()) {
        ctx.extAddOpen = false;
        if (ctx.renderExtModal) ctx.renderExtModal();
        return;
      }
      if (ctx.extModalOpen && ctx.extModalOpen()) {
        if (ctx.closeExtModal) ctx.closeExtModal();
        return;
      }
      const drawerEl = document.getElementById("drawer");
      if (drawerEl && !drawerEl.hidden) {
        if (ctx.closeDrawer) ctx.closeDrawer();
      }
      if (ctx.closeMobile) ctx.closeMobile();
    }
  });

  document.addEventListener("i18n-change", () => {
    hideTip(false);
    if (ctx.closeSessMenu) ctx.closeSessMenu();
    relocalizeDyn(document);
    if (ctx.syncThemeButton && ctx.effectiveTheme && ctx.themePref) {
      ctx.syncThemeButton(ctx.effectiveTheme(ctx.themePref()));
    }
    if (ctx.syncDirLabel) ctx.syncDirLabel();
    if (ctx.syncSendBtn) ctx.syncSendBtn();
    if (ctx.applyContext) ctx.applyContext(ctx.current && ctx.current.context);
    if (ctx.fillModels) ctx.fillModels();
    if (ctx.paintPromptPh) ctx.paintPromptPh(false);
    if (ctx.renderSlash) ctx.renderSlash();
    if (ctx.renderChips) ctx.renderChips();
    if (ctx.syncSearchTip) ctx.syncSearchTip();
    if (ctx.syncFinderKeys) ctx.syncFinderKeys();
    if (ctx.syncFinderPreview) ctx.syncFinderPreview();
    if (ctx.finderOpen && ctx.finderOpen() && ctx.renderFinder) ctx.renderFinder();
    if (ctx.extModalOpen && ctx.extModalOpen() && ctx.renderExtModal) ctx.renderExtModal();
    if (ctx.lastAccount && ctx.renderAccount) ctx.renderAccount(ctx.lastAccount);
    if (ctx.syncWsButton) ctx.syncWsButton();
    if (ctx.filesDrawerOpen && ctx.filesDrawerOpen() && ctx.renderWsList) ctx.renderWsList();
    if (ctx.drawerMode === "status") {
      if (ctx.renderUsage) ctx.renderUsage((ctx.current && ctx.current.usage) || {});
      if (ctx.lastHost && ctx.renderHost) ctx.renderHost(ctx.lastHost);
    } else if (ctx.drawerMode === "process") {
      const drawerEl = document.getElementById("drawer");
      if (drawerEl && !drawerEl.hidden && ctx.renderDrawer) ctx.renderDrawer();
    }
    if (ctx.dirModalOpen && ctx.dirModalOpen() && ctx.renderDirModal) ctx.renderDirModal();
    const modelMenu = document.getElementById("model-menu");
    if (modelMenu && !modelMenu.hidden && ctx.renderModelMenu) ctx.renderModelMenu();
    const infoPop = document.getElementById("info-pop");
    if (infoPop && !infoPop.hidden && ctx.setInfoOpen) ctx.setInfoOpen(true, ctx.lastInfoKind);
  });

  const emptyState = document.getElementById("empty-state");
  Promise.all([api("/api/runtime"), ctx.loadList ? ctx.loadList() : Promise.resolve()])
    .then(([rt]) => {
      ctx.runtime = rt || ctx.runtime;
      ctx.runtimeReady = true;
      if (ctx.runtime.email) {
        ctx.accountEmail = String(ctx.runtime.email).trim();
        ctx.setPageTitle();
      }
      if (ctx.fillModels) ctx.fillModels();
      ctx.selectedCwd = "";
      if (ctx.syncDirLabel) ctx.syncDirLabel();
      if (ctx.syncWsButton) ctx.syncWsButton();
      if (ctx.renderSlash) ctx.renderSlash();
      const hash = location.hash.replace(/^#/, "");
      if (hash && ctx.openSession) ctx.openSession(hash);
    })
    .catch((e) => {
      ctx.runtimeReady = true;
      if (ctx.fillModels) ctx.fillModels();
      if (!emptyState) return;
      const p = document.createElement("p");
      p.className = "error";
      p.textContent = formatError(e);
      emptyState.appendChild(p);
    });
}
