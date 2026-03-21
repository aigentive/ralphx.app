// Unit tests for is_merge_worktree_path() in merge_helpers.rs.
//
// Verifies prefix detection for all temporary merge-pipeline worktree basename
// prefixes: merge-, rebase-, source-update-, plan-update-.
//
// The function extracts the basename (file_name) from the path before checking,
// so full paths must use their final component for classification.

use crate::domain::state_machine::transition_handler::merge_helpers::is_merge_worktree_path;

// ──────────────────────────────────────────────────────────────────────────────
// True cases — each recognized prefix
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn merge_prefix_bare_name_returns_true() {
    assert!(
        is_merge_worktree_path("merge-abc123"),
        "bare 'merge-abc123' should be detected as a merge worktree path"
    );
}

#[test]
fn merge_prefix_full_path_returns_true() {
    assert!(
        is_merge_worktree_path("/path/to/merge-task-id"),
        "full path ending in 'merge-task-id' should be detected"
    );
}

#[test]
fn rebase_prefix_bare_name_returns_true() {
    assert!(
        is_merge_worktree_path("rebase-abc123"),
        "bare 'rebase-abc123' should be detected as a merge worktree path"
    );
}

#[test]
fn rebase_prefix_full_path_returns_true() {
    assert!(
        is_merge_worktree_path("/worktrees/rebase-task-id"),
        "full path ending in 'rebase-task-id' should be detected"
    );
}

#[test]
fn source_update_prefix_bare_name_returns_true() {
    assert!(
        is_merge_worktree_path("source-update-abc123"),
        "bare 'source-update-abc123' should be detected"
    );
}

#[test]
fn plan_update_prefix_bare_name_returns_true() {
    assert!(
        is_merge_worktree_path("plan-update-abc123"),
        "bare 'plan-update-abc123' should be detected"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// False cases — task worktrees and unrelated names
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn task_prefix_bare_name_returns_false() {
    assert!(
        !is_merge_worktree_path("task-abc123"),
        "'task-abc123' must NOT be detected as a merge worktree path"
    );
}

#[test]
fn empty_string_returns_false() {
    assert!(
        !is_merge_worktree_path(""),
        "empty string must return false"
    );
}

#[test]
fn task_prefix_full_path_returns_false() {
    assert!(
        !is_merge_worktree_path("/path/to/task-abc123"),
        "full path ending in 'task-abc123' must return false"
    );
}

#[test]
fn root_path_only_returns_false() {
    // "/" has no file_name component; basename falls back to ""
    assert!(
        !is_merge_worktree_path("/"),
        "'/' has no basename and must return false"
    );
}

#[test]
fn infix_merge_word_returns_false() {
    // "somemerge-abc" does NOT start with "merge-" after basename extraction
    assert!(
        !is_merge_worktree_path("somemerge-abc"),
        "'somemerge-abc' does not start with 'merge-' and must return false"
    );
}

#[test]
fn merge_no_hyphen_returns_false() {
    // "merge" alone has no hyphen — does not match "merge-"
    assert!(
        !is_merge_worktree_path("merge"),
        "'merge' without a hyphen must return false"
    );
}
