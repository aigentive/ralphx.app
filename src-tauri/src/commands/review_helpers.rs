// Shared helpers for review-related functionality
// Used by both Tauri commands and HTTP handlers

use super::review_commands_types::ReviewIssue;

/// Parse issues from notes field if present
/// Format stored by complete_review: {"issues":[...]}\n<feedback_text>
/// Returns (parsed_issues, clean_notes_without_json)
pub fn parse_issues_from_notes(
    notes: &Option<String>,
) -> (Option<Vec<ReviewIssue>>, Option<String>) {
    let Some(notes_text) = notes else {
        return (None, None);
    };

    // Check if notes starts with {"issues": pattern
    if !notes_text.starts_with("{\"issues\":") {
        return (None, Some(notes_text.clone()));
    }

    // Find the end of the JSON line (first newline)
    if let Some(newline_pos) = notes_text.find('\n') {
        let json_part = &notes_text[..newline_pos];
        let feedback_part = notes_text[newline_pos + 1..].to_string();

        // Parse the issues wrapper: {"issues":[...]}
        #[derive(serde::Deserialize)]
        struct IssuesWrapper {
            issues: Vec<IssueJson>,
        }

        // Intermediate type for JSON deserialization
        #[derive(serde::Deserialize)]
        struct IssueJson {
            severity: String,
            file: Option<String>,
            line: Option<i32>,
            description: String,
        }

        match serde_json::from_str::<IssuesWrapper>(json_part) {
            Ok(wrapper) => {
                let issues = if wrapper.issues.is_empty() {
                    None
                } else {
                    Some(
                        wrapper
                            .issues
                            .into_iter()
                            .map(|i| ReviewIssue {
                                severity: i.severity,
                                file: i.file,
                                line: i.line,
                                description: i.description,
                            })
                            .collect(),
                    )
                };
                let clean_notes = if feedback_part.is_empty() {
                    None
                } else {
                    Some(feedback_part)
                };
                (issues, clean_notes)
            }
            Err(_) => {
                // Failed to parse, return original notes
                (None, Some(notes_text.clone()))
            }
        }
    } else {
        // No newline, entire notes is JSON (no feedback text)
        #[derive(serde::Deserialize)]
        struct IssuesWrapper {
            issues: Vec<IssueJson>,
        }

        #[derive(serde::Deserialize)]
        struct IssueJson {
            severity: String,
            file: Option<String>,
            line: Option<i32>,
            description: String,
        }

        match serde_json::from_str::<IssuesWrapper>(notes_text) {
            Ok(wrapper) => {
                let issues = if wrapper.issues.is_empty() {
                    None
                } else {
                    Some(
                        wrapper
                            .issues
                            .into_iter()
                            .map(|i| ReviewIssue {
                                severity: i.severity,
                                file: i.file,
                                line: i.line,
                                description: i.description,
                            })
                            .collect(),
                    )
                };
                (issues, None)
            }
            Err(_) => (None, Some(notes_text.clone())),
        }
    }
}

#[cfg(test)]
mod tests {
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
        let input = Some(
            r#"{"issues":[{"severity":"minor","description":"Naming issue"}]}"#.to_string(),
        );
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
}
