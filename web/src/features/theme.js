import { t } from "../lib/helpers.js";

export function bindTheme(ctx) {
  const { THEME_KEY } = ctx;

  function themePref() {
    const p = localStorage.getItem(THEME_KEY);
    return p === "light" || p === "dark" ? p : "system";
  }

  function effectiveTheme(p) {
    if (p === "light" || p === "dark") return p;
    return matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }

  function syncThemeButton(theme) {
    const themeBtn = document.getElementById("theme-btn");
    if (!themeBtn) return;
    const label = t(theme === "dark" ? "themeDark" : "themeLight");
    themeBtn.setAttribute("aria-label", label);
  }

  function applyTheme(p) {
    const theme = effectiveTheme(p);
    document.documentElement.dataset.theme = theme;
    document.documentElement.style.colorScheme = theme;
    syncThemeButton(theme);
  }

  applyTheme(themePref());

  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
    if (themePref() === "system") applyTheme("system");
  });

  const themeBtn = document.getElementById("theme-btn");
  if (themeBtn) {
    themeBtn.addEventListener("click", () => {
      const next = effectiveTheme(themePref()) === "light" ? "dark" : "light";
      localStorage.setItem(THEME_KEY, next);
      applyTheme(next);
      themeBtn.querySelectorAll(".theme-icon").forEach((el) => {
        el.classList.remove("t-icon-swap");
        void el.offsetWidth;
        el.classList.add("t-icon-swap");
      });
    });
  }

  ctx.themePref = themePref;
  ctx.effectiveTheme = effectiveTheme;
  ctx.syncThemeButton = syncThemeButton;
  ctx.applyTheme = applyTheme;
}
