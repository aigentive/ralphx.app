use crate::entities::ReviewScopeMetadata;

use super::compute_scope_drift;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeScopeBackstopViolation {
    pub reason: String,
    pub out_of_scope_files: Vec<String>,
}

pub fn evaluate_merge_scope_backstop(
    review_scope: &ReviewScopeMetadata,
    changed_files: &[String],
) -> Option<MergeScopeBackstopViolation> {
    if review_scope.planned_paths.is_empty() {
        return None;
    }

    let (_, current_out_of_scope_files) =
        compute_scope_drift(changed_files, &review_scope.planned_paths);

    if current_out_of_scope_files.is_empty() {
        return None;
    }

    match review_scope.drift_classification.as_deref() {
        None => Some(MergeScopeBackstopViolation {
            reason: format!(
                "Task branch still contains scope expansion at merge time, but review never recorded a drift classification: {}",
                current_out_of_scope_files.join(", ")
            ),
            out_of_scope_files: current_out_of_scope_files,
        }),
        Some("unrelated_drift") => Some(MergeScopeBackstopViolation {
            reason: format!(
                "Task branch still contains unrelated scope drift at merge time: {}",
                current_out_of_scope_files.join(", ")
            ),
            out_of_scope_files: current_out_of_scope_files,
        }),
        Some("adjacent_scope_expansion") | Some("plan_correction") => {
            let reviewed = review_scope
                .reviewed_out_of_scope_files
                .iter()
                .cloned()
                .collect::<std::collections::HashSet<_>>();
            let unreviewed = current_out_of_scope_files
                .iter()
                .filter(|path| !reviewed.contains(*path))
                .cloned()
                .collect::<Vec<_>>();

            if unreviewed.is_empty() {
                None
            } else {
                Some(MergeScopeBackstopViolation {
                    reason: format!(
                        "Task branch introduced new out-of-scope files after review without fresh classification: {}",
                        unreviewed.join(", ")
                    ),
                    out_of_scope_files: unreviewed,
                })
            }
        }
        Some(other) => Some(MergeScopeBackstopViolation {
            reason: format!(
                "Task branch has unsupported review scope drift classification '{}' at merge time",
                other
            ),
            out_of_scope_files: current_out_of_scope_files,
        }),
    }
}

#[cfg(test)]
#[path = "merge_backstop_tests.rs"]
mod tests;
