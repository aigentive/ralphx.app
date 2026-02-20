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
#[path = "review_helpers_tests.rs"]
mod tests;
