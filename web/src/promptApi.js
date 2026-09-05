const changeFns = new Set();
const focusFns = new Set();
const blurFns = new Set();
const keyFns = new Set();
const composeStartFns = new Set();
const composeEndFns = new Set();

let impl = null;
let pendingText = null;
let pendingCaret = null;

function flushPending() {
  if (!impl) return;
  if (pendingText != null) {
    impl.setText(pendingText);
    pendingText = null;
  }
  if (pendingCaret != null) {
    impl.setCaret(pendingCaret);
    pendingCaret = null;
  }
}

export const promptApi = {
  bind(next) {
    impl = next;
    flushPending();
  },
  unbind(current) {
    if (impl === current) impl = null;
  },
  getText() {
    return impl ? impl.getText() : pendingText || "";
  },
  setText(text) {
    const v = String(text ?? "");
    if (!impl) {
      pendingText = v;
      return;
    }
    impl.setText(v);
  },
  getCaret() {
    return impl ? impl.getCaret() : pendingCaret || 0;
  },
  setCaret(n) {
    const pos = Math.max(0, Number(n) || 0);
    if (!impl) {
      pendingCaret = pos;
      return;
    }
    impl.setCaret(pos);
  },
  getSelection() {
    if (impl && impl.getSelection) return impl.getSelection();
    const c = this.getCaret();
    return { start: c, end: c };
  },
  setSelection(start, end) {
    if (impl && impl.setSelection) impl.setSelection(start, end);
    else this.setCaret(end);
  },
  focus() {
    if (impl) impl.focus();
  },
  isFocused() {
    return impl ? impl.isFocused() : false;
  },
  onChange(fn) {
    changeFns.add(fn);
    return () => changeFns.delete(fn);
  },
  onFocus(fn) {
    focusFns.add(fn);
    return () => focusFns.delete(fn);
  },
  onBlur(fn) {
    blurFns.add(fn);
    return () => blurFns.delete(fn);
  },
  onKeyDown(fn) {
    keyFns.add(fn);
    return () => keyFns.delete(fn);
  },
  onCompositionStart(fn) {
    composeStartFns.add(fn);
    return () => composeStartFns.delete(fn);
  },
  onCompositionEnd(fn) {
    composeEndFns.add(fn);
    return () => composeEndFns.delete(fn);
  },
  emitChange() {
    for (const fn of changeFns) fn();
  },
  emitFocus() {
    for (const fn of focusFns) fn();
  },
  emitBlur() {
    for (const fn of blurFns) fn();
  },
  emitKeyDown(event) {
    let used = false;
    const wrapped = {
      key: event.key,
      code: event.code,
      keyCode: event.keyCode,
      which: event.which,
      shiftKey: event.shiftKey,
      altKey: event.altKey,
      metaKey: event.metaKey,
      ctrlKey: event.ctrlKey,
      isComposing: event.isComposing,
      preventDefault() {
        used = true;
        if (event.preventDefault) event.preventDefault();
      },
      stopPropagation() {
        if (event.stopPropagation) event.stopPropagation();
      }
    };
    for (const fn of keyFns) fn(wrapped);
    return used;
  },
  emitCompositionStart() {
    for (const fn of composeStartFns) fn();
  },
  emitCompositionEnd() {
    for (const fn of composeEndFns) fn();
  }
};
