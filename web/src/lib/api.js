const t = (key, vars) => (window.I18n && window.I18n.t ? window.I18n.t(key, vars) : key);

let apiEpoch = 0;

export function discardInflight() {
  apiEpoch += 1;
}

function staleError() {
  const err = new Error("stale");
  err.stale = true;
  return err;
}

export async function api(path, opts) {
  const mine = apiEpoch;
  const res = await fetch(path, Object.assign({ credentials: "same-origin" }, opts || {}));
  if (mine !== apiEpoch) throw staleError();
  if (res.status === 401) {
    location.href = "/login";
    throw new Error("unauthorized");
  }
  if (!res.ok) {
    const body = await res.text();
    if (res.status === 409 && body.includes("session_busy")) {
      const err = new Error(t("sessionBusy"));
      err.code = "session_busy";
      throw err;
    }
    throw new Error(body || path + " " + res.status);
  }
  const ct = res.headers.get("content-type") || "";
  if (ct.includes("json")) return res.json();
  return null;
}

export function post(path, body) {
  return api(path, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body)
  });
}

export function patch(path, body) {
  return api(path, {
    method: "PATCH",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body)
  });
}

export function del(path) {
  return api(path, { method: "DELETE" });
}
