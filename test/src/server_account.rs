use ggok_server::mask_email;

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
