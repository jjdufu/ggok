import { t } from "./helpers.js";

export function beginInlineRename(host, { value, inputClass, onCommit, onRestore }) {
  const input = document.createElement("input");
  input.type = "text";
  input.className = inputClass;
  input.value = value || "";
  input.setAttribute("aria-label", t("rename"));
  input.placeholder = t("renamePlaceholder");
  let done = false;
  const finish = async (ok) => {
    if (done) return;
    done = true;
    if (ok && onCommit) await onCommit(input.value);
    if (onRestore) onRestore();
  };
  input.addEventListener("click", (e) => e.stopPropagation());
  input.addEventListener("pointerdown", (e) => e.stopPropagation());
  input.addEventListener("keydown", (e) => {
    e.stopPropagation();
    if (e.key === "Enter") {
      e.preventDefault();
      finish(true);
    } else if (e.key === "Escape") {
      e.preventDefault();
      finish(false);
    }
  });
  input.addEventListener("blur", () => finish(true));
  host.replaceWith(input);
  input.focus();
  input.select();
  return input;
}
