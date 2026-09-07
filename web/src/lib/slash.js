export const SLASH_LOCAL = new Set([
  "model",
  "m",
  "effort",
  "new",
  "clear",
  "usage",
  "cost",
  "session-info",
  "status",
  "info",
  "context",
  "mcps",
  "mcp",
  "plugins",
  "plugin",
  "marketplace",
  "skills",
  "skill",
  "resume",
  "dashboard",
  "agents-dashboard",
  "hooks",
  "workflows",
  "agents",
  "personas"
]);

export const SLASH_RPC = new Set([
  "rewind",
  "undo",
  "fork",
  "compact",
  "plan",
  "always-approve",
  "auto",
  "view-plan",
  "show-plan",
  "plan-view",
  "export",
  "btw"
]);

export const SLASH_SHELL = new Set([
  "imagine",
  "imagine-video",
  "loop",
  "goal",
  "deep-research",
  "workflow",
  "remember",
  "flush",
  "dream",
  "rename",
  "delete"
]);

export const SLASH_TUI = new Set([
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
  "edit-prompt",
  "history"
]);

export function slashKind(name) {
  const n = String(name || "")
    .trim()
    .replace(/^\//, "")
    .toLowerCase();
  if (!n) return "";
  if (SLASH_LOCAL.has(n)) return "local";
  if (SLASH_RPC.has(n)) return "rpc";
  if (SLASH_SHELL.has(n)) return "shell";
  if (SLASH_TUI.has(n)) return "tui";
  return "";
}
