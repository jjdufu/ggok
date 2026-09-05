use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

const CACHE_OK: Duration = Duration::from_secs(60);
const CACHE_ERR: Duration = Duration::from_secs(15);
const DEFAULT_PROXY: &str = "https://cli-chat-proxy.grok.com/v1";

static CACHE: Mutex<Cache> = Mutex::new(Cache {
    at: None,
    view: None,
});
static REFRESHING: AtomicBool = AtomicBool::new(false);

struct Cache {
    at: Option<Instant>,
    view: Option<AccountView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AccountView {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used_percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining_percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resets_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period_start: Option<String>,
    pub products: Vec<ProductUsage>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProductUsage {
    pub product: String,
    pub used_percent: f64,
}

enum Hit {
    Fresh(AccountView),
    Stale(AccountView),
    Miss,
}

pub async fn snapshot(grok_home: &Path) -> AccountView {
    match cached() {
        Hit::Fresh(v) => v,
        Hit::Stale(v) => {
            spawn_refresh(grok_home.to_path_buf());
            v
        }
        Hit::Miss => load(grok_home).await,
    }
}

pub fn warm(grok_home: PathBuf) {
    tokio::spawn(async move {
        let _ = snapshot(&grok_home).await;
    });
}

fn spawn_refresh(grok_home: PathBuf) {
    if REFRESHING.swap(true, Ordering::SeqCst) {
        return;
    }
    tokio::spawn(async move {
        let _ = load(&grok_home).await;
        REFRESHING.store(false, Ordering::SeqCst);
    });
}

async fn load(grok_home: &Path) -> AccountView {
    match fetch(grok_home).await {
        Ok(view) => {
            store(&view);
            view
        }
        Err(e) => match cached() {
            Hit::Fresh(v) | Hit::Stale(v) if v.ok => v,
            _ => {
                let view = failed_view(e);
                store(&view);
                view
            }
        },
    }
}

fn failed_view(error: String) -> AccountView {
    AccountView {
        ok: false,
        error: Some(error),
        tier: None,
        tier_label: None,
        email: None,
        period: None,
        used_percent: None,
        remaining_percent: None,
        resets_at: None,
        period_start: None,
        products: Vec::new(),
    }
}

fn cached() -> Hit {
    let Ok(g) = CACHE.lock() else {
        return Hit::Miss;
    };
    let Some(view) = g.view.clone() else {
        return Hit::Miss;
    };
    let Some(at) = g.at else {
        return Hit::Stale(view);
    };
    let ttl = if view.ok { CACHE_OK } else { CACHE_ERR };
    if at.elapsed() <= ttl {
        Hit::Fresh(view)
    } else {
        Hit::Stale(view)
    }
}

fn store(view: &AccountView) {
    if let Ok(mut g) = CACHE.lock() {
        g.at = Some(Instant::now());
        g.view = Some(view.clone());
    }
}

async fn fetch(grok_home: &Path) -> Result<AccountView, String> {
    let token = bearer(grok_home).ok_or_else(|| "no grok credentials".to_string())?;
    let base = proxy_base();
    let user_url = format!("{base}/user?include=subscription");
    let bill_url = format!("{base}/billing?format=credits");
    let (user, bill) = tokio::join!(get_json(&user_url, &token), get_json(&bill_url, &token));
    match (user, bill) {
        (Err(e), Err(_)) => Err(e),
        (profile, credits) => {
            let profile = profile.ok();
            let credits = credits.ok();
            Ok(merge(profile.as_ref(), credits.as_ref()))
        }
    }
}

fn merge(profile: Option<&Value>, credits: Option<&Value>) -> AccountView {
    let mut view = AccountView {
        ok: profile.is_some() || credits.is_some(),
        error: None,
        tier: None,
        tier_label: None,
        email: None,
        period: None,
        used_percent: None,
        remaining_percent: None,
        resets_at: None,
        period_start: None,
        products: Vec::new(),
    };
    if let Some(u) = profile {
        if let Some(tier) = str_field(u, "subscriptionTier") {
            view.tier_label = Some(tier_label(&tier));
            view.tier = Some(tier);
        }
        view.email = str_field(u, "email").map(|e| mask_email(&e));
    }
    let cfg = credits.and_then(|b| b.get("config")).or(credits);
    if let Some(cfg) = cfg {
        if let Some(pct) = f64_field(cfg, "creditUsagePercent") {
            let pct = pct.clamp(0.0, 100.0);
            view.used_percent = Some(pct);
            view.remaining_percent = Some((100.0 - pct).clamp(0.0, 100.0));
        }
        if let Some(period) = cfg.get("currentPeriod") {
            view.period = str_field(period, "type").map(|t| period_label(&t));
            view.period_start =
                str_field(period, "start").or_else(|| str_field(cfg, "billingPeriodStart"));
            view.resets_at =
                str_field(period, "end").or_else(|| str_field(cfg, "billingPeriodEnd"));
        } else {
            view.period_start = str_field(cfg, "billingPeriodStart");
            view.resets_at = str_field(cfg, "billingPeriodEnd");
        }
        if let Some(arr) = cfg.get("productUsage").and_then(Value::as_array) {
            for row in arr {
                let product = str_field(row, "product").unwrap_or_else(|| "product".to_string());
                let pct = f64_field(row, "usagePercent")
                    .unwrap_or(0.0)
                    .clamp(0.0, 100.0);
                view.products.push(ProductUsage {
                    product,
                    used_percent: pct,
                });
            }
        }
    }
    if !view.ok {
        view.error = Some("couldn't load usage".to_string());
    }
    view
}

pub fn local_email(grok_home: &Path) -> Option<String> {
    local_email_raw(grok_home).map(|e| mask_email(&e))
}

fn local_email_raw(grok_home: &Path) -> Option<String> {
    let raw = fs::read_to_string(grok_home.join("auth.json")).ok()?;
    let v: Value = serde_json::from_str(&raw).ok()?;
    if let Some(email) = str_field(&v, "email").filter(|s| looks_email(s)) {
        return Some(email);
    }
    let obj = v.as_object()?;
    for (k, cred) in obj {
        if looks_email(k) {
            return Some(k.trim().to_string());
        }
        if let Some(email) = str_field(cred, "email").filter(|s| looks_email(s)) {
            return Some(email);
        }
    }
    None
}

#[must_use]
pub fn mask_email(raw: &str) -> String {
    let s = raw.trim();
    let Some((user, host)) = s.split_once('@') else {
        return "***".to_string();
    };
    if host.is_empty() {
        return "***".to_string();
    }
    let mut chars = user.chars();
    let masked_user = match (chars.next(), chars.next()) {
        (Some(first), Some(_)) => format!("{first}***"),
        _ => "*".to_string(),
    };
    format!("{masked_user}@{host}")
}

fn looks_email(s: &str) -> bool {
    let s = s.trim();
    let Some((user, host)) = s.split_once('@') else {
        return false;
    };
    !user.is_empty()
        && host.contains('.')
        && !host.starts_with('.')
        && !host.ends_with('.')
        && !s.chars().any(char::is_whitespace)
}

fn bearer(grok_home: &Path) -> Option<String> {
    if let Ok(key) = std::env::var("XAI_API_KEY") {
        let key = key.trim().to_string();
        if !key.is_empty() {
            return Some(key);
        }
    }
    let raw = fs::read_to_string(grok_home.join("auth.json")).ok()?;
    let v: Value = serde_json::from_str(&raw).ok()?;
    let obj = v.as_object()?;
    let mut fallback: Option<String> = None;
    for cred in obj.values() {
        let Some(key) = cred.get("key").and_then(Value::as_str) else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        if !expired(cred.get("expires_at").and_then(Value::as_str)) {
            return Some(key.to_string());
        }
        fallback = Some(key.to_string());
    }
    fallback
}

fn expired(raw: Option<&str>) -> bool {
    let Some(raw) = raw.filter(|s| !s.is_empty()) else {
        return false;
    };
    chrono::DateTime::parse_from_rfc3339(raw)
        .is_ok_and(|t| t.timestamp() <= chrono::Utc::now().timestamp())
}

fn proxy_base() -> String {
    let raw = std::env::var("GROK_CLI_CHAT_PROXY_BASE_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_PROXY.to_string());
    raw.trim().trim_end_matches('/').to_string()
}

async fn get_json(url: &str, token: &str) -> Result<Value, String> {
    let mut child = Command::new(curl_bin())
        .args([
            "-sS",
            "-f",
            "--max-time",
            "8",
            "--connect-timeout",
            "4",
            "-K",
            "-",
            "--url",
            url,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("curl: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        let cfg = curl_header_config(token);
        let _ = stdin.write_all(cfg.as_bytes()).await;
        let _ = stdin.shutdown().await;
    }
    let out = child
        .wait_with_output()
        .await
        .map_err(|e| format!("curl: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let err = err.trim();
        if err.contains("401") || err.contains("403") {
            return Err("auth expired".to_string());
        }
        if err.is_empty() {
            return Err("couldn't load usage".to_string());
        }
        return Err(truncate(err, 160));
    }
    let body = String::from_utf8(out.stdout).map_err(|_| "usage response is not utf-8")?;
    serde_json::from_str(&body).map_err(|_| "couldn't parse usage".to_string())
}

fn curl_header_config(token: &str) -> String {
    format!(
        "header = \"Authorization: Bearer {}\"\nheader = \"Accept: application/json\"\nheader = \"X-XAI-Token-Auth: xai-grok-cli\"\n",
        escape_curl_dq(token)
    )
}

fn escape_curl_dq(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' | '"' => {
                out.push('\\');
                out.push(c);
            }
            '\n' | '\r' => {}
            _ => out.push(c),
        }
    }
    out
}

fn curl_bin() -> PathBuf {
    let p = PathBuf::from("/usr/bin/curl");
    if p.is_file() {
        p
    } else {
        PathBuf::from("curl")
    }
}

fn str_field(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
}

fn f64_field(v: &Value, key: &str) -> Option<f64> {
    match v.get(key)? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

fn period_label(raw: &str) -> String {
    let s = raw.to_ascii_uppercase();
    if s.contains("WEEK") {
        "weekly".to_string()
    } else if s.contains("MONTH") {
        "monthly".to_string()
    } else if s.contains("DAY") {
        "daily".to_string()
    } else {
        raw.to_ascii_lowercase()
    }
}

fn tier_label(raw: &str) -> String {
    let key: String = raw
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect();
    match key.as_str() {
        "grokpro" | "supergrok" => "SuperGrok".to_string(),
        "groklite" | "supergroklite" => "SuperGrok Lite".to_string(),
        "grokplus" | "supergrokplus" => "SuperGrok Plus".to_string(),
        "grokheavy" | "supergrokheavy" => "SuperGrok Heavy".to_string(),
        "xpremium" => "X Premium".to_string(),
        "xpremiumplus" => "X Premium+".to_string(),
        "xbasic" => "X Basic".to_string(),
        "free" => "Free".to_string(),
        _ => raw.to_string(),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max).collect();
    out.push('…');
    out
}
