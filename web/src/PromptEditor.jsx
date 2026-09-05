import { useEffect } from "react";
import { EditorContent, useEditor } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import Placeholder from "@tiptap/extension-placeholder";
import { promptApi } from "./promptApi.js";

function textOf(editor) {
  if (!editor) return "";
  return editor.getText({ blockSeparator: "\n" });
}

function posFromOffset(editor, offset) {
  const doc = editor.state.doc;
  const maxOff = textOf(editor).length;
  const goal = Math.max(0, Math.min(offset, maxOff));
  let seen = 0;
  let target = 1;
  let blocks = 0;
  doc.forEach((node, pos) => {
    if (node.type.name === "paragraph" || node.isBlock) {
      if (blocks > 0) {
        if (seen === goal) {
          target = pos + 1;
          return false;
        }
        seen += 1;
      }
      const t = node.textContent;
      if (seen + t.length >= goal) {
        target = pos + 1 + (goal - seen);
        return false;
      }
      seen += t.length;
      blocks += 1;
    }
    return true;
  });
  const maxPos = doc.content.size;
  return Math.max(1, Math.min(target, maxPos));
}

function offsetFromEditor(editor) {
  const from = editor.state.selection.from;
  return editor.state.doc.textBetween(1, from, "\n").length;
}

function syncSendOrb(text) {
  const btn = document.getElementById("send-btn");
  if (!btn) return;
  const running = btn.classList.contains("stopping");
  btn.classList.toggle("has-text", !!(text && text.trim()) || running);
  const shell = document.querySelector(".prompt-shell");
  if (shell) shell.classList.toggle("has-text", !!(text && text.trim()));
}

let composerFocusViaKeyboard = false;
if (typeof document !== "undefined" && !window.__ggokKbFocusBound) {
  window.__ggokKbFocusBound = true;
  document.addEventListener("keydown", () => {
    composerFocusViaKeyboard = true;
  }, true);
  document.addEventListener("pointerdown", () => {
    composerFocusViaKeyboard = false;
  }, true);
}

function markComposerFocus(on) {
  const inner = document.querySelector(".composer-inner");
  if (!inner) return;
  inner.classList.toggle("focused", on);
  if (on && composerFocusViaKeyboard) inner.setAttribute("data-keyboard-focus", "");
  else inner.removeAttribute("data-keyboard-focus");
}

export function PromptEditor() {
  const editor = useEditor({
    immediatelyRender: false,
    extensions: [
      StarterKit.configure({
        heading: false,
        bold: false,
        italic: false,
        strike: false,
        code: false,
        codeBlock: false,
        blockquote: false,
        bulletList: false,
        orderedList: false,
        listItem: false,
        horizontalRule: false
      }),
      Placeholder.configure({ placeholder: "" })
    ],
    editorProps: {
      attributes: {
        id: "prompt-editor",
        class: "prompt-editor",
        spellcheck: "false"
      },
      handleKeyDown(view, event) {
        if (event.isComposing || event.keyCode === 229 || event.key === "Process") return false;
        const used = promptApi.emitKeyDown(event);
        if (used) return true;
        return false;
      }
    },
    onUpdate({ editor: ed }) {
      syncSendOrb(textOf(ed));
      promptApi.emitChange();
    },
    onFocus() {
      markComposerFocus(true);
      promptApi.emitFocus();
    },
    onBlur() {
      markComposerFocus(false);
      promptApi.emitBlur();
    }
  });

  useEffect(() => {
    if (!editor) return;
    const api = {
      getText: () => textOf(editor),
      setText(v) {
        const next = String(v || "");
        if (textOf(editor) === next) {
          syncSendOrb(next);
          return;
        }
        editor.commands.setContent(next, { emitUpdate: false });
        syncSendOrb(next);
      },
      getCaret: () => offsetFromEditor(editor),
      setCaret(n) {
        try {
          editor.commands.setTextSelection(posFromOffset(editor, Number(n || 0)));
        } catch (e) {
        }
      },
      getSelection() {
        const { from, to } = editor.state.selection;
        return {
          start: editor.state.doc.textBetween(1, from, "\n").length,
          end: editor.state.doc.textBetween(1, to, "\n").length
        };
      },
      setSelection(start, end) {
        try {
          const a = posFromOffset(editor, Number(start || 0));
          const b = posFromOffset(editor, Number(end || 0));
          editor.commands.setTextSelection({ from: a, to: b });
        } catch (e) {
        }
      },
      focus() {
        editor.commands.focus("end");
      },
      isFocused: () => editor.isFocused
    };
    promptApi.bind(api);
    const dom = editor.view && editor.view.dom;
    const onCompStart = () => promptApi.emitCompositionStart();
    const onCompEnd = () => promptApi.emitCompositionEnd();
    if (dom) {
      dom.addEventListener("compositionstart", onCompStart);
      dom.addEventListener("compositionend", onCompEnd);
    }
    syncSendOrb(textOf(editor));
    return () => {
      if (dom) {
        dom.removeEventListener("compositionstart", onCompStart);
        dom.removeEventListener("compositionend", onCompEnd);
      }
      promptApi.unbind(api);
    };
  }, [editor]);

  return <EditorContent editor={editor} />;
}
