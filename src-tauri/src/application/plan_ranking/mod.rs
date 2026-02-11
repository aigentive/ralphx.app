//! Plan ranking algorithm for prioritizing ideation sessions
//!
//! Computes weighted scores combining interaction, activity, and recency to
//! determine which plans should appear first in the quick switcher.

use chrono::{DateTime, Utc};

/// Breakdown of ranking score components for debugging
#[derive(Debug, Clone, PartialEq)]
pub struct ScoreBreakdown {
    pub interaction_score: f64,
    pub activity_score: f64,
    pub recency_score: f64,
    pub final_score: f64,
}

/// Computes interaction score based on selection frequency and recency
///
/// Formula: frequency_score * recency_decay
/// - Frequency: log scale, normalized to [0, 1] with 10 selections = 1.0
/// - Recency: exponential decay with 21-day half-life
///
/// # Arguments
/// * `selected_count` - Number of times this plan has been selected
/// * `last_selected_at` - Most recent selection timestamp
/// * `now` - Current timestamp for calculating decay
pub fn compute_interaction_score(
    selected_count: u32,
    last_selected_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> f64 {
    // Frequency component: logarithmic scale
    // ln(1) = 0, ln(10) ≈ 2.3
    // Normalized so 10 selections = score of 1.0
    let frequency_score = ((selected_count as f64 + 1.0).ln() / 10_f64.ln()).min(1.0);

    // Recency component: exponential decay
    // 21-day half-life means e^(-21/21) = e^(-1) ≈ 0.37
    let recency_decay = match last_selected_at {
        Some(timestamp) => {
            let days_since = (now - timestamp).num_days() as f64;
            (-days_since / 21.0).exp()
        }
        None => 0.0,
    };

    frequency_score * recency_decay
}

/// Computes activity score based on active tasks and completion ratio
///
/// Formula: 0.6 * active_bonus + 0.4 * incomplete_ratio
/// - Active bonus: 1.0 if any tasks in executing/review/merge states, else 0.0
/// - Incomplete ratio: proportion of tasks not yet completed
///
/// # Arguments
/// * `active_now` - Count of tasks in active execution states
/// * `incomplete` - Count of incomplete tasks
/// * `total` - Total task count
pub fn compute_activity_score(active_now: u32, incomplete: u32, total: u32) -> f64 {
    let active_now_bonus = if active_now > 0 { 1.0 } else { 0.0 };
    let incomplete_ratio = if total > 0 {
        incomplete as f64 / total as f64
    } else {
        0.0
    };

    0.6 * active_now_bonus + 0.4 * incomplete_ratio
}

/// Computes recency score based on plan acceptance date
///
/// Formula: exponential decay with 30-day half-life
/// - Recently accepted plans score higher
/// - Old plans gradually decay toward 0
///
/// # Arguments
/// * `accepted_at` - When the plan was accepted (converted_at timestamp)
/// * `now` - Current timestamp for calculating decay
pub fn compute_recency_score(accepted_at: DateTime<Utc>, now: DateTime<Utc>) -> f64 {
    let days_since = (now - accepted_at).num_days() as f64;
    (-days_since / 30.0).exp()
}

/// Computes final weighted score combining all three components
///
/// Weights: 45% interaction + 35% activity + 20% recency
///
/// # Arguments
/// * `selected_count` - Number of times this plan has been selected
/// * `last_selected_at` - Most recent selection timestamp
/// * `active_now` - Count of tasks in active execution states
/// * `incomplete` - Count of incomplete tasks
/// * `total` - Total task count
/// * `accepted_at` - When the plan was accepted
/// * `now` - Current timestamp
pub fn compute_final_score(
    selected_count: u32,
    last_selected_at: Option<DateTime<Utc>>,
    active_now: u32,
    incomplete: u32,
    total: u32,
    accepted_at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> f64 {
    let interaction = compute_interaction_score(selected_count, last_selected_at, now);
    let activity = compute_activity_score(active_now, incomplete, total);
    let recency = compute_recency_score(accepted_at, now);

    0.45 * interaction + 0.35 * activity + 0.20 * recency
}

/// Computes final score with breakdown for debugging
pub fn compute_final_score_with_breakdown(
    selected_count: u32,
    last_selected_at: Option<DateTime<Utc>>,
    active_now: u32,
    incomplete: u32,
    total: u32,
    accepted_at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> ScoreBreakdown {
    let interaction_score = compute_interaction_score(selected_count, last_selected_at, now);
    let activity_score = compute_activity_score(active_now, incomplete, total);
    let recency_score = compute_recency_score(accepted_at, now);
    let final_score = 0.45 * interaction_score + 0.35 * activity_score + 0.20 * recency_score;

    ScoreBreakdown {
        interaction_score,
        activity_score,
        recency_score,
        final_score,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    // Fixed test timestamp: 2026-02-01 12:00:00 UTC
    fn test_now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 2, 1, 12, 0, 0).unwrap()
    }

    #[test]
    fn test_interaction_score_no_selections() {
        let now = test_now();
        let score = compute_interaction_score(0, None, now);
        assert_eq!(score, 0.0, "No selections should give zero score");
    }

    #[test]
    fn test_interaction_score_with_frequency() {
        let now = test_now();
        let last_selected = now - chrono::Duration::days(1);

        // 1 selection (ln(2) / ln(10) ≈ 0.301) * exp(-1/21) ≈ 0.287
        let score1 = compute_interaction_score(1, Some(last_selected), now);
        assert!(score1 > 0.2 && score1 < 0.35);

        // 9 selections (ln(10) / ln(10) = 1.0) * exp(-1/21) ≈ 0.953
        let score10 = compute_interaction_score(9, Some(last_selected), now);
        assert!(score10 > 0.9 && score10 < 1.0);
    }

    #[test]
    fn test_interaction_score_decay() {
        let now = test_now();

        // Recent selection (1 day ago)
        let recent = now - chrono::Duration::days(1);
        let score_recent = compute_interaction_score(5, Some(recent), now);

        // Old selection (21 days ago - one half-life)
        let old = now - chrono::Duration::days(21);
        let score_old = compute_interaction_score(5, Some(old), now);

        // Score should decay by ~63% after one half-life
        assert!(score_old < score_recent * 0.4);
    }

    #[test]
    fn test_activity_score_no_activity() {
        let score = compute_activity_score(0, 0, 10);
        assert_eq!(score, 0.0, "No active tasks and all complete should be zero");
    }

    #[test]
    fn test_activity_score_active_bonus() {
        // With active tasks
        let score_active = compute_activity_score(1, 5, 10);
        // active_bonus = 1.0, incomplete_ratio = 0.5
        // 0.6 * 1.0 + 0.4 * 0.5 = 0.8
        assert!((score_active - 0.8).abs() < 0.001);

        // Without active tasks
        let score_inactive = compute_activity_score(0, 5, 10);
        // active_bonus = 0.0, incomplete_ratio = 0.5
        // 0.6 * 0.0 + 0.4 * 0.5 = 0.2
        assert!((score_inactive - 0.2).abs() < 0.001);
    }

    #[test]
    fn test_activity_score_completion_ratio() {
        // All incomplete
        let score_all = compute_activity_score(1, 10, 10);
        assert!((score_all - 1.0).abs() < 0.001);

        // Half complete
        let score_half = compute_activity_score(1, 5, 10);
        assert!((score_half - 0.8).abs() < 0.001);

        // Almost complete
        let score_almost = compute_activity_score(1, 1, 10);
        assert!((score_almost - 0.64).abs() < 0.001);
    }

    #[test]
    fn test_recency_score_recent() {
        let now = test_now();
        let recent = now - chrono::Duration::days(1);
        let score = compute_recency_score(recent, now);
        assert!(score > 0.95, "Recent plans should score high");
    }

    #[test]
    fn test_recency_score_decay() {
        let now = test_now();

        // 30 days ago (one half-life)
        let old = now - chrono::Duration::days(30);
        let score = compute_recency_score(old, now);
        assert!((score - 0.368).abs() < 0.01, "Should be e^-1 ≈ 0.368");
    }

    #[test]
    fn test_final_score_weights() {
        let now = test_now();
        let accepted = now - chrono::Duration::days(10);
        let last_selected = now - chrono::Duration::days(5);

        let breakdown = compute_final_score_with_breakdown(
            5,
            Some(last_selected),
            1,
            5,
            10,
            accepted,
            now,
        );

        // Verify weights are applied correctly
        let expected = 0.45 * breakdown.interaction_score
            + 0.35 * breakdown.activity_score
            + 0.20 * breakdown.recency_score;
        assert!((breakdown.final_score - expected).abs() < 0.001);
    }

    #[test]
    fn test_determinism() {
        let now = test_now();
        let accepted = now - chrono::Duration::days(10);
        let last_selected = now - chrono::Duration::days(5);

        // Same inputs should produce same outputs
        let score1 = compute_final_score(5, Some(last_selected), 1, 5, 10, accepted, now);
        let score2 = compute_final_score(5, Some(last_selected), 1, 5, 10, accepted, now);
        assert_eq!(score1, score2, "Scoring must be deterministic");
    }

    #[test]
    fn test_zero_total_tasks() {
        let now = test_now();
        let accepted = now - chrono::Duration::days(10);

        // Plan with no tasks should not crash
        let score = compute_final_score(0, None, 0, 0, 0, accepted, now);
        assert!((0.0..=1.0).contains(&score));
    }
}
