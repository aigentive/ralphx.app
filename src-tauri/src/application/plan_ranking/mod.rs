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
mod tests;
