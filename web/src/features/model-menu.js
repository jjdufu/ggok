import { t, isSpectatingSource } from "../lib/helpers.js";
import { placePopover } from "../lib/popover.js";
import { svgUse } from "../lib/svg.js";
import { post } from "../lib/api.js";
import { toast } from "../lib/clipboard.js";

export function bindModelMenu(ctx) {
  const { EFFORT_I18N } = ctx;
  const modelBtn = document.getElementById("model-btn");
  const modelLabel = document.getElementById("model-label");
  const modelMenu = document.getElementById("model-menu");
  let pendingOpen = false;

  function modelById(id) {
    return (ctx.runtime.models || []).find((m) => m.id === id);
  }

  function effortInfo(modelId, effortId) {
    const m = modelById(modelId);
    if (!m) return null;
    return (m.efforts || []).find((e) => e.id === effortId) || null;
  }

  function effortLabel(e) {
    if (!e) return "";
    const keys = EFFORT_I18N[e.id];
    if (keys) {
      const tr = t(keys.label);
      if (tr !== keys.label) return tr;
    }
    return (e.label || "").replace(/\s*Effort$/i, "") || e.id || "";
  }

  function effortDesc(e) {
    if (!e) return "";
    const keys = EFFORT_I18N[e.id];
    if (keys) {
      const tr = t(keys.desc);
      if (tr !== keys.desc) return tr;
    }
    return e.description || "";
  }

  function modelDisplayName(m) {
    if (!m) return "";
    return m.name || m.id || "";
  }

  function modelDescText(m) {
    if (!m) return "";
    const key = "modelDesc." + m.id;
    const tr = t(key);
    if (tr !== key) return tr;
    return m.description || "";
  }

  function matchEffort(m, raw) {
    const q = String(raw || "").trim().toLowerCase();
    if (!q || !m) return "";
    const efforts = m.efforts || [];
    const hit = efforts.find((e) => {
      const id = String(e.id || "").toLowerCase();
      const lab = String(e.label || "").toLowerCase();
      return id === q || lab === q || lab.replace(/\s*effort$/i, "").trim() === q || lab.startsWith(q);
    });
    return hit ? hit.id : "";
  }

  function modelFace() {
    const m = modelById(ctx.selectedModel);
    const name = modelDisplayName(m) || ctx.selectedModel;
    const e = effortInfo(ctx.selectedModel, ctx.selectedEffort);
    const lab = effortLabel(e);
    if (lab) return name + " · " + lab;
    return name || t("model");
  }

  function fillModels() {
    const spectating = isSpectatingSource(ctx.source);
    if (!ctx.selectedModel) ctx.selectedModel = ctx.runtime.current_model || ((ctx.runtime.models || [])[0] && ctx.runtime.models[0].id) || "";
    if (ctx.selectedModel && ctx.selectedEffort) {
      const m = modelById(ctx.selectedModel);
      if (m && (m.efforts || []).length && !(m.efforts || []).some((e) => e.id === ctx.selectedEffort)) {
        if (!spectating) {
          ctx.selectedEffort = m.effort || (m.efforts[0] && m.efforts[0].id) || "";
        }
      }
    }
    if (ctx.selectedModel && !ctx.selectedEffort && !spectating) {
      const m = modelById(ctx.selectedModel);
      if (m) ctx.selectedEffort = m.effort || ((m.efforts || [])[0] && m.efforts[0].id) || "";
    }
    if (modelLabel) modelLabel.textContent = modelFace();
    if (pendingOpen && ((ctx.runtime && ctx.runtime.models) || []).length) {
      pendingOpen = false;
      openModelMenu();
    }
  }

  function openModelMenu() {
    const models = (ctx.runtime && ctx.runtime.models) || [];
    if (!models.length) {
      if (!ctx.runtimeReady) {
        pendingOpen = true;
        return;
      }
      pendingOpen = false;
      toast(t("noModels"));
      return;
    }
    pendingOpen = false;
    ctx.modelExpanded = new Set(ctx.selectedModel ? [ctx.selectedModel] : []);
    renderModelMenu();
  }

  function toggleModelGroup(id) {
    if (ctx.modelExpanded.has(id)) ctx.modelExpanded.delete(id);
    else ctx.modelExpanded.add(id);
    renderModelMenu();
  }

  function renderModelMenu() {
    if (!modelMenu) return;
    const models = (ctx.runtime && ctx.runtime.models) || [];
    if (!models.length) {
      modelMenu.hidden = true;
      if (ctx.runtimeReady) toast(t("noModels"));
      else pendingOpen = true;
      return;
    }
    modelMenu.replaceChildren();
    for (const m of models) {
      const open = ctx.modelExpanded && ctx.modelExpanded.has(m.id);
      const current = ctx.selectedModel === m.id;
      const g = document.createElement("div");
      g.className = "model-group" + (open ? " open" : " collapsed") + (current ? " current" : "");
      const lab = document.createElement("button");
      lab.type = "button";
      lab.className = "model-group-label";
      const dot = document.createElement("span");
      dot.className = "model-dot";
      const title = document.createElement("span");
      title.className = "model-group-title";
      title.textContent = modelDisplayName(m);
      const meta = document.createElement("span");
      meta.className = "model-group-meta";
      const chev = svgUse("i-chevron");
      chev.classList.add("model-group-chev");
      meta.appendChild(chev);
      lab.append(dot, title, meta);
      lab.addEventListener("click", (ev) => {
        ev.stopPropagation();
        toggleModelGroup(m.id);
      });
      g.appendChild(lab);
      if (open) {
        const body = document.createElement("div");
        body.className = "model-group-body";
        const efforts = m.efforts || [];
        const rows = efforts.length ? efforts : [{ id: "", label: modelDisplayName(m), description: modelDescText(m) }];
        for (const e of rows) {
          const b = document.createElement("button");
          b.type = "button";
          b.className = "model-item" + (ctx.selectedModel === m.id && (ctx.selectedEffort || "") === (e.id || "") ? " on" : "");
          const text = document.createElement("span");
          text.className = "model-item-text";
          const name = document.createElement("span");
          name.className = "model-item-name";
          name.textContent = effortLabel(e) || e.id || modelDisplayName(m);
          text.appendChild(name);
          const desc = effortDesc(e) || (efforts.length ? "" : modelDescText(m));
          if (desc) {
            const d = document.createElement("span");
            d.className = "model-item-desc";
            d.textContent = desc;
            text.appendChild(d);
          }
          const dotEl = document.createElement("span");
          dotEl.className = "model-dot";
          b.append(dotEl, text);
          b.addEventListener("click", (ev) => {
            ev.stopPropagation();
            pickModel(m.id, e.id || "");
          });
          body.appendChild(b);
        }
        g.appendChild(body);
      }
      modelMenu.appendChild(g);
    }
    modelMenu.hidden = false;
    pinModelMenu();
  }

  function pinModelMenu() {
    if (!modelBtn || !modelMenu || modelMenu.hidden) return;
    placePopover(modelMenu, modelBtn, {
      gap: 8,
      pad: 12,
      minH: 120,
      width: 280,
      align: "right",
      zIndex: 40
    });
  }

  async function pickModel(model, effort) {
    ctx.selectedModel = model;
    ctx.selectedEffort = effort || "";
    fillModels();
    if (modelMenu) modelMenu.hidden = true;
    if (isSpectatingSource(ctx.source) || ctx.writable === false) return;
    if (ctx.persistLastModel) ctx.persistLastModel(model, effort || "");
    if (!ctx.currentId) return;
    try {
      const out = await post("/api/sessions/" + encodeURIComponent(ctx.currentId) + "/model", {
        model,
        effort: effort || undefined
      });
      if (out && out.effort) ctx.selectedEffort = out.effort;
      fillModels();
    } catch (e) {
      toast(String(e.message || e));
    }
  }

  if (modelBtn) {
    modelBtn.addEventListener("click", (e) => {
      e.stopPropagation();
      if (modelMenu && !modelMenu.hidden) {
        modelMenu.hidden = true;
      } else {
        openModelMenu();
      }
    });
  }

  document.addEventListener("click", (e) => {
    if (modelMenu && !modelMenu.hidden && !modelMenu.contains(e.target) && (!modelBtn || !modelBtn.contains(e.target))) {
      modelMenu.hidden = true;
    }
  });

  window.addEventListener("resize", pinModelMenu);
  window.addEventListener("scroll", pinModelMenu, true);

  ctx.modelById = modelById;
  ctx.effortInfo = effortInfo;
  ctx.effortLabel = effortLabel;
  ctx.effortDesc = effortDesc;
  ctx.modelDisplayName = modelDisplayName;
  ctx.modelDescText = modelDescText;
  ctx.matchEffort = matchEffort;
  ctx.modelFace = modelFace;
  ctx.fillModels = fillModels;
  ctx.openModelMenu = openModelMenu;
  ctx.toggleModelGroup = toggleModelGroup;
  ctx.renderModelMenu = renderModelMenu;
  ctx.pinModelMenu = pinModelMenu;
  ctx.pickModel = pickModel;
}
