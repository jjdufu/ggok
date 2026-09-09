use ggok_server::{mask_email, merge_account};
use serde_json::json;

#[test]
fn mask_email_keeps_host_and_first_char() {
    assert_eq!(mask_email("ab@example.com"), "a***@example.com");
    assert_eq!(mask_email("alice@x.ai"), "a***@x.ai");
    assert_eq!(mask_email("a@x.ai"), "*@x.ai");
    assert_eq!(mask_email("  bob@host  "), "b***@host");
}

#[test]
fn mask_email_invalid_is_stars() {
    assert_eq!(mask_email("no-at"), "***");
    assert_eq!(mask_email("user@"), "***");
    assert_eq!(mask_email(""), "***");
}

#[test]
fn merge_account_reset_without_usage_is_zero() {
    let profile = json!({
        "subscriptionTier": "GrokPro",
        "email": "jane@outlook.com"
    });
    let credits = json!({
        "config": {
            "currentPeriod": {
                "type": "WEEK",
                "start": "2026-09-09T12:19:03.619589+00:00",
                "end": "2026-09-16T12:19:03.619589+00:00"
            },
            "productUsage": []
        }
    });
    let view = merge_account(Some(&profile), Some(&credits));
    assert!(view.ok);
    assert_eq!(view.used_percent, Some(0.0));
    assert_eq!(view.remaining_percent, Some(100.0));
    assert!(view.products.is_empty());
    assert_eq!(view.period.as_deref(), Some("weekly"));
    assert_eq!(
        view.resets_at.as_deref(),
        Some("2026-09-16T12:19:03.619589+00:00")
    );
    assert_eq!(view.tier_label.as_deref(), Some("SuperGrok"));
}

#[test]
fn merge_account_keeps_explicit_usage_percent() {
    let credits = json!({
        "creditUsagePercent": 99.0,
        "currentPeriod": {
            "type": "WEEK",
            "start": "2026-09-02T12:19:03Z",
            "end": "2026-09-09T12:19:03Z"
        },
        "productUsage": [{ "product": "GrokBuild", "usagePercent": 99.0 }]
    });
    let view = merge_account(None, Some(&credits));
    assert!(view.ok);
    assert_eq!(view.used_percent, Some(99.0));
    assert_eq!(view.remaining_percent, Some(1.0));
    assert_eq!(view.products.len(), 1);
    assert_eq!(view.products[0].product, "GrokBuild");
}

#[test]
fn merge_account_profile_only_does_not_invent_zero_usage() {
    let profile = json!({
        "subscriptionTier": "GrokPro",
        "email": "jane@outlook.com"
    });
    let view = merge_account(Some(&profile), None);
    assert!(view.ok);
    assert_eq!(view.used_percent, None);
    assert_eq!(view.remaining_percent, None);
    assert!(view.products.is_empty());
}
