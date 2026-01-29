// Query builder helpers for complex conditional queries

use super::queries::TASK_COLUMNS;

/// Build paginated query based on status count and archived filters
///
/// # Arguments
/// * `status_count` - Number of statuses to filter by (0 = no filter)
/// * `include_archived` - Whether to include archived tasks
///
/// When status_count > 0, generates `internal_status IN (?2, ?3, ...)` clause.
/// Parameter indices: ?1=project_id, ?2..?(1+count)=statuses, ?(2+count)=limit, ?(3+count)=offset
pub(super) fn build_paginated_query(status_count: usize, include_archived: bool) -> String {
    let base = format!("SELECT {} FROM tasks WHERE project_id = ?1", TASK_COLUMNS);

    if status_count == 0 {
        // No status filter
        if include_archived {
            format!("{} ORDER BY created_at DESC LIMIT ?2 OFFSET ?3", base)
        } else {
            format!(
                "{} AND archived_at IS NULL ORDER BY created_at DESC LIMIT ?2 OFFSET ?3",
                base
            )
        }
    } else {
        // Build IN clause: ?2, ?3, ... for status_count statuses
        let placeholders: Vec<String> = (2..=status_count + 1)
            .map(|i| format!("?{}", i))
            .collect();
        let in_clause = placeholders.join(", ");
        let limit_idx = status_count + 2;
        let offset_idx = status_count + 3;

        if include_archived {
            format!(
                "{} AND internal_status IN ({}) ORDER BY created_at DESC LIMIT ?{} OFFSET ?{}",
                base, in_clause, limit_idx, offset_idx
            )
        } else {
            format!(
                "{} AND internal_status IN ({}) AND archived_at IS NULL ORDER BY created_at DESC LIMIT ?{} OFFSET ?{}",
                base, in_clause, limit_idx, offset_idx
            )
        }
    }
}

/// Build search query with archived filter
pub(super) fn build_search_query(include_archived: bool) -> String {
    let base = format!(
        "SELECT {} FROM tasks WHERE project_id = ?1 AND (LOWER(title) LIKE LOWER(?2) OR LOWER(description) LIKE LOWER(?2))",
        TASK_COLUMNS
    );

    if include_archived {
        format!("{} ORDER BY created_at DESC", base)
    } else {
        format!(
            "{} AND archived_at IS NULL ORDER BY created_at DESC",
            base
        )
    }
}

/// Build filtered project query
pub(super) fn build_filtered_query(include_archived: bool) -> String {
    let base = format!("SELECT {} FROM tasks WHERE project_id = ?1", TASK_COLUMNS);

    if include_archived {
        format!("{} ORDER BY priority DESC, created_at ASC", base)
    } else {
        format!(
            "{} AND archived_at IS NULL ORDER BY priority DESC, created_at ASC",
            base
        )
    }
}
