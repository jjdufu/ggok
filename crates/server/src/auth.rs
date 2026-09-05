use cookie::{Cookie, SameSite, time::Duration};
use hex::{decode as hex_decode, encode as hex_encode};
use hmac::{Hmac, Mac};
use http::header::SET_COOKIE;
use http::{HeaderValue, StatusCode};
use sha2::Sha256;
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration as StdDuration, Instant, SystemTime, UNIX_EPOCH};
use subtle::ConstantTimeEq;

const COOKIE_TTL_SECS: i64 = 7 * 24 * 60 * 60;
const FAIL_WINDOW: StdDuration = StdDuration::from_secs(15 * 60);
const FAIL_LIMIT: usize = 5;
const MAX_TRACKED_IPS: usize = 2048;

type HmacSha256 = Hmac<Sha256>;

#[must_use]
pub fn constant_time_eq(a: &str, b: &str) -> bool {
    let aa = a.as_bytes();
    let bb = b.as_bytes();
    if aa.len() != bb.len() {
        let dummy = vec![0_u8; aa.len()];
        let _ = aa.ct_eq(&dummy);
        return false;
    }
    bool::from(aa.ct_eq(bb))
}

#[must_use]
pub fn sign_cookie(key: &[u8; 32], now_unix: i64) -> String {
    let exp = now_unix + COOKIE_TTL_SECS;
    let payload = exp.to_string();
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts 32-byte key");
    mac.update(payload.as_bytes());
    let sig = mac.finalize().into_bytes();
    format!("{payload}.{}", hex_encode(sig))
}

#[must_use]
pub fn verify_cookie(key: &[u8; 32], value: &str, now_unix: i64) -> bool {
    let Some((exp_s, sig_hex)) = value.split_once('.') else {
        return false;
    };
    let Ok(exp) = exp_s.parse::<i64>() else {
        return false;
    };
    if exp < now_unix {
        return false;
    }
    let Ok(sig) = hex_decode(sig_hex) else {
        return false;
    };
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts 32-byte key");
    mac.update(exp_s.as_bytes());
    mac.verify_slice(&sig).is_ok()
}

#[must_use]
pub fn now_unix() -> i64 {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    )
    .unwrap_or(0)
}

#[must_use]
pub fn port_of_bind(bind: &str) -> Option<u16> {
    let hostport = bind.trim();
    if hostport.is_empty() {
        return None;
    }
    if let Some((_, rest)) = hostport.rsplit_once(']') {
        return rest.trim_start_matches(':').parse().ok();
    }
    hostport.rsplit_once(':')?.1.parse().ok()
}

#[must_use]
pub fn cookie_name_for_bind(bind: &str) -> String {
    match port_of_bind(bind) {
        Some(port) if port > 0 => format!("ggok_{port}"),
        _ => "ggok".to_string(),
    }
}

#[must_use]
pub fn build_session_cookie(name: &str, value: &str) -> Cookie<'static> {
    let mut cookie = Cookie::new(name.to_string(), value.to_string());
    cookie.set_http_only(true);
    cookie.set_same_site(SameSite::Lax);
    cookie.set_path("/");
    cookie.set_max_age(Duration::seconds(COOKIE_TTL_SECS));
    cookie
}

#[must_use]
pub fn clear_session_cookie(name: &str) -> Cookie<'static> {
    let mut cookie = Cookie::new(name.to_string(), String::new());
    cookie.set_http_only(true);
    cookie.set_same_site(SameSite::Lax);
    cookie.set_path("/");
    cookie.set_max_age(Duration::ZERO);
    cookie
}

#[must_use]
pub fn cookie_from_header(header: Option<&str>, name: &str) -> Option<String> {
    let header = header?;
    let prefix = format!("{name}=");
    for part in header.split(';') {
        let part = part.trim();
        if let Some(value) = part.strip_prefix(&prefix) {
            return Some(value.to_string());
        }
    }
    None
}

#[must_use]
pub fn bearer_token(header: Option<&str>) -> Option<&str> {
    header.and_then(|h| {
        h.strip_prefix("Bearer ")
            .or_else(|| h.strip_prefix("bearer "))
    })
}

pub fn append_set_cookie(headers: &mut http::HeaderMap, cookie: &Cookie<'_>) {
    if let Ok(value) = HeaderValue::from_str(&cookie.to_string()) {
        headers.append(SET_COOKIE, value);
    }
}

#[derive(Default)]
pub struct LoginLimiter {
    fails: HashMap<IpAddr, Vec<Instant>>,
}

impl LoginLimiter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn blocked(&mut self, ip: IpAddr, now: Instant) -> bool {
        self.sweep(now);
        self.fails.get(&ip).is_some_and(|v| v.len() >= FAIL_LIMIT)
    }

    pub fn record_fail(&mut self, ip: IpAddr, now: Instant) {
        self.sweep(now);
        if !self.fails.contains_key(&ip) && self.fails.len() >= MAX_TRACKED_IPS {
            self.evict_one();
        }
        self.fails.entry(ip).or_default().push(now);
    }

    pub fn clear(&mut self, ip: IpAddr) {
        self.fails.remove(&ip);
    }

    fn sweep(&mut self, now: Instant) {
        self.fails.retain(|_, v| {
            v.retain(|t| now.saturating_duration_since(*t) < FAIL_WINDOW);
            !v.is_empty()
        });
    }

    fn evict_one(&mut self) {
        let last_at = |v: &[Instant]| v.last().copied();
        let open = self
            .fails
            .iter()
            .filter(|(_, v)| v.len() < FAIL_LIMIT)
            .min_by_key(|(_, v)| last_at(v))
            .map(|(ip, _)| *ip);
        let ip = open.or_else(|| {
            self.fails
                .iter()
                .min_by_key(|(_, v)| last_at(v))
                .map(|(ip, _)| *ip)
        });
        if let Some(ip) = ip {
            self.fails.remove(&ip);
        }
    }
}

#[must_use]
pub fn unauthorized() -> (StatusCode, &'static str) {
    (StatusCode::UNAUTHORIZED, "unauthorized")
}

#[must_use]
pub fn too_many() -> (StatusCode, &'static str) {
    (StatusCode::TOO_MANY_REQUESTS, "too many login attempts")
}
