import { t } from "../lib/helpers.js";
import { mkEl } from "../lib/dom.js";
import { svgUse } from "../lib/svg.js";
import { api, post } from "../lib/api.js";
import { toast, bindCodeCopy } from "../lib/clipboard.js";
import { renderMarkdown, codeCardHtml } from "../lib/markdown.js";
import { openOverlay, closeOverlay } from "../lib/overlay.js";

export function bindExtModal(ctx) {
  const extScrim = document.getElementById("ext-scrim");
  const extModal = document.getElementById("ext-modal");
  const extTabs = document.getElementById("ext-tabs");
  const extSearch = document.getElementById("ext-search");
  const extCta = document.getElementById("ext-cta");
  const extSkillMenu = document.getElementById("ext-skill-menu");
  const extSkillFile = document.getElementById("ext-skill-file");
  const extGrid = document.getElementById("ext-grid");
  const extAdd = document.getElementById("ext-add");
  const extDetail = document.getElementById("ext-detail");
  const extBtn = document.getElementById("ext-btn");

  let extComposing = false;

  function mcpCwd() {
    return ctx.selectedCwd || (ctx.current && ctx.current.cwd) || "";
  }

  function mcpServers() {
    const rows = (ctx.mcpData && ctx.mcpData.servers) || [];
    const q = String(ctx.extQuery || "").trim().toLowerCase();
    if (!q) return rows;
    return rows.filter((row) => {
      const name = String(row.name || "").toLowerCase();
      const blob = JSON.stringify(row).toLowerCase();
      return name.includes(q) || blob.includes(q);
    });
  }

  function mcpPluginName(row) {
    const src = String((row && (row.source || (row.doctor && row.doctor.source))) || "");
    const m = src.match(/^plugin:\s*(.+)$/i);
    return m ? String(m[1] || "").trim() : "";
  }

  function mcpMeta(row) {
    const bits = [];
    const plug = mcpPluginName(row);
    if (plug) bits.push(t("mcpFromPlugin").replace("{name}", plug));
    else if (row.scope) bits.push(row.scope);
    if (row.enabled === false || row.disabled) bits.push(t("mcpDisabled"));
    const doc = row.doctor || {};
    const st = doc.status || doc.state || doc.health;
    if (st) bits.push(String(st));
    return bits.join(" · ");
  }

  function mcpTools(row) {
    const pools = [
      row && row.tools,
      row && row.doctor && row.doctor.tools,
      row && row.doctor && row.doctor.available_tools,
      row && row.doctor && row.doctor.toolList
    ];
    const out = [];
    const seen = new Set();
    for (const pool of pools) {
      if (!Array.isArray(pool)) continue;
      for (const item of pool) {
        const name = typeof item === "string" ? item : item && (item.name || item.tool || item.id);
        if (!name || seen.has(String(name))) continue;
        seen.add(String(name));
        out.push({
          name: String(name),
          description: (item && typeof item === "object" && (item.description || item.desc)) || ""
        });
      }
    }
    return out;
  }

  function mcpHealthy(row) {
    const doc = row.doctor || {};
    const st = String(doc.status || doc.state || doc.health || "").toLowerCase();
    if (["ok", "ready", "healthy", "connected", "found"].some((x) => st.includes(x))) return true;
    if (["fail", "error", "unhealthy", "disconnected"].some((x) => st.includes(x))) return false;
    if (row.enabled === false || row.disabled) return null;
    return doc.ok === true ? true : doc.ok === false ? false : null;
  }

  async function loadMcps() {
    ctx.mcpBusy = true;
    renderExtModal();
    try {
      const cwd = mcpCwd();
      const qs = cwd ? "?cwd=" + encodeURIComponent(cwd) : "";
      ctx.mcpData = await api("/api/mcp" + qs);
      if (!ctx.mcpData || !Array.isArray(ctx.mcpData.servers)) ctx.mcpData = { servers: [], sources: [] };
    } catch (e) {
      toast(String(e.message || e));
      if (!ctx.mcpData) ctx.mcpData = { servers: [], sources: [] };
    }
    ctx.mcpBusy = false;
    renderExtModal();
  }

  async function mcpOp(op, extra) {
    ctx.mcpBusy = true;
    renderExtModal();
    try {
      await post("/api/mcp", Object.assign({ op, cwd: mcpCwd() }, extra || {}));
      await loadMcps();
    } catch (e) {
      ctx.mcpBusy = false;
      renderExtModal();
      toast(String(e.message || e));
    }
  }

  function pluginCwd() {
    return ctx.selectedCwd || (ctx.current && ctx.current.cwd) || "";
  }

  function pluginIsInstalled(row) {
    const st = String((row && row.status) || "").toLowerCase();
    return st && st !== "available";
  }

  function pluginRows(tab) {
    const market = (tab || ctx.extTab) === "marketplace";
    const rows = ((ctx.pluginData && ctx.pluginData.plugins) || []).filter((row) =>
      market ? !pluginIsInstalled(row) : pluginIsInstalled(row)
    );
    const q = String(ctx.extQuery || "").trim().toLowerCase();
    if (!q) return rows;
    return rows.filter((row) => {
      const name = String(row.name || row.id || "").toLowerCase();
      const desc = String(row.description || "").toLowerCase();
      const blob = JSON.stringify(row).toLowerCase();
      return name.includes(q) || desc.includes(q) || blob.includes(q);
    });
  }

  function pluginConnectorNames(row) {
    const servers = (row && row.components && row.components.mcpServers) || [];
    return servers.map((s) => String((s && (s.name || s.id)) || "").trim()).filter(Boolean);
  }

  function pluginMeta(row) {
    const bits = [];
    if (row.marketplace) bits.push(row.marketplace);
    if (row.version) bits.push(row.version);
    if (pluginIsInstalled(row)) bits.push(t("pluginInstalled"));
    const st = String(row.status || "").toLowerCase();
    if (st === "disabled") bits.push(t("mcpDisabled"));
    const skills = (row.components && row.components.skills) || [];
    const n = row.skill_count || skills.length;
    if (n) bits.push(t("pluginSkills").replace("{n}", String(n)));
    const cons = pluginConnectorNames(row);
    if (cons.length) bits.push(t("pluginProvidesMcp").replace("{name}", cons.join(", ")));
    else if (row.has_mcp) bits.push("MCP");
    return bits.join(" · ");
  }

  function skillKind(skill) {
    const k = String((skill && skill.kind) || "").toLowerCase();
    if (k === "auto" || k === "guide" || k === "slash") return k;
    return "slash";
  }

  function skillKindLabel(kind) {
    if (kind === "auto") return t("skillKindAuto");
    if (kind === "guide") return t("skillKindGuide");
    return t("skillKindSlash");
  }

  function skillI18nKey(skill) {
    return "skillQ_" + String((skill && (skill.name || skill.id)) || "").trim().replace(/-/g, "_");
  }

  function skillI18n(skill, kind) {
    const base = skillI18nKey(skill);
    const key = kind === "desc" ? base + "Desc" : base;
    const s = t(key);
    if (s && s !== key) return s;
    if (kind === "desc") return String((skill && skill.description) || "");
    return String((skill && (skill.label || skill.name || skill.id)) || "");
  }

  function skillRows(kind) {
    const wantQuick = kind === "quick";
    const out = [];
    const seen = new Set();
    (ctx.personalSkills || []).forEach((row) => {
      const name = String((row && (row.name || row.id)) || "").trim();
      if (!name || seen.has(name)) return;
      const bundled = String((row && row.scope) || "") === "bundled";
      if (wantQuick ? !bundled : bundled) return;
      seen.add(name);
      out.push(row);
    });
    const q = String(ctx.extQuery || "").trim().toLowerCase();
    if (!q) return out;
    return out.filter((skill) => {
      const name = String((skill && (skill.name || skill.id)) || "").toLowerCase();
      const label = skillI18n(skill).toLowerCase();
      const desc = skillI18n(skill, "desc").toLowerCase();
      return name.includes(q) || label.includes(q) || desc.includes(q);
    });
  }

  async function loadSkills() {
    try {
      const cwd = pluginCwd();
      const qs = cwd ? "?cwd=" + encodeURIComponent(cwd) : "";
      const data = await api("/api/skills" + qs);
      ctx.personalSkills = data && Array.isArray(data.skills) ? data.skills : [];
      ctx.skillDetailCache = {};
    } catch (e) {
      ctx.personalSkills = [];
    }
    if (extModalOpen()) renderExtModal();
    if (ctx.renderSlash) ctx.renderSlash();
  }

  async function loadSkillDetail(skill) {
    const name = String((skill && (skill.name || skill.id)) || "").trim();
    const scope = String((skill && skill.scope) || "");
    const key = scope + "::" + name;
    if (ctx.skillDetailCache && ctx.skillDetailCache[key]) return ctx.skillDetailCache[key];
    const qs = new URLSearchParams({ name });
    if (scope) qs.set("scope", scope);
    const cwd = pluginCwd();
    if (cwd) qs.set("cwd", cwd);
    const data = await api("/api/skills/item?" + qs.toString());
    if (!ctx.skillDetailCache) ctx.skillDetailCache = {};
    ctx.skillDetailCache[key] = data;
    return data;
  }

  async function loadPlugins() {
    ctx.pluginBusy = true;
    renderExtModal();
    try {
      const cwd = pluginCwd();
      const qs = cwd ? "?cwd=" + encodeURIComponent(cwd) : "";
      ctx.pluginData = await api("/api/plugins" + qs);
      if (!ctx.pluginData || !Array.isArray(ctx.pluginData.plugins)) {
        ctx.pluginData = { plugins: [], sources: [] };
      }
    } catch (e) {
      toast(String(e.message || e));
      if (!ctx.pluginData) ctx.pluginData = { plugins: [], sources: [] };
    }
    ctx.pluginBusy = false;
    renderExtModal();
  }

  function pluginBusyOn(name, op) {
    if (!ctx.pluginBusy) return false;
    if (op && ctx.pluginBusyOp && ctx.pluginBusyOp !== op) return false;
    const target = String(ctx.pluginBusyName || "");
    if (name && target && target !== String(name)) return false;
    return !!(name && target && target === String(name));
  }

  function fillBtnContent(btn, label, busy) {
    btn.replaceChildren();
    btn.classList.toggle("is-busy", !!busy);
    if (busy) {
      const spin = mkEl("span", "ext-spin");
      spin.setAttribute("aria-hidden", "true");
      btn.appendChild(spin);
    }
    const lab = mkEl("span");
    lab.textContent = label;
    btn.appendChild(lab);
  }

  async function pluginOp(op, extra) {
    if (ctx.closeConfirm) ctx.closeConfirm();
    ctx.pluginBusy = true;
    ctx.pluginBusyOp = op;
    ctx.pluginBusyName = String((extra && (extra.name || extra.source)) || "");
    renderExtModal();
    try {
      await post("/api/plugins", Object.assign({ op, cwd: pluginCwd() }, extra || {}));
      await loadPlugins();
      if (op === "install") {
        const name = ctx.pluginBusyName;
        toast(t("pluginInstallOk").replace("{name}", name || t("pluginInstalled")));
        if (ctx.extOpen && name && ctx.extOpen === name) ctx.extTab = "plugins";
      }
    } catch (e) {
      toast(String(e.message || e));
    }
    ctx.pluginBusy = false;
    ctx.pluginBusyOp = "";
    ctx.pluginBusyName = "";
    renderExtModal();
  }

  function extModalOpen() {
    return extModal && !extModal.hidden;
  }

  function extInitial(name) {
    const s = String(name || "").trim();
    if (!s) return "·";
    const parts = s.split(/\s+/).filter(Boolean);
    const letters = parts.length >= 2
      ? [Array.from(parts[0])[0], Array.from(parts[1])[0]]
      : Array.from(s).slice(0, 2);
    return letters.map((ch) => (/[A-Za-z]/.test(ch) ? ch.toUpperCase() : ch)).join("");
  }

  function openExtModal(tab) {
    if (tab) ctx.extTab = tab;
    ctx.extConfirm = "";
    ctx.extOpen = "";
    ctx.extAddOpen = false;
    ctx.mcpConfirm = "";
    ctx.pluginConfirm = "";
    openOverlay(extScrim, extModal);
    renderExtModal();
    hideSkillMenu();
    Promise.all([loadMcps(), loadPlugins(), loadSkills()]);
    if (extSearch) extSearch.focus();
  }

  function askExtInstall(source, label) {
    if (!ctx.openConfirm) return;
    ctx.openConfirm({
      title: t("extTrustTitle"),
      body: t("extTrustBody").replace("{name}", label || source),
      ok: t("pluginConfirmInstall"),
      host: extModal,
      dismissOnScrim: true,
      onOk: () => pluginOp("install", { source })
    });
  }

  function extAddActions(submitLabel, busy) {
    const row = mkEl("div", "ext-add-actions");
    const cancel = mkEl("button", "ext-cancel-btn");
    cancel.type = "button";
    cancel.textContent = t("cancel");
    cancel.addEventListener("click", (e) => {
      e.preventDefault();
      e.stopPropagation();
      ctx.extAddOpen = false;
      renderExtModal();
    });
    const submit = mkEl("button", "mcp-add-btn");
    submit.type = "submit";
    submit.textContent = submitLabel;
    submit.disabled = !!busy;
    row.append(cancel, submit);
    return row;
  }

  function hideSkillMenu() {
    if (extSkillMenu) extSkillMenu.hidden = true;
    if (extCta) extCta.setAttribute("aria-expanded", "false");
  }

  function closeExtModal() {
    if (ctx.closeConfirm) ctx.closeConfirm();
    hideSkillMenu();
    closeOverlay(extScrim, { panel: extModal });
    ctx.extAddOpen = false;
    ctx.extConfirm = "";
    ctx.extOpen = "";
    ctx.mcpConfirm = "";
    ctx.pluginConfirm = "";
  }

  function fillExtAddForm() {
    if (!extAdd) return;
    const active = document.activeElement;
    const inForm = active && extAdd.contains(active);
    const tag = active && active.tagName;
    if (inForm && (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT")) {
      extAdd.querySelectorAll("button").forEach((el) => {
        el.disabled = ctx.extTab === "mcp" ? ctx.mcpBusy : ctx.extTab === "skills" ? ctx.skillBusy : ctx.pluginBusy;
      });
      return;
    }
    extAdd.replaceChildren();
    if (ctx.extTab === "skills") {
      const name = mkEl("input", "mcp-input");
      name.type = "text";
      name.name = "name";
      name.placeholder = t("skillName");
      name.autocomplete = "off";
      name.spellcheck = false;
      const desc = mkEl("input", "mcp-input");
      desc.type = "text";
      desc.name = "description";
      desc.placeholder = t("skillDesc");
      desc.autocomplete = "off";
      desc.spellcheck = false;
      const body = mkEl("textarea", "mcp-input");
      body.name = "body";
      body.placeholder = t("skillBody");
      body.rows = 8;
      extAdd.append(name, desc, body, extAddActions(t("newSkill"), ctx.skillBusy));
      return;
    }
    if (ctx.extTab === "mcp") {
      const name = mkEl("input", "mcp-input");
      name.type = "text";
      name.name = "name";
      name.placeholder = t("mcpName");
      name.autocomplete = "off";
      name.spellcheck = false;
      const row1 = mkEl("div", "ext-add-row");
      const transport = mkEl("select", "mcp-input");
      transport.name = "transport";
      [["stdio", "stdio"], ["http", "http"], ["sse", "sse"]].forEach(([v, lab]) => {
        const o = document.createElement("option");
        o.value = v;
        o.textContent = lab;
        transport.appendChild(o);
      });
      const scope = mkEl("select", "mcp-input");
      scope.name = "scope";
      [["user", t("mcpScopeUser")], ["project", t("mcpScopeProject")]].forEach(([v, lab]) => {
        const o = document.createElement("option");
        o.value = v;
        o.textContent = lab;
        scope.appendChild(o);
      });
      row1.append(transport, scope);
      const cmd = mkEl("input", "mcp-input");
      cmd.type = "text";
      cmd.name = "cmd";
      cmd.placeholder = t("mcpCommand");
      cmd.autocomplete = "off";
      cmd.spellcheck = false;
      const argsEl = mkEl("input", "mcp-input");
      argsEl.type = "text";
      argsEl.name = "args";
      argsEl.placeholder = t("mcpArgs");
      argsEl.autocomplete = "off";
      argsEl.spellcheck = false;
      extAdd.append(name, row1, cmd, argsEl, extAddActions(t("mcpAdd"), ctx.mcpBusy));
      return;
    }
    if (ctx.extTab === "marketplace") {
      (ctx.pluginData.sources || []).forEach((srcRow) => {
        const name = srcRow.name || (srcRow.source && srcRow.source.url) || srcRow.url || "";
        const line = mkEl("div", "mcp-source");
        const lab = mkEl("span", "mcp-source-name");
        const url = (srcRow.source && srcRow.source.url) || srcRow.url || "";
        lab.textContent = [name, url].filter(Boolean).join(" · ");
        const key = "rmsrc:" + name;
        const rm = mkEl("button", "mcp-text-btn danger");
        rm.type = "button";
        rm.textContent = ctx.pluginConfirm === key ? t("pluginConfirmRemoveSource") : t("pluginRemoveSource");
        rm.disabled = ctx.pluginBusy;
        rm.addEventListener("click", (e) => {
          e.stopPropagation();
          if (ctx.pluginConfirm !== key) {
            ctx.pluginConfirm = key;
            fillExtAddForm();
            return;
          }
          ctx.pluginConfirm = "";
          pluginOp("marketplace_remove", { source: name });
        });
        line.append(lab, rm);
        extAdd.appendChild(line);
      });
    }
    const src = mkEl("input", "mcp-input");
    src.type = "text";
    src.name = "source";
    src.placeholder = ctx.extTab === "marketplace" ? t("pluginSourcePh") : t("pluginInstallPh");
    src.autocomplete = "off";
    src.spellcheck = false;
    const lab = ctx.extTab === "marketplace" ? t("pluginAddSource") : t("pluginInstall");
    extAdd.append(src, extAddActions(lab, ctx.pluginBusy));
  }

  function extMkBtn(labelText, opts) {
    const b = document.createElement("button");
    b.type = "button";
    b.className = ((opts && opts.cls) || "ext-add-btn") + (opts && opts.danger ? " danger" : "");
    fillBtnContent(b, labelText, !!(opts && opts.busy));
    b.disabled = !!(opts && opts.disabled);
    b.addEventListener("click", (e) => {
      e.stopPropagation();
      if (opts && opts.onClick) opts.onClick();
    });
    return b;
  }

  function extCard(name, desc, buttons, onCard, tags) {
    const card = document.createElement("div");
    card.className = "ext-card";
    const ico = mkEl("div", "ext-ico");
    ico.textContent = extInitial(name);
    const main = mkEl("div", "ext-main");
    const nm = mkEl("div", "ext-name");
    nm.textContent = name || "—";
    if (tags && tags.length) {
      const row = mkEl("div", "ext-name-row");
      row.appendChild(nm);
      tags.forEach((tag) => {
        const el = mkEl("span", "ext-tag");
        el.textContent = tag;
        row.appendChild(el);
      });
      main.appendChild(row);
    } else {
      main.appendChild(nm);
    }
    const ds = mkEl("div", "ext-desc");
    ds.textContent = desc || "";
    main.appendChild(ds);
    card.append(ico, main);
    if (buttons && buttons.length) {
      const actions = mkEl("div", "ext-actions");
      buttons.forEach((b) => actions.appendChild(b));
      card.appendChild(actions);
    }
    if (onCard) {
      card.addEventListener("click", (e) => {
        e.stopPropagation();
        onCard();
      });
    }
    return card;
  }

  function fillExtGrid() {
    if (!extGrid) return;
    extGrid.replaceChildren();
    const empty = (key) => {
      const el = mkEl("div", "ext-empty");
      el.textContent = t(key);
      extGrid.appendChild(el);
    };
    const openDetail = (key) => {
      ctx.extOpen = key;
      ctx.extConfirm = "";
      ctx.mcpConfirm = "";
      ctx.pluginConfirm = "";
      renderExtModal();
    };
    if (ctx.extTab === "mcp") {
      const raw = (ctx.mcpData && ctx.mcpData.servers) || [];
      if (ctx.mcpBusy && !raw.length) {
        empty("loading");
        return;
      }
      const rows = mcpServers();
      if (!rows.length) {
        empty("mcpEmpty");
        return;
      }
      const manual = [];
      const fromPlugin = [];
      rows.forEach((row) => (mcpPluginName(row) ? fromPlugin : manual).push(row));
      const paint = (title, list) => {
        if (!list.length) return;
        const sec = mkEl("div", "ext-skill-sec");
        const head = mkEl("h2", "ext-sec-head");
        head.textContent = title;
        const listEl = mkEl("div", "ext-mcp-list");
        list.forEach((row) => {
          const name = String(row.name || "");
          const desc = row.description || mcpMeta(row) || t("mcpLocal");
          listEl.appendChild(extCard(name, desc, [], () => openDetail(name)));
        });
        sec.append(head, listEl);
        extGrid.appendChild(sec);
      };
      paint(t("mcpManual"), manual);
      paint(t("mcpFromPlugins"), fromPlugin);
      return;
    }
    if (ctx.extTab === "plugins") {
      const installed = ((ctx.pluginData && ctx.pluginData.plugins) || []).filter(pluginIsInstalled);
      if (ctx.pluginBusy && !installed.length) {
        empty("loading");
        return;
      }
      const rows = pluginRows("plugins");
      if (!rows.length) {
        empty("pluginEmpty");
        return;
      }
      rows.forEach((row) => {
        const name = row.name || row.id || "";
        const desc = row.description || pluginMeta(row);
        extGrid.appendChild(extCard(name, desc, [], () => openDetail(name)));
      });
      return;
    }
    if (ctx.extTab === "marketplace") {
      const available = ((ctx.pluginData && ctx.pluginData.plugins) || []).filter((row) => !pluginIsInstalled(row));
      if (ctx.pluginBusy && !available.length && !((ctx.pluginData.plugins || []).length || (ctx.pluginData.sources || []).length)) {
        empty("loading");
        return;
      }
      const rows = pluginRows("marketplace");
      if (!rows.length) {
        empty("marketEmpty");
        return;
      }
      rows.forEach((row) => {
        const name = row.name || row.id || "";
        const desc = row.description || pluginMeta(row);
        const installing = pluginBusyOn(name, "install");
        const btn = extMkBtn(installing ? t("pluginInstalling") : t("extAdd"), {
          disabled: ctx.pluginBusy,
          busy: installing,
          onClick: () => askExtInstall(name, name)
        });
        extGrid.appendChild(extCard(name, desc, [btn], () => openDetail(name)));
      });
      return;
    }
    if (ctx.extTab === "skills") {
      const anySkills = (ctx.personalSkills || []).some((s) => String(s.scope || "") !== "bundled");
      if (ctx.skillBusy && !anySkills) {
        empty("loading");
        return;
      }
      const rows = skillRows("personal");
      const sec = mkEl("div", "ext-skill-sec");
      const head = mkEl("h2", "ext-sec-head");
      head.textContent = t("personal");
      sec.appendChild(head);
      if (!rows.length) {
        extGrid.appendChild(sec);
        empty("skillEmpty");
        return;
      }
      const list = mkEl("div", "ext-skill-list");
      rows.forEach((skill) => {
        const name = (skill && (skill.name || skill.id)) || "";
        const id = String(skill.scope || "personal") + "::" + name;
        list.appendChild(
          extCard(skillI18n(skill), skillI18n(skill, "desc"), [], () => openDetail(id), [
            skillKindLabel(skillKind(skill))
          ])
        );
      });
      sec.appendChild(list);
      extGrid.appendChild(sec);
      return;
    }
    if (ctx.extTab === "quick") {
      const anyQuick = (ctx.personalSkills || []).some((s) => String(s.scope || "") === "bundled");
      if (ctx.skillBusy && !anyQuick) {
        empty("loading");
        return;
      }
      const rows = skillRows("quick");
      if (!rows.length) {
        empty("quickEmpty");
        return;
      }
      const list = mkEl("div", "ext-skill-list");
      rows.forEach((skill) => {
        const name = (skill && (skill.name || skill.id)) || "";
        const id = String(skill.scope || "bundled") + "::" + name;
        list.appendChild(
          extCard(skillI18n(skill), skillI18n(skill, "desc"), [], () => openDetail(id), [
            skillKindLabel(skillKind(skill))
          ])
        );
      });
      extGrid.appendChild(list);
    }
  }

  function extTryNow(installed, source, label) {
    if (installed) {
      useSkill({ name: label || source, kind: "slash" });
      return;
    }
    askExtInstall(source, label);
  }

  function useSkill(skill) {
    const slug = String((skill && (skill.name || skill.id)) || "").trim();
    if (!slug) return;
    const kind = skillKind(skill);
    const label = skillI18n(skill) || slug;
    const hint = String((skill && (skill.argument_hint || skill.argumentHint)) || "").trim();
    closeExtModal();
    ctx.activeSkill = { name: slug, kind, label, hint };
    if (ctx.renderChips) ctx.renderChips();
    if (kind === "guide") {
      const msg = t("skillGuidePrompt").replace("{name}", label);
      if (ctx.fillComposer) ctx.fillComposer(msg);
    } else if (ctx.focusPrompt) {
      ctx.focusPrompt();
    }
    if (ctx.paintPromptPh) ctx.paintPromptPh(false);
  }

  function fillExtDetail() {
    if (!extDetail || !ctx.extOpen) return;
    extDetail.replaceChildren();
    const backKey =
      {
        skills: "extAllSkills",
        quick: "extAllBuiltin",
        marketplace: "extAllMarket",
        plugins: "extAllPlugins",
        mcp: "extAllConnectors"
      }[ctx.extTab] || "extBack";
    const back = mkEl("button", "ext-back");
    back.type = "button";
    back.append(svgUse("i-chevron-left"), mkEl("span"));
    back.lastChild.textContent = t(backKey);
    back.addEventListener("click", (e) => {
      e.stopPropagation();
      ctx.extOpen = "";
      ctx.extConfirm = "";
      ctx.mcpConfirm = "";
      ctx.pluginConfirm = "";
      renderExtModal();
    });
    const secondary = [];
    const pushSec = (label, opts) => {
      secondary.push(extMkBtn(label, Object.assign({ cls: "mcp-text-btn" }, opts || {})));
    };
    let name = "";
    let desc = "";
    let extra = "";
    let primary = null;
    const skillCards = [];
    let tools = [];
    let skillDetail = null;
    let skillKey = "";
    if (ctx.extTab === "mcp") {
      const row = ((ctx.mcpData && ctx.mcpData.servers) || []).find((r) => String(r.name || "") === ctx.extOpen);
      if (!row) {
        if (ctx.mcpBusy) return;
        ctx.extOpen = "";
        renderExtModal();
        return;
      }
      name = String(row.name || "");
      desc = row.description || "";
      extra = mcpMeta(row) || (mcpPluginName(row) ? "" : t("mcpLocal"));
      const bits = [];
      if (row.command) bits.push(row.command);
      if (row.target) bits.push(row.target);
      if (row.url) bits.push(row.url);
      if (row.transport) bits.push(String(row.transport));
      const doc = row.doctor || {};
      const err = doc.error || doc.message || doc.detail;
      if (err) bits.push(String(err).replace(/\s+/g, " ").slice(0, 180));
      if (bits.length) extra = extra ? extra + " · " + bits.join(" · ") : bits.join(" · ");
      tools = mcpTools(row);
      const plug = mcpPluginName(row);
      if (plug) {
        pushSec(t("viewPlugin"), {
          onClick: () => {
            ctx.extTab = "plugins";
            ctx.extOpen = plug;
            loadPlugins().then(() => {
              ctx.extTab = "plugins";
              ctx.extOpen = plug;
              renderExtModal();
            });
          }
        });
      } else {
        const off = row.enabled === false || row.disabled;
        pushSec(off ? t("mcpEnable") : t("mcpDisable"), {
          disabled: ctx.mcpBusy,
          onClick: () => mcpOp(off ? "enable" : "disable", { name })
        });
        pushSec(ctx.mcpConfirm === name ? t("mcpConfirmRemove") : t("mcpRemove"), {
          danger: true,
          disabled: ctx.mcpBusy,
          onClick: () => {
            if (ctx.mcpConfirm !== name) {
              ctx.mcpConfirm = name;
              fillExtDetail();
              return;
            }
            ctx.mcpConfirm = "";
            mcpOp("remove", { name, scope: row.scope || "" });
          }
        });
      }
    } else if (ctx.extTab === "plugins" || ctx.extTab === "marketplace") {
      const row = ((ctx.pluginData && ctx.pluginData.plugins) || []).find((r) => (r.name || r.id || "") === ctx.extOpen);
      if (!row) {
        if (ctx.pluginBusy) return;
        ctx.extOpen = "";
        renderExtModal();
        return;
      }
      name = row.name || row.id || "";
      desc = row.description || "";
      extra = pluginMeta(row);
      const installed = pluginIsInstalled(row);
      if (installed) {
        const disabled = String(row.status || "").toLowerCase() === "disabled";
        pushSec(disabled ? t("mcpEnable") : t("mcpDisable"), {
          disabled: ctx.pluginBusy,
          onClick: () => pluginOp(disabled ? "enable" : "disable", { name })
        });
        pushSec(t("pluginUpdate"), {
          disabled: ctx.pluginBusy,
          onClick: () => pluginOp("update", { name })
        });
        const key = "uninstall:" + name;
        pushSec(ctx.pluginConfirm === key ? t("pluginConfirmUninstall") : t("pluginUninstall"), {
          danger: true,
          disabled: ctx.pluginBusy,
          onClick: () => {
            if (ctx.pluginConfirm !== key) {
              ctx.pluginConfirm = key;
              fillExtDetail();
              return;
            }
            ctx.pluginConfirm = "";
            pluginOp("uninstall", { name });
          }
        });
        const cons = pluginConnectorNames(row);
        if (cons[0]) {
          pushSec(t("viewConnector"), {
            onClick: () => {
              ctx.extTab = "mcp";
              ctx.extOpen = cons[0];
              loadMcps().then(() => {
                ctx.extTab = "mcp";
                ctx.extOpen = cons[0];
                renderExtModal();
              });
            }
          });
        }
      } else {
        const installing = pluginBusyOn(name, "install");
        primary = {
          label: installing ? t("pluginInstalling") : t("pluginInstall"),
          disabled: ctx.pluginBusy,
          busy: installing,
          onClick: () => askExtInstall(name, name)
        };
      }
      if (ctx.extTab === "marketplace") {
        const rawSkills = (row.components && Array.isArray(row.components.skills) && row.components.skills) || [];
        const shown = rawSkills.length <= 4 ? rawSkills : rawSkills.slice(0, 3);
        shown.forEach((skill) => {
          const sn = (skill && (skill.name || skill.id)) || "";
          if (!sn) return;
          skillCards.push({
            name: sn,
            desc: (skill && skill.description) || "",
            parent: name,
            installed: installed
          });
        });
      }
    } else if (ctx.extTab === "skills" || ctx.extTab === "quick") {
      const sep = ctx.extOpen.indexOf("::");
      const parent = sep >= 0 ? ctx.extOpen.slice(0, sep) : "";
      const skillName = sep >= 0 ? ctx.extOpen.slice(sep + 2) : ctx.extOpen;
      const found =
        (ctx.personalSkills || []).find((s) => {
          const n = String(s.name || s.id || "");
          const sc = String(s.scope || "");
          if (n !== skillName) return false;
          if (!parent || parent === "personal") return sc !== "bundled";
          return sc === parent;
        }) ||
        (ctx.personalSkills || []).find((s) => String(s.name || s.id || "") === skillName) ||
        null;
      if (!found) {
        ctx.extOpen = "";
        renderExtModal();
        return;
      }
      name = skillI18n(found);
      desc = skillI18n(found, "desc");
      const kind = skillKind(found);
      extra = [skillKindLabel(kind)];
      extra.push(
        found.scope === "bundled"
          ? t("skillBuiltin")
          : found.scope === "project"
            ? t("mcpScopeProject")
            : t("personal")
      );
      extra = extra.filter(Boolean).join(" · ");
      const cta =
        kind === "auto" ? t("skillUseInChat") : kind === "guide" ? t("skillWritePrompt") : t("skillInsertCmd");
      primary = {
        label: cta,
        disabled: false,
        onClick: () => useSkill(found)
      };
      skillKey = String(found.scope || "") + "::" + String(found.name || found.id || "");
      skillDetail = ctx.skillDetailCache && ctx.skillDetailCache[skillKey];
      if (!skillDetail && ctx.skillDetailLoading !== skillKey) {
        ctx.skillDetailLoading = skillKey;
        loadSkillDetail(found)
          .then((data) => {
            if ((ctx.extTab === "skills" || ctx.extTab === "quick") && ctx.extOpen && ctx.extOpen.indexOf(String(found.name || found.id || "")) >= 0) {
              fillExtDetail();
            }
          })
          .catch((e) => toast(e))
          .finally(() => {
            if (ctx.skillDetailLoading === skillKey) ctx.skillDetailLoading = "";
          });
      }
    } else {
      ctx.extOpen = "";
      renderExtModal();
      return;
    }
    const scroll = mkEl("div", "ext-detail-scroll");
    const hero = mkEl("div", "ext-detail-hero");
    const ico = mkEl("div", "ext-detail-ico");
    ico.textContent = extInitial(name);
    const titles = mkEl("div", "ext-detail-main");
    const title = mkEl("div", "ext-detail-name");
    title.textContent = name || "—";
    titles.appendChild(title);
    hero.append(ico, titles);
    scroll.append(back, hero);
    if (desc) {
      const body = mkEl("div", "ext-detail-body");
      body.textContent = desc;
      scroll.appendChild(body);
    }
    if (extra) {
      const sub = mkEl("div", "ext-detail-sub");
      sub.textContent = extra;
      scroll.appendChild(sub);
    }
    if (ctx.extTab === "mcp") {
      const sec = mkEl("div", "ext-try-sec");
      const head = mkEl("div", "ext-try-head");
      head.textContent = t("mcpTools");
      sec.appendChild(head);
      if (!tools.length) {
        const empty = mkEl("div", "ext-detail-sub");
        empty.textContent = t("mcpNoTools");
        sec.appendChild(empty);
      } else {
        const list = mkEl("div", "ext-tool-list");
        tools.forEach((tool) => {
          const card = mkEl("div", "ext-tool-row");
          const nm = mkEl("div", "ext-try-name");
          nm.textContent = tool.name;
          card.appendChild(nm);
          if (tool.description) {
            const ds = mkEl("div", "ext-try-desc");
            ds.textContent = tool.description;
            card.appendChild(ds);
          }
          list.appendChild(card);
        });
        sec.appendChild(list);
      }
      scroll.appendChild(sec);
    }
    if (ctx.extTab === "skills") {
      const sec = mkEl("div", "ext-try-sec");
      const head = mkEl("div", "ext-try-head");
      head.textContent = t("skillFiles");
      sec.appendChild(head);
      if (!skillDetail && ctx.skillDetailLoading === skillKey) {
        const loading = mkEl("div", "ext-detail-sub");
        loading.textContent = t("skillLoading");
        sec.appendChild(loading);
      } else if (skillDetail) {
        const files = Array.isArray(skillDetail.files) ? skillDetail.files : [];
        const tabs = mkEl("div", "ext-file-tabs");
        const body = mkEl("div", "ext-file-body md");
        const current =
          ctx.skillFileTab && files.some((f) => f.name === ctx.skillFileTab)
            ? ctx.skillFileTab
            : (files[0] && files[0].name) || "SKILL.md";
        ctx.skillFileTab = current;
        const paint = (file) => {
          body.replaceChildren();
          if (!file) return;
          if (file.text == null || file.kind === "binary") {
            const p = mkEl("div", "ext-detail-sub");
            p.textContent = t("skillBinary") + (file.bytes ? " · " + file.bytes + " B" : "");
            body.appendChild(p);
            return;
          }
          if (file.kind === "markdown" || /\.md$/i.test(file.name || "")) {
            body.innerHTML = renderMarkdown(file.text);
            bindCodeCopy(body);
            return;
          }
          const lang = String(file.name || "").split(".").pop() || "text";
          body.innerHTML = codeCardHtml(lang, file.text);
          bindCodeCopy(body);
        };
        if (!files.length) {
          const md = skillDetail.markdown;
          if (md) {
            body.innerHTML = renderMarkdown(md);
            bindCodeCopy(body);
            sec.appendChild(body);
          }
        } else {
          files.forEach((file) => {
            const tab = mkEl("button", "ext-file-tab" + (file.name === current ? " on" : ""));
            tab.type = "button";
            tab.textContent = file.name;
            tab.addEventListener("click", (e) => {
              e.stopPropagation();
              ctx.skillFileTab = file.name;
              tabs.querySelectorAll(".ext-file-tab").forEach((el) => el.classList.toggle("on", el === tab));
              paint(file);
            });
            tabs.appendChild(tab);
          });
          sec.append(tabs, body);
          paint(files.find((f) => f.name === current) || files[0]);
        }
      }
      scroll.appendChild(sec);
    }
    if (skillCards.length) {
      const sec = mkEl("div", "ext-try-sec");
      const head = mkEl("div", "ext-try-head");
      head.textContent = t("skills");
      const grid = mkEl("div", "ext-try-grid");
      skillCards.forEach((sk) => {
        const card = mkEl("div", "ext-try-card");
        card.setAttribute("role", "button");
        card.tabIndex = 0;
        const nm = mkEl("div", "ext-try-name");
        nm.textContent = sk.name;
        card.appendChild(nm);
        if (sk.desc) {
          const ds = mkEl("div", "ext-try-desc");
          ds.textContent = sk.desc;
          card.appendChild(ds);
        }
        const plus = mkEl("button", "ext-try-plus");
        plus.type = "button";
        plus.setAttribute("aria-label", t("extTryNow"));
        plus.appendChild(svgUse("i-plus-circle"));
        plus.addEventListener("click", (e) => {
          e.stopPropagation();
          extTryNow(sk.installed, sk.parent, sk.name);
        });
        card.appendChild(plus);
        const openSkill = () => {
          ctx.extTab = "skills";
          ctx.extOpen = sk.parent + "::" + sk.name;
          ctx.extConfirm = "";
          ctx.pluginConfirm = "";
          renderExtModal();
        };
        card.addEventListener("click", (e) => {
          e.stopPropagation();
          openSkill();
        });
        card.addEventListener("keydown", (e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            openSkill();
          }
        });
        grid.appendChild(card);
      });
      sec.append(head, grid);
      scroll.appendChild(sec);
    }
    extDetail.appendChild(scroll);
    if (secondary.length || primary) {
      const foot = mkEl("div", "ext-detail-foot");
      const left = mkEl("div", "ext-detail-foot-left");
      secondary.forEach((b) => left.appendChild(b));
      foot.appendChild(left);
      if (primary) {
        const cta = mkEl("button", "ext-cta");
        cta.type = "button";
        fillBtnContent(cta, primary.label, !!primary.busy);
        cta.disabled = !!primary.disabled;
        cta.addEventListener("click", (e) => {
          e.stopPropagation();
          if (primary.onClick) primary.onClick();
        });
        foot.appendChild(cta);
      }
      extDetail.appendChild(foot);
    }
  }

  function renderExtModal() {
    if (!extModal || extModal.hidden) return;
    if (extTabs) {
      extTabs.setAttribute("role", "tablist");
      extTabs.replaceChildren();
      [
        ["mcp", t("connectors")],
        ["plugins", t("plugins")],
        ["marketplace", t("marketplace")],
        ["skills", t("mySkills")],
        ["quick", t("builtinSkills")]
      ].forEach(([id, lab]) => {
        const b = document.createElement("button");
        b.type = "button";
        b.setAttribute("role", "tab");
        b.setAttribute("aria-selected", ctx.extTab === id ? "true" : "false");
        b.className = "ext-tab" + (ctx.extTab === id ? " on" : "");
        if (ctx.extTab === id) {
          const glow = document.createElement("span");
          glow.className = "ext-tab-glow";
          glow.setAttribute("aria-hidden", "true");
          b.appendChild(glow);
        }
        const labEl = document.createElement("span");
        labEl.className = "ext-tab-lab";
        labEl.textContent = lab;
        b.appendChild(labEl);
        b.addEventListener("click", (e) => {
          e.stopPropagation();
          ctx.extTab = id;
          ctx.extConfirm = "";
          ctx.extOpen = "";
          ctx.extAddOpen = false;
          ctx.mcpConfirm = "";
          ctx.pluginConfirm = "";
          hideSkillMenu();
          if (id === "skills" || id === "quick") loadSkills();
          renderExtModal();
        });
        extTabs.appendChild(b);
      });
    }
    if (extSearch) {
      extSearch.placeholder = t("extSearch");
      if (!extComposing) extSearch.value = ctx.extQuery || "";
    }
    if (extCta) {
      const ctaKey = { mcp: "newConnector", plugins: "pluginInstall", marketplace: "pluginAddSource", skills: "newSkill" }[ctx.extTab];
      extCta.hidden = !ctaKey;
      if (ctaKey) extCta.textContent = t(ctaKey);
      extCta.disabled = ctx.extTab === "mcp" ? ctx.mcpBusy : ctx.extTab === "skills" || ctx.extTab === "quick" ? ctx.skillBusy : ctx.pluginBusy;
      if (ctx.extTab === "skills") {
        extCta.setAttribute("aria-haspopup", "menu");
        extCta.setAttribute("aria-expanded", extSkillMenu && !extSkillMenu.hidden ? "true" : "false");
      } else {
        extCta.removeAttribute("aria-haspopup");
        hideSkillMenu();
      }
    }
    if (extGrid) {
      extGrid.classList.toggle("skills", ctx.extTab === "skills" || ctx.extTab === "quick");
      if (ctx.extTab === "skills" || ctx.extTab === "quick") extGrid.dataset.tab = ctx.extTab;
      else delete extGrid.dataset.tab;
    }
    const inDetail = !!ctx.extOpen;
    if (extGrid) extGrid.hidden = inDetail;
    if (extDetail) extDetail.hidden = !inDetail;
    if (extAdd) {
      if (!inDetail && ctx.extAddOpen) {
        extAdd.hidden = false;
        fillExtAddForm();
      } else {
        extAdd.hidden = true;
        if (!inDetail) extAdd.replaceChildren();
      }
    }
    if (inDetail) fillExtDetail();
    else fillExtGrid();
  }

  if (extBtn) {
    extBtn.addEventListener("click", (e) => {
      e.stopPropagation();
      openExtModal(ctx.extTab || "mcp");
    });
  }

  if (extScrim) {
    extScrim.addEventListener("click", closeExtModal);
  }

  if (extModal) {
    extModal.addEventListener("click", (e) => e.stopPropagation());
  }

  if (extSearch) {
    extSearch.addEventListener("compositionstart", () => {
      extComposing = true;
    });
    extSearch.addEventListener("compositionend", () => {
      extComposing = false;
      ctx.extQuery = extSearch.value;
      if (ctx.extOpen) {
        ctx.extOpen = "";
        renderExtModal();
        return;
      }
      fillExtGrid();
    });
    extSearch.addEventListener("input", (e) => {
      ctx.extQuery = extSearch.value;
      if (extComposing || e.isComposing) return;
      if (ctx.extOpen) {
        ctx.extOpen = "";
        renderExtModal();
        return;
      }
      fillExtGrid();
    });
  }

  if (extCta) {
    extCta.addEventListener("click", (e) => {
      e.stopPropagation();
      if (ctx.extTab === "skills") {
        if (!extSkillMenu) return;
        const open = extSkillMenu.hidden;
        extSkillMenu.hidden = !open;
        extCta.setAttribute("aria-expanded", open ? "true" : "false");
        return;
      }
      hideSkillMenu();
      ctx.extAddOpen = !ctx.extAddOpen;
      renderExtModal();
    });
  }

  if (extSkillMenu) {
    extSkillMenu.addEventListener("click", (e) => e.stopPropagation());
    extSkillMenu.querySelectorAll("[data-skill-act]").forEach((btn) => {
      btn.addEventListener("click", (e) => {
        e.stopPropagation();
        const act = btn.getAttribute("data-skill-act");
        hideSkillMenu();
        if (act === "write") {
          ctx.extAddOpen = true;
          renderExtModal();
          return;
        }
        if (act === "upload") {
          if (extSkillFile) {
            extSkillFile.value = "";
            extSkillFile.click();
          }
          return;
        }
        if (act === "ai") {
          closeExtModal();
          if (ctx.startNewChat) ctx.startNewChat();
          const lang = (window.I18n && window.I18n.lang) || "en";
          if (ctx.fillComposer) {
            ctx.fillComposer(
              lang === "zh"
                ? "请写一个技能。先问我它要做什么，然后输出完整的 SKILL.md（YAML frontmatter 含 name 与 description，后面是步骤）"
                : "Please write a skill. First ask me what it should do, then output a complete SKILL.md (YAML frontmatter with name and description, then the steps)."
            );
          }
        }
      });
    });
  }

  if (extSkillFile) {
    extSkillFile.addEventListener("change", async () => {
      const file = extSkillFile.files && extSkillFile.files[0];
      if (!file) return;
      const fd = new FormData();
      const cwd = pluginCwd();
      if (cwd) fd.append("cwd", cwd);
      fd.append("file", file);
      ctx.skillBusy = true;
      renderExtModal();
      try {
        await api("/api/skills/upload", { method: "POST", body: fd });
        await loadSkills();
      } catch (err) {
        toast(String(err.message || err));
      }
      ctx.skillBusy = false;
      renderExtModal();
    });
  }

  if (extAdd) {
    extAdd.addEventListener("submit", (e) => {
      e.preventDefault();
      if (ctx.extTab === "skills") {
        const nameVal = String(((extAdd.querySelector("[name=name]") || {}).value) || "").trim();
        const descVal = String(((extAdd.querySelector("[name=description]") || {}).value) || "").trim();
        const bodyVal = String(((extAdd.querySelector("[name=body]") || {}).value) || "");
        if (!nameVal || !descVal) {
          toast(t("skillCreateNeed"));
          return;
        }
        ctx.skillBusy = true;
        renderExtModal();
        post("/api/skills", { op: "create", name: nameVal, description: descVal, body: bodyVal, cwd: pluginCwd() })
          .then(() => {
            ctx.extAddOpen = false;
            return loadSkills();
          })
          .catch((err) => toast(String(err.message || err)))
          .finally(() => {
            ctx.skillBusy = false;
            renderExtModal();
          });
        return;
      }
      if (ctx.extTab === "mcp") {
        const fname = (extAdd.querySelector("[name=name]") || {}).value;
        const transport = (extAdd.querySelector("[name=transport]") || {}).value;
        const command_or_url = (extAdd.querySelector("[name=cmd]") || {}).value;
        const scope = (extAdd.querySelector("[name=scope]") || {}).value;
        const extra = (extAdd.querySelector("[name=args]") || {}).value;
        const nameVal = String(fname || "").trim();
        const cmdVal = String(command_or_url || "").trim();
        if (!nameVal || !cmdVal) {
          toast(t("mcpAddNeed"));
          return;
        }
        const args = String(extra || "").trim() ? String(extra).trim().split(/\s+/).filter(Boolean) : [];
        mcpOp("add", { name: nameVal, transport, command_or_url: cmdVal, args, scope });
        return;
      }
      const value = String(((extAdd.querySelector("[name=source]") || {}).value) || "").trim();
      if (!value) {
        toast(t("pluginAddNeed"));
        return;
      }
      if (ctx.extTab === "marketplace") pluginOp("marketplace_add", { source: value });
      else pluginOp("install", { source: value });
    });
    extAdd.addEventListener("click", (e) => e.stopPropagation());
  }

  ctx.openExtModal = openExtModal;
  ctx.closeExtModal = closeExtModal;
  ctx.extModalOpen = extModalOpen;
  ctx.renderExtModal = renderExtModal;
  ctx.fillExtGrid = fillExtGrid;
  ctx.fillExtDetail = fillExtDetail;
  ctx.fillExtAddForm = fillExtAddForm;
  ctx.extCard = extCard;
  ctx.extMkBtn = extMkBtn;
  ctx.loadMcps = loadMcps;
  ctx.mcpOp = mcpOp;
  ctx.mcpCwd = mcpCwd;
  ctx.mcpServers = mcpServers;
  ctx.mcpMeta = mcpMeta;
  ctx.mcpHealthy = mcpHealthy;
  ctx.loadPlugins = loadPlugins;
  ctx.pluginOp = pluginOp;
  ctx.pluginCwd = pluginCwd;
  ctx.pluginIsInstalled = pluginIsInstalled;
  ctx.pluginRows = pluginRows;
  ctx.pluginMeta = pluginMeta;
  ctx.loadSkills = loadSkills;
  ctx.skillRows = skillRows;
  ctx.skillI18n = skillI18n;
  ctx.skillI18nKey = skillI18nKey;
  ctx.trySkill = useSkill;
  ctx.useSkill = useSkill;
  ctx.extTryNow = extTryNow;
  ctx.extAddActions = extAddActions;
  ctx.askExtInstall = askExtInstall;
  ctx.hideSkillMenu = hideSkillMenu;
  ctx.extInitial = extInitial;
}
