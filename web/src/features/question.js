import { t } from "../lib/helpers.js";
import { post } from "../lib/api.js";
import { renderMarkdown } from "../lib/markdown.js";
import { toast } from "../lib/clipboard.js";

function optionLabel(opt) {
  if (opt == null) return "";
  if (typeof opt === "string" || typeof opt === "number") return String(opt).trim();
  return String(
    opt.label || opt.name || opt.title || opt.text || opt.value || opt.const || opt.id || ""
  ).trim();
}

function optionDesc(opt) {
  if (!opt || typeof opt !== "object") return "";
  return String(opt.description || opt.desc || opt.detail || "").trim();
}

function optionPreview(opt) {
  if (!opt || typeof opt !== "object") return "";
  return String(opt.preview || "").trim();
}

function questionOptions(q) {
  const raw = (q && (q.options || q.choices || q.items || q.oneOf)) || [];
  if (Array.isArray(raw)) {
    return raw
      .map((opt) => ({
        label: optionLabel(opt),
        description: optionDesc(opt),
        preview: optionPreview(opt)
      }))
      .filter((o) => o.label);
  }
  if (raw && typeof raw === "object") {
    return Object.keys(raw)
      .map((key) => ({
        label: String(key || "").trim(),
        description: typeof raw[key] === "string" ? String(raw[key]).trim() : optionDesc(raw[key]),
        preview: optionPreview(raw[key])
      }))
      .filter((o) => o.label);
  }
  return [];
}

function reqList(map) {
  return Object.keys(map || {}).sort();
}

function sameReqs(a, b) {
  const left = reqList(a);
  const right = reqList(b);
  return left.length === right.length && left.every((k, i) => k === right[i]);
}

function ensureDraft(ctx, card) {
  const n = (card.questions || []).length;
  const prev = ctx.questionDrafts && ctx.questionDrafts[card.req];
  if (prev && Array.isArray(prev.items) && prev.items.length === n) return prev;
  const next = {
    items: (card.questions || []).map(() => ({ labels: [], other: false, notes: "" }))
  };
  if (!ctx.questionDrafts) ctx.questionDrafts = {};
  ctx.questionDrafts[card.req] = next;
  return next;
}

function selectedSet(item) {
  return new Set(item && Array.isArray(item.labels) ? item.labels : []);
}

function questionComplete(q, item) {
  if (!item) return false;
  if (item.other) return String(item.notes || "").trim().length > 0;
  return selectedSet(item).size > 0;
}

function allComplete(card, draft) {
  return (card.questions || []).every((q, i) => questionComplete(q, draft.items[i]));
}

function buildReply(card, draft, outcome) {
  if (outcome !== "accepted") return { outcome };
  const answers = {};
  const notes = {};
  (card.questions || []).forEach((q, i) => {
    const item = draft.items[i] || { labels: [], other: false, notes: "" };
    const note = String(item.notes || "").trim();
    const labels = [...selectedSet(item)];
    if (item.other && note) {
      if (q.multi_select || q.multiSelect) answers[q.question] = labels.concat([note]);
      else answers[q.question] = note;
      notes[q.question] = note;
      return;
    }
    if (q.multi_select || q.multiSelect) answers[q.question] = labels;
    else answers[q.question] = labels[0] || note;
    if (note) notes[q.question] = note;
  });
  return { outcome: "accepted", answers, notes };
}

export function bindQuestion(ctx) {
  function applyQuestions(list) {
    const next = {};
    for (const q of list || []) {
      if (q && q.req) next[q.req] = q;
    }
    const prev = ctx.pendingQuestions || {};
    const same = sameReqs(prev, next);
    ctx.pendingQuestions = next;
    if (ctx.questionDrafts) {
      for (const req of Object.keys(ctx.questionDrafts)) {
        if (!next[req]) delete ctx.questionDrafts[req];
      }
    }
    if (!same && ctx.scheduleRender) ctx.scheduleRender();
    else if (ctx.syncQuestionCards) ctx.syncQuestionCards();
  }

  function hostRow() {
    const timeline = document.getElementById("timeline");
    if (!timeline) return null;
    const turns = timeline.querySelectorAll(":scope > article.turn");
    if (turns.length) return turns[turns.length - 1];
    return timeline;
  }

  function syncQuestionCards() {
    const timeline = document.getElementById("timeline");
    if (!timeline) return;
    const writable = ctx.writable !== false && !!(ctx.pendingQuestions && Object.keys(ctx.pendingQuestions).length);
    const host = hostRow();
    const cards = [...timeline.querySelectorAll(".q-card")];
    if (!writable) {
      for (const el of cards) el.remove();
      return;
    }
    if (!host) return;
    const pending = reqList(ctx.pendingQuestions).map((req) => ctx.pendingQuestions[req]);
    const byReq = new Map(cards.map((el) => [el.dataset.req, el]));
    const keep = new Set();
    for (const card of pending) {
      keep.add(card.req);
      let el = byReq.get(card.req);
      if (!el || el.dataset.fp !== questionFp(card)) {
        const next = renderQuestionCard(card);
        if (el) el.replaceWith(next);
        else host.appendChild(next);
        el = next;
      } else if (el.parentNode !== host) {
        host.appendChild(el);
      }
    }
    for (const el of cards) {
      if (!keep.has(el.dataset.req)) el.remove();
    }
  }

  function questionFp(card) {
    return JSON.stringify({
      req: card.req,
      questions: (card.questions || []).map((q) => ({
        question: q.question,
        header: q.header,
        multi: !!(q.multi_select || q.multiSelect),
        options: questionOptions(q).map((o) => [o.label, o.description, o.preview || ""])
      }))
    });
  }

  function renderQuestionCard(card) {
    const draft = ensureDraft(ctx, card);
    const el = document.createElement("div");
    el.className = "q-card";
    el.dataset.req = card.req;
    el.dataset.fp = questionFp(card);
    el.setAttribute("role", "group");
    el.setAttribute("aria-label", t("questionTitle"));

    const head = document.createElement("div");
    head.className = "q-head";
    const kicker = document.createElement("span");
    kicker.className = "q-kicker";
    kicker.setAttribute("data-i18n", "questionTitle");
    kicker.textContent = t("questionTitle");
    head.appendChild(kicker);
    if ((card.questions || []).length > 1) {
      const tabs = document.createElement("div");
      tabs.className = "q-tabs";
      card.questions.forEach((q, i) => {
        const tab = document.createElement("button");
        tab.type = "button";
        tab.className = "q-tab";
        tab.textContent = q.header || String(i + 1);
        tab.addEventListener("click", () => {
          const target = el.querySelector('[data-q="' + i + '"]');
          if (target) target.scrollIntoView({ block: "nearest" });
        });
        tabs.appendChild(tab);
      });
      head.appendChild(tabs);
    }
    el.appendChild(head);

    const body = document.createElement("div");
    body.className = "q-body";
    (card.questions || []).forEach((q, i) => {
      body.appendChild(renderQuestionItem(card, draft, i, () => paintCard(el, card, draft)));
    });
    el.appendChild(body);

    const actions = document.createElement("div");
    actions.className = "q-actions";
    const skip = document.createElement("button");
    skip.type = "button";
    skip.className = "q-skip";
    skip.setAttribute("data-i18n", "questionSkip");
    skip.textContent = t("questionSkip");
    skip.addEventListener("click", () => submitCard(card, draft, "skip_interview", el));
    const chat = document.createElement("button");
    chat.type = "button";
    chat.className = "q-chat";
    chat.setAttribute("data-i18n", "questionChat");
    chat.textContent = t("questionChat");
    chat.addEventListener("click", () => submitCard(card, draft, "chat_about_this", el));
    const submit = document.createElement("button");
    submit.type = "button";
    submit.className = "q-submit";
    submit.setAttribute("data-i18n", "questionSubmit");
    submit.textContent = t("questionSubmit");
    submit.addEventListener("click", () => submitCard(card, draft, "accepted", el));
    actions.append(skip, chat, submit);
    el.appendChild(actions);
    paintCard(el, card, draft);
    return el;
  }

  function renderQuestionItem(card, draft, idx, onChange) {
    const q = card.questions[idx];
    const wrap = document.createElement("section");
    wrap.className = "q-item";
    wrap.dataset.q = String(idx);
    const title = document.createElement("div");
    title.className = "q-question";
    title.textContent = q.question || "";
    wrap.appendChild(title);
    if (q.multi_select || q.multiSelect) {
      const hint = document.createElement("div");
      hint.className = "q-hint";
      hint.setAttribute("data-i18n", "questionMulti");
      hint.textContent = t("questionMulti");
      wrap.appendChild(hint);
    }
    const opts = document.createElement("div");
    opts.className = "q-opts";
    questionOptions(q).forEach((opt) => {
      opts.appendChild(optionButton(q, draft, idx, opt.label, opt.description, false, onChange));
    });
    opts.appendChild(optionButton(q, draft, idx, t("questionOther"), "", true, onChange));
    wrap.appendChild(opts);
    const other = document.createElement("textarea");
    other.className = "q-other";
    other.rows = 2;
    other.setAttribute("data-i18n-placeholder", "questionOtherPh");
    other.placeholder = t("questionOtherPh");
    other.value = draft.items[idx].notes || "";
    other.addEventListener("input", () => {
      draft.items[idx].notes = other.value;
      onChange();
    });
    other.addEventListener("keydown", (e) => e.stopPropagation());
    wrap.appendChild(other);
    const preview = document.createElement("div");
    preview.className = "q-preview md";
    wrap.appendChild(preview);
    return wrap;
  }

  function optionButton(q, draft, idx, label, desc, isOther, onChange) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "q-opt" + (isOther ? " other" : "");
    btn.dataset.label = isOther ? "" : label;
    btn.dataset.other = isOther ? "1" : "0";
    const name = document.createElement("span");
    name.className = "q-opt-label";
    name.textContent = label;
    if (isOther) name.setAttribute("data-i18n", "questionOther");
    btn.appendChild(name);
    if (desc) {
      const d = document.createElement("span");
      d.className = "q-opt-desc";
      d.textContent = desc;
      btn.appendChild(d);
    }
    btn.addEventListener("click", () => {
      const item = draft.items[idx];
      if (isOther) {
        if (q.multi_select || q.multiSelect) item.other = !item.other;
        else {
          item.other = true;
          item.labels = [];
        }
      } else if (q.multi_select || q.multiSelect) {
        const set = selectedSet(item);
        if (set.has(label)) set.delete(label);
        else set.add(label);
        item.labels = [...set];
      } else {
        item.other = false;
        item.labels = [label];
      }
      onChange();
    });
    return btn;
  }

  function paintCard(el, card, draft) {
    (card.questions || []).forEach((q, i) => {
      const item = draft.items[i];
      const section = el.querySelector('[data-q="' + i + '"]');
      if (!section) return;
      const selected = selectedSet(item);
      for (const btn of section.querySelectorAll(".q-opt")) {
        const other = btn.dataset.other === "1";
        const on = other ? !!item.other : selected.has(btn.dataset.label);
        btn.classList.toggle("selected", on);
        btn.setAttribute("aria-pressed", on ? "true" : "false");
      }
      const other = section.querySelector(".q-other");
      if (other) {
        other.hidden = !item.other;
        if (item.other && other.value !== (item.notes || "")) other.value = item.notes || "";
      }
      const preview = section.querySelector(".q-preview");
      if (preview) {
        let text = "";
        if (!(q.multi_select || q.multiSelect) && selected.size === 1) {
          const label = [...selected][0];
          const opt = questionOptions(q).find((o) => o.label === label);
          text = (opt && opt.preview) || "";
        }
        preview.hidden = !text;
        if (text && preview.dataset.src !== text) {
          preview.dataset.src = text;
          preview.innerHTML = renderMarkdown(text);
        }
      }
    });
    const submit = el.querySelector(".q-submit");
    if (submit) submit.disabled = !allComplete(card, draft) || el.classList.contains("busy");
  }

  async function submitCard(card, draft, outcome, el) {
    if (!ctx.currentId || el.classList.contains("busy")) return;
    if (outcome === "accepted" && !allComplete(card, draft)) {
      toast(t("questionNeedAnswers"));
      return;
    }
    el.classList.add("busy");
    paintCard(el, card, draft);
    try {
      await post(
        "/api/sessions/" + encodeURIComponent(ctx.currentId) + "/questions/" + encodeURIComponent(card.req),
        buildReply(card, draft, outcome)
      );
      if (ctx.pendingQuestions) delete ctx.pendingQuestions[card.req];
      if (ctx.questionDrafts) delete ctx.questionDrafts[card.req];
      if (ctx.scheduleRender) ctx.scheduleRender();
    } catch (e) {
      el.classList.remove("busy");
      paintCard(el, card, draft);
      toast(String(e.message || e));
    }
  }

  ctx.applyQuestions = applyQuestions;
  ctx.applyQuestionsFromSession = function applyQuestionsFromSession(list) {
    const rows = Array.isArray(list) ? list : [];
    const live =
      ctx.es &&
      ctx.es.readyState === EventSource.OPEN &&
      ctx.pendingQuestions &&
      Object.keys(ctx.pendingQuestions).length;
    if (!rows.length && live) return;
    applyQuestions(rows);
  };
  ctx.syncQuestionCards = syncQuestionCards;
  ctx.resetQuestions = function resetQuestions() {
    ctx.pendingQuestions = {};
    ctx.questionDrafts = {};
  };
}
