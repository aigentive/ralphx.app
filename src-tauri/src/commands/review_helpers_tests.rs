use super::*;

#[test]
fn test_parse_issues_from_notes_none() {
    let (issues, notes) = parse_issues_from_notes(&None);
    assert!(issues.is_none());
    assert!(notes.is_none());
}

#[test]
fn test_parse_issues_from_notes_no_json() {
    let input = Some("Just regular feedback text".to_string());
    let (issues, notes) = parse_issues_from_notes(&input);
    assert!(issues.is_none());
    assert_eq!(notes, Some("Just regular feedback text".to_string()));
}

#[test]
fn test_parse_issues_from_notes_with_issues_and_feedback() {
    let input = Some(
        r#"{"issues":[{"severity":"critical","file":"src/main.rs","line":42,"description":"Memory leak"}]}
This is the feedback text."#
            .to_string(),
    );
    let (issues, notes) = parse_issues_from_notes(&input);

    assert!(issues.is_some());
    let issues = issues.unwrap();
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].severity, "critical");
    assert_eq!(issues[0].file, Some("src/main.rs".to_string()));
    assert_eq!(issues[0].line, Some(42));
    assert_eq!(issues[0].description, "Memory leak");

    assert_eq!(notes, Some("This is the feedback text.".to_string()));
}

#[test]
fn test_parse_issues_from_notes_only_json() {
    let input =
        Some(r#"{"issues":[{"severity":"minor","description":"Naming issue"}]}"#.to_string());
    let (issues, notes) = parse_issues_from_notes(&input);

    assert!(issues.is_some());
    let issues = issues.unwrap();
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].severity, "minor");
    assert!(issues[0].file.is_none());
    assert!(issues[0].line.is_none());

    assert!(notes.is_none());
}

#[test]
fn test_parse_issues_from_notes_empty_issues() {
    let input = Some(r#"{"issues":[]}"#.to_string());
    let (issues, notes) = parse_issues_from_notes(&input);

    assert!(issues.is_none()); // Empty vec becomes None
    assert!(notes.is_none());
}
