use super::*;

#[test]
fn test_commit_info_response_conversion() {
    let info = CommitInfo {
        sha: "abcdef1234567890abcdef1234567890abcdef12".to_string(),
        short_sha: "abcdef1".to_string(),
        message: "Test commit".to_string(),
        author: "Test Author".to_string(),
        timestamp: "2026-02-02T12:00:00+00:00".to_string(),
    };

    let response = CommitInfoResponse::from(info);
    assert_eq!(response.short_sha, "abcdef1");
    assert_eq!(response.message, "Test commit");
}

#[test]
fn test_diff_stats_response_conversion() {
    let stats = DiffStats {
        files_changed: 5,
        insertions: 100,
        deletions: 50,
        changed_files: vec!["src/foo.rs".to_string(), "src/bar.rs".to_string()],
    };

    let response = TaskDiffStatsResponse::from(stats);
    assert_eq!(response.files_changed, 5);
    assert_eq!(response.insertions, 100);
    assert_eq!(response.deletions, 50);
    assert_eq!(response.changed_files.len(), 2);
}
