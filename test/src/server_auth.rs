use ggok_server::{
    LoginLimiter, bearer_token, cookie_from_header, cookie_name_for_bind, constant_time_eq,
    port_of_bind, sign_cookie, verify_cookie,
};
use std::net::IpAddr;
use std::time::{Duration, Instant};

#[test]
fn constant_time_eq_compares_bytes() {
    assert!(constant_time_eq("token", "token"));
    assert!(!constant_time_eq("token", "Token"));
    assert!(!constant_time_eq("abc", "ab"));
    assert!(!constant_time_eq("", "x"));
}

#[test]
fn sign_and_verify_cookie_roundtrip() {
    let key = [7_u8; 32];
    let now = 1_700_000_000;
    let value = sign_cookie(&key, now);
    assert!(verify_cookie(&key, &value, now));
    assert!(verify_cookie(&key, &value, now + 60));
    assert!(!verify_cookie(&key, &value, now + 8 * 24 * 60 * 60));
    assert!(!verify_cookie(&[8_u8; 32], &value, now));
    assert!(!verify_cookie(&key, "no-dot", now));
    assert!(!verify_cookie(&key, "notanumber.abcd", now));
}

#[test]
fn port_and_cookie_name() {
    assert_eq!(port_of_bind("0.0.0.0:9888"), Some(9888));
    assert_eq!(port_of_bind("[::1]:443"), Some(443));
    assert_eq!(port_of_bind(""), None);
    assert_eq!(port_of_bind("no-port"), None);
    assert_eq!(cookie_name_for_bind("0.0.0.0:9888"), "ggok_9888");
    assert_eq!(cookie_name_for_bind("bad"), "ggok");
}

#[test]
fn cookie_header_and_bearer() {
    assert_eq!(
        cookie_from_header(Some("a=1; ggok_9888=secret; b=2"), "ggok_9888").as_deref(),
        Some("secret")
    );
    assert!(cookie_from_header(Some("a=1"), "ggok").is_none());
    assert!(cookie_from_header(None, "ggok").is_none());
    assert_eq!(bearer_token(Some("Bearer abc")), Some("abc"));
    assert_eq!(bearer_token(Some("bearer xyz")), Some("xyz"));
    assert!(bearer_token(Some("Token abc")).is_none());
    assert!(bearer_token(None).is_none());
}

#[test]
fn login_limiter_blocks_after_five_fails() {
    let mut lim = LoginLimiter::new();
    let ip: IpAddr = "127.0.0.1".parse().expect("ip");
    let t0 = Instant::now();
    for i in 0..5 {
        assert!(!lim.blocked(ip, t0 + Duration::from_secs(i)));
        lim.record_fail(ip, t0 + Duration::from_secs(i));
    }
    assert!(lim.blocked(ip, t0 + Duration::from_secs(5)));
    lim.clear(ip);
    assert!(!lim.blocked(ip, t0 + Duration::from_secs(6)));
}
