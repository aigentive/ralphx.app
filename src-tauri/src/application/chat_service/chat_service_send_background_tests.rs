use super::session_changed_after_resume;

#[test]
fn session_changed_returns_true_when_ids_differ() {
    assert!(session_changed_after_resume(
        Some("session-old-abc"),
        Some("session-new-xyz"),
    ));
}

#[test]
fn session_changed_returns_false_when_ids_match() {
    assert!(!session_changed_after_resume(
        Some("session-abc"),
        Some("session-abc"),
    ));
}

#[test]
fn session_changed_returns_false_when_no_stored_id() {
    // --resume was not used; no comparison possible
    assert!(!session_changed_after_resume(None, Some("session-new")));
}

#[test]
fn session_changed_returns_false_when_no_new_id() {
    // Stream returned no session ID; cannot detect change
    assert!(!session_changed_after_resume(Some("session-old"), None));
}

#[test]
fn session_changed_returns_false_when_both_none() {
    assert!(!session_changed_after_resume(None, None));
}
