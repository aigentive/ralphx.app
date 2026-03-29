// Review domain module
// Configuration and logic for AI and human code review

pub mod config;
pub mod complete_review;
pub mod complete_support;
pub mod review_points;
pub mod scope_drift;

pub use config::ReviewSettings;
pub use complete_review::{
    CompleteReviewInput, CompleteReviewValidationError, ParseReviewToolOutcomeError,
    ReviewIssueInput, ReviewIssueValidationError, ReviewToolOutcome,
};
pub use complete_support::{
    build_unrelated_drift_followup_prompt, parse_review_issue, parse_review_issues,
    ParsedReviewIssue, RawReviewIssueInput,
};
pub use review_points::{
    get_review_point_type, is_complex_task, is_destructive_task, should_auto_insert_review_point,
    ReviewPointConfig, ReviewPointType,
};
pub use scope_drift::{
    compute_out_of_scope_blocker_fingerprint, compute_scope_drift, matches_planned_scope,
    normalize_scope_path, ParseScopeDriftClassificationError, ScopeDriftClassification,
};
