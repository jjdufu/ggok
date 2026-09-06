import { t, setTip, fmtBytes, formatError, parentOf, filenameFromDisposition } from "../lib/helpers.js";
import { emptyEl, iconAct } from "../lib/dom.js";
import { svgUse } from "../lib/svg.js";
import { api, del } from "../lib/api.js";
import { toast } from "../lib/clipboard.js";

export function bindWorkspace(ctx) {
  const wsToggle = document.getElementById("ws-toggle");
  const wsUp = document.getElementById("ws-up");
  const wsRefresh = document.getElementById("ws-refresh");
  const wsPack = document.getElementById("ws-pack");
  const wsPath = document.getElementById("ws-path");
  const wsList = document.getElementById("ws-list");

  ctx.wsRel = "";
  ctx.wsRows = [];
  ctx.wsSel = "";
  ctx.wsSeq = 0;

  function filesDrawerOpen() {
    return ctx.filesDrawerOpen ? ctx.filesDrawerOpen() : false;
  }

  function syncWsButton() {
    if (!wsToggle) return;
    const tip = ctx.selectedCwd || t("workspace");
    wsToggle.setAttribute("aria-label", tip);
  }

  function wsAct(icon, tip, cls, onClick) {
    return iconAct({
      icon,
      className: "ws-act" + (cls ? " " + cls : ""),
      tip,
      clearTip: true,
      onClick
    });
  }

  function goUp() {
    if (!ctx.wsRel) return;
    ctx.wsRel = parentOf(ctx.wsRel);
    ctx.wsSel = "";
    loadWsList();
  }

  function appendWsUpRow() {
    const row = document.createElement("div");
    row.className = "ws-item dir ws-item-up";
    const main = document.createElement("button");
    main.type = "button";
    main.className = "ws-main";
    const ico = svgUse("i-folder");
    ico.classList.add("ws-ico");
    main.appendChild(ico);
    const name = document.createElement("span");
    name.className = "ws-name";
    name.textContent = "..";
    main.appendChild(name);
    setTip(main, t("wsUp"));
    main.setAttribute("aria-label", t("wsUp"));
    main.addEventListener("click", goUp);
    const tail = document.createElement("span");
    tail.className = "ws-tail";
    row.append(main, tail);
    wsList.appendChild(row);
  }

  function renderWsList() {
    const atRoot = !ctx.wsRel;
    if (wsPath) {
      wsPath.hidden = atRoot;
      wsPath.textContent = ctx.wsRel;
      setTip(wsPath, atRoot ? "" : ctx.wsRel);
    }
    if (wsUp) {
      wsUp.hidden = atRoot;
      wsUp.disabled = atRoot;
    }
    if (wsPack) setTip(wsPack, t("wsSkipHint"));
    if (!wsList) return;
    wsList.replaceChildren();
    if (!ctx.selectedCwd) {
      wsList.appendChild(emptyEl("ws-empty", t("pickCwdFirst")));
      return;
    }
    if (!atRoot) appendWsUpRow();
    if (!ctx.wsRows.length) {
      if (atRoot) wsList.appendChild(emptyEl("ws-empty", t("wsEmpty")));
      return;
    }
    for (const entry of ctx.wsRows) {
      const row = document.createElement("div");
      row.className = "ws-item" + (entry.dir ? " dir" : "") + (ctx.wsSel === entry.path ? " on" : "");
      const main = document.createElement("button");
      main.type = "button";
      main.className = "ws-main";
      if (entry.dir) {
        const ico = svgUse("i-folder");
        ico.classList.add("ws-ico");
        main.appendChild(ico);
      }
      const name = document.createElement("span");
      name.className = "ws-name";
      name.textContent = entry.name;
      main.appendChild(name);
      setTip(main, entry.path);
      main.addEventListener("click", () => {
        if (entry.dir) {
          ctx.wsRel = entry.path;
          ctx.wsSel = "";
          loadWsList();
        } else {
          ctx.wsSel = entry.path;
          renderWsList();
        }
      });
      const tail = document.createElement("span");
      tail.className = "ws-tail";
      if (!entry.dir) {
        const sz = document.createElement("span");
        sz.className = "ws-size";
        sz.textContent = fmtBytes(entry.size);
        tail.appendChild(sz);
      }
      const acts = document.createElement("span");
      acts.className = "ws-acts";
      acts.appendChild(
        wsAct("i-at", t("wsAtRef"), "", () => {
          if (ctx.insertAtRef) ctx.insertAtRef(entry);
        })
      );
      acts.appendChild(wsAct("i-download", t("wsDownload"), "", () => downloadWs(entry)));
      acts.appendChild(wsAct("i-trash", t("delete"), "danger", () => askDelete(entry)));
      tail.appendChild(acts);
      row.append(main, tail);
      wsList.appendChild(row);
    }
  }

  async function loadWsList() {
    const cwd = ctx.selectedCwd;
    if (!cwd) {
      ctx.wsRows = [];
      renderWsList();
      return;
    }
    const seq = ++ctx.wsSeq;
    const path = ctx.wsRel || "";
    try {
      const data = await api(
        "/api/workspace?cwd=" + encodeURIComponent(cwd) + "&path=" + encodeURIComponent(path)
      );
      if (seq !== ctx.wsSeq) return;
      ctx.wsRel = data.dir || "";
      ctx.wsRows = Array.isArray(data.entries) ? data.entries : [];
      if (data.truncated) toast(t("wsTruncated"));
      renderWsList();
    } catch (e) {
      if (seq !== ctx.wsSeq) return;
      toast(formatError(e));
    }
  }

  function setFilesOpen(on) {
    if (!on) {
      if (filesDrawerOpen()) ctx.closeDrawer();
      return;
    }
    if (!ctx.selectedCwd) {
      if (ctx.renderDirModal) ctx.renderDirModal();
      toast(t("pickCwdFirst"));
      return;
    }
    const infoPop = document.getElementById("info-pop");
    if (infoPop) infoPop.hidden = true;
    if (ctx.setQuotaOpen) ctx.setQuotaOpen(false);
    ctx.setDrawerMode("files");
    ctx.showDrawer();
    loadWsList();
  }

  async function downloadWs(row) {
    const cwd = ctx.selectedCwd;
    if (!cwd) {
      toast(t("pickCwdFirst"));
      return;
    }
    const path = row ? row.path : "";
    const url =
      row && !row.dir
        ? "/api/workspace/file?cwd=" + encodeURIComponent(cwd) + "&path=" + encodeURIComponent(path)
        : "/api/workspace/archive?cwd=" +
          encodeURIComponent(cwd) +
          "&path=" +
          encodeURIComponent(path || "");
    let res;
    try {
      res = await fetch(url, { credentials: "same-origin" });
    } catch (e) {
      toast(formatError(e));
      return;
    }
    if (res.status === 401) {
      location.href = "/login";
      return;
    }
    if (!res.ok) {
      toast(formatError(await res.text()));
      return;
    }
    const blob = await res.blob();
    const name =
      filenameFromDisposition(res.headers.get("content-disposition")) ||
      (row && row.dir ? row.name + ".zip" : row ? row.name : "workspace.zip");
    const a = document.createElement("a");
    const href = URL.createObjectURL(blob);
    a.href = href;
    a.download = name;
    a.click();
    URL.revokeObjectURL(href);
  }

  async function packCwd() {
    if (!wsPack || wsPack.disabled) return;
    wsPack.disabled = true;
    wsPack.textContent = t("wsPacking");
    setTip(wsPack, t("wsPacking"));
    try {
      await downloadWs(null);
    } finally {
      wsPack.disabled = false;
      wsPack.textContent = t("wsPack");
      setTip(wsPack, t("wsSkipHint"));
    }
  }

  function askDelete(row) {
    if (!ctx.openConfirm || !row) return;
    const abs = String(ctx.selectedCwd || "").replace(/\/+$/, "") + "/" + row.path;
    ctx.openConfirm({
      title: t("wsDeleteTitle", { name: row.name }),
      body: abs + "\n" + t("wsDeleteBody"),
      ok: t("confirmDelete"),
      danger: true,
      onOk: () => {
        del(
          "/api/workspace?cwd=" +
            encodeURIComponent(ctx.selectedCwd) +
            "&path=" +
            encodeURIComponent(row.path)
        )
          .then(() => {
            if (row.dir && (ctx.wsRel === row.path || String(ctx.wsRel || "").startsWith(row.path + "/"))) {
              ctx.wsRel = parentOf(row.path);
            }
            if (ctx.wsSel === row.path) ctx.wsSel = "";
            loadWsList();
          })
          .catch((e) => toast(formatError(e)));
      }
    });
  }

  if (wsToggle) {
    wsToggle.addEventListener("click", (e) => {
      e.stopPropagation();
      if (ctx.setQuotaOpen) ctx.setQuotaOpen(false);
      setFilesOpen(!filesDrawerOpen());
    });
  }

  if (wsUp) {
    wsUp.addEventListener("click", goUp);
  }

  if (wsRefresh) {
    wsRefresh.addEventListener("click", () => loadWsList());
  }

  if (wsPack) {
    wsPack.addEventListener("click", () => packCwd());
  }

  ctx.onCwdPicked = function onCwdPicked() {
    syncWsButton();
    if (filesDrawerOpen()) {
      ctx.wsRel = "";
      ctx.wsSel = "";
      loadWsList();
    }
  };

  ctx.syncWsButton = syncWsButton;
  ctx.setFilesOpen = setFilesOpen;
  ctx.loadWsList = loadWsList;
  ctx.renderWsList = renderWsList;
  syncWsButton();
}
