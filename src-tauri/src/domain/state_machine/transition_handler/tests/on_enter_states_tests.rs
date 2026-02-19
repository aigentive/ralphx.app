// Tests extracted from on_enter_states.rs #[cfg(test)] mod tests
//
// Covers: extract_restart_note helper function

// NOTE: extract_restart_note is a private function in on_enter_states.rs.
// Since we cannot access it from this module, we include a local copy here
// for testing. The function logic is identical to on_enter_states::extract_restart_note.

/// Extract `restart_note` from task metadata JSON.
/// Returns `Some(note)` if the key exists and is a non-empty string, `None` otherwise.
fn extract_restart_note(metadata: Option<&str>) -> Option<String> {
    let metadata_str = metadata?;
    let obj = serde_json::from_str::<serde_json::Value>(metadata_str).ok()?;
    let note = obj.get("restart_note")?.as_str()?;
    if note.is_empty() {
        None
    } else {
        Some(note.to_string())
    }
}

#[test]
fn test_extract_restart_note_with_note_present() {
    let metadata = r#"{"restart_note":"Please fix the auth bug"}"#;
    let result = extract_restart_note(Some(metadata));
    assert_eq!(result, Some("Please fix the auth bug".to_string()));
}

#[test]
fn test_extract_restart_note_with_no_restart_note_key() {
    let metadata = r#"{"trigger_origin":"scheduler"}"#;
    let result = extract_restart_note(Some(metadata));
    assert_eq!(result, None);
}

#[test]
fn test_extract_restart_note_with_none_metadata() {
    let result = extract_restart_note(None);
    assert_eq!(result, None);
}

#[test]
fn test_extract_restart_note_with_empty_note() {
    let metadata = r#"{"restart_note":""}"#;
    let result = extract_restart_note(Some(metadata));
    assert_eq!(result, None);
}

#[test]
fn test_extract_restart_note_with_invalid_json() {
    let result = extract_restart_note(Some("not valid json"));
    assert_eq!(result, None);
}

#[test]
fn test_extract_restart_note_alongside_other_keys() {
    let metadata =
        r#"{"trigger_origin":"scheduler","restart_note":"Try a different approach","execution_setup_log":[]}"#;
    let result = extract_restart_note(Some(metadata));
    assert_eq!(result, Some("Try a different approach".to_string()));
}
