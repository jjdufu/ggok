import { t, setTip, parentOf } from "../lib/helpers.js";
import { svgUse } from "../lib/svg.js";
import { api } from "../lib/api.js";
import { toast } from "../lib/clipboard.js";
import { openOverlay, closeOverlay } from "../lib/overlay.js";

export function bindDirModal(ctx) {
  const dirBtn = document.getElementById("dir-btn");
  const dirLabel = document.getElementById("dir-label");
  const dirPathText = dirLabel && dirLabel.querySelector(".dir-path-text");
  const dirIconUse = dirBtn && dirBtn.querySelector("use");
  const dirScrim = document.getElementById("dir-scrim");
  const dirModal = document.getElementById("dir-modal");
  const dirModalPath = document.getElementById("dir-modal-path");
  const dirModalList = document.getElementById("dir-modal-list");
  const dirModalChoose = document.getElementById("dir-modal-choose");
  const dirModalClose = document.getElementById("dir-modal-close");

  let dirRenderSeq = 0;
  let dirSel = "";

  function syncDirLabel() {
    const pending = !ctx.selectedCwd;
    if (dirBtn) dirBtn.classList.toggle("pending", pending);
    if (dirIconUse) dirIconUse.setAttribute("href", "#i-folder");
    if (dirPathText) dirPathText.textContent = ctx.selectedCwd || "";
    if (dirLabel) dirLabel.hidden = pending;
    const dirTip = ctx.selectedCwd || t("pickCwd");
    if (dirBtn) {
      setTip(dirBtn, dirTip);
      dirBtn.setAttribute("aria-label", dirTip);
    }
  }

  function dirModalOpen() {
    return dirModal && !dirModal.hidden;
  }

  function closeDirModal() {
    dirRenderSeq += 1;
    closeOverlay(dirScrim, { panel: dirModal });
  }

  function dirNorm(p) {
    return String(p || "").replace(/\/+$/, "");
  }

  function dirBase(p) {
    const n = dirNorm(p);
    if (!n || n === "/") return "/";
    const i = n.lastIndexOf("/");
    return i < 0 ? n : n.slice(i + 1) || "/";
  }

  function dirUnderRoots(path) {
    const roots = (ctx.runtime && ctx.runtime.workspace_roots) || [];
    if (!roots.length) return true;
    const n = dirNorm(path);
    if (!n) return false;
    return roots.some((r) => {
      const root = dirNorm(r);
      return !!root && (n === root || n.startsWith(root + "/"));
    });
  }

  function dirUpPath(path) {
    const parentPath = parentOf(dirNorm(path));
    const roots = (ctx.runtime && ctx.runtime.workspace_roots) || [];
    if (!roots.length) return parentPath;
    if (!parentPath || !dirUnderRoots(parentPath)) return "";
    return parentPath;
  }

  function chooseTarget() {
    return (ctx.dirPath || dirSel || "").trim();
  }

  function syncChoose() {
    if (!dirModalChoose) return;
    dirModalChoose.disabled = !chooseTarget();
    dirModalChoose.textContent = t("chooseThisDir");
  }

  function appendDirItem({ path, name, sub, up, git, onClick }) {
    const b = document.createElement("button");
    b.type = "button";
    b.className = "dir-item" + (up ? " dir-item-up" : "") + (!up && dirSel && dirSel === path ? " on" : "");
    if (path) b.dataset.path = path;
    const ico = svgUse("i-folder");
    ico.classList.add("dir-ico");
    b.appendChild(ico);
    const body = document.createElement("span");
    body.className = "dir-item-body";
    const title = document.createElement("span");
    title.className = "dir-item-name";
    title.textContent = name;
    body.appendChild(title);
    if (sub) {
      const s = document.createElement("span");
      s.className = "dir-item-sub";
      s.textContent = sub;
      body.appendChild(s);
    }
    b.appendChild(body);
    if (git) {
      const g = document.createElement("span");
      g.className = "git";
      g.appendChild(svgUse("i-git"));
      b.appendChild(g);
    }
    setTip(b, up ? t("wsUp") : path || name);
    if (up) b.setAttribute("aria-label", t("wsUp"));
    b.addEventListener("click", onClick);
    dirModalList.appendChild(b);
  }

  async function renderDirModal() {
    if (!dirModal || !dirModalList) return;
    const seq = ++dirRenderSeq;
    if (ctx.closeFinder) ctx.closeFinder();
    if (ctx.dirPath && !dirUnderRoots(ctx.dirPath)) ctx.dirPath = "";
    const path = ctx.dirPath || "";
    const atStart = !path;
    if (dirModalPath) {
      dirModalPath.textContent = path || t("allowedRoots");
      setTip(dirModalPath, path || "");
    }
    openOverlay(dirScrim, dirModal);
    dirModal.focus();
    let rows = [];
    try {
      const qs = path ? "?parent=" + encodeURIComponent(path) : "";
      const data = await api("/api/dirs" + qs);
      if (seq !== dirRenderSeq) return;
      rows = Array.isArray(data) ? data : [];
    } catch (e) {
      if (seq !== dirRenderSeq) return;
      const msg = String(e.message || e);
      if (path && msg.includes("outside workspace_roots")) {
        ctx.dirPath = "";
        dirSel = "";
        await renderDirModal();
        return;
      }
      toast(msg);
    }
    if (seq !== dirRenderSeq) return;
    if (atStart) {
      const paths = rows.map((r) => r.path).filter(Boolean);
      if (!paths.includes(dirSel)) dirSel = paths[0] || "";
    } else {
      dirSel = "";
    }
    syncChoose();
    dirModalList.replaceChildren();
    if (!atStart) {
      appendDirItem({
        name: "..",
        up: true,
        onClick: async (e) => {
          e.stopPropagation();
          ctx.dirPath = dirUpPath(path);
          dirSel = "";
          await renderDirModal();
        }
      });
    }
    for (const row of rows) {
      const full = row.path || "";
      appendDirItem({
        path: full,
        name: atStart ? dirBase(full || row.name) : row.name,
        sub: atStart ? full : "",
        git: !!row.git,
        onClick: async (e) => {
          e.stopPropagation();
          if (atStart) {
            if (dirSel === full) {
              ctx.dirPath = full;
              dirSel = "";
              await renderDirModal();
              return;
            }
            dirSel = full;
            dirModalList.querySelectorAll(".dir-item").forEach((el) => {
              el.classList.toggle("on", el.dataset.path === dirSel);
            });
            syncChoose();
            return;
          }
          ctx.dirPath = full;
          await renderDirModal();
        }
      });
    }
    dirModalList.scrollTop = 0;
  }

  async function pickCwd(path) {
    if (ctx.onCwdPicked) ctx.onCwdPicked(path);
    ctx.selectedCwd = path;
    syncDirLabel();
  }

  if (dirBtn) {
    dirBtn.addEventListener("click", () => {
      ctx.dirPath = "";
      dirSel = "";
      renderDirModal();
    });
  }

  if (dirScrim) {
    dirScrim.addEventListener("click", closeDirModal);
  }

  if (dirModalClose) {
    dirModalClose.addEventListener("click", closeDirModal);
  }

  if (dirModalChoose) {
    dirModalChoose.addEventListener("click", () => {
      const pick = chooseTarget();
      if (!pick) return;
      closeDirModal();
      pickCwd(pick);
    });
  }

  ctx.syncDirLabel = syncDirLabel;
  ctx.dirModalOpen = dirModalOpen;
  ctx.closeDirModal = closeDirModal;
  ctx.renderDirModal = renderDirModal;
  ctx.pickCwd = pickCwd;
}
