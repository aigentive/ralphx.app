use super::chat_service::has_meaningful_output;

#[test]
fn response_text_with_stderr_is_still_meaningful_output() {
    assert!(
        has_meaningful_output("I can help with that.", 0, "warning: stderr noise"),
        "successful providers may write warnings to stderr while still returning assistant text"
    );
}

#[test]
fn stderr_without_response_text_is_not_meaningful_output() {
    assert!(!has_meaningful_output("", 0, "fatal: provider exited"));
}

#[test]
fn provider_error_text_is_not_meaningful_output() {
    assert!(!has_meaningful_output(
        "You've hit your limit. Please try again later.",
        0,
        ""
    ));
}
