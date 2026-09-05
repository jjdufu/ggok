import { t, formatError } from "./helpers.js";

export function toast(msg) {
  const n = document.createElement("div");
  n.className = "toast";
  n.textContent = formatError(msg);
  document.body.appendChild(n);
  setTimeout(() => n.remove(), 2800);
}

export function copyFallback(text) {
  const ta = document.createElement("textarea");
  ta.value = text;
  ta.setAttribute("readonly", "");
  ta.style.cssText = "position:fixed;left:-9999px;top:0";
  document.body.appendChild(ta);
  ta.focus();
  ta.select();
  let ok = false;
  try {
    ok = document.execCommand("copy");
  } catch (err) {
    ok = false;
  }
  ta.remove();
  return ok;
}

export function copyText(text) {
  const done = (ok) => toast(ok ? t("copied") : t("copyFailed"));
  if (navigator.clipboard && window.isSecureContext) {
    navigator.clipboard.writeText(text).then(
      () => done(true),
      () => done(copyFallback(text))
    );
    return;
  }
  done(copyFallback(text));
}

export function bindCodeCopy(root) {
  root.querySelectorAll(".copy-code").forEach((btn) => {
    btn.addEventListener("click", (e) => {
      e.preventDefault();
      e.stopPropagation();
      const card = btn.closest(".code-card");
      const code = card && card.querySelector("code");
      if (code) copyText(code.textContent);
    });
  });
}
