// Query builder helpers for complex conditional queries

use super::queries::TASK_COLUMNS;

/// Build paginated query based on status count and archived filters
///
/// # Arguments
/// * `status_count` - Number of statuses to filter by (0 = no filter)
/// * `include_archived` - Whether to include archived tasks
/// * `has_session_filter` - Whether to filter by ideation_session_id
///
/// When status_count > 0, generates `internal_status IN (?2, ?3, ...)` clause.
/// When has_session_filter = true, adds ideation_session_id = ? filter.
/// Parameter indices depend on filters used (see implementation)
pub(super) fn build_paginated_query(status_count: usize, include_archived: bool, has_session_filter: bool) -> String {
    let mut conditions = vec!["project_id = ?1".to_string()];
    let mut param_idx = 2;

    // Add status IN clause if needed
    if status_count > 0 {
        let placeholders: Vec<String> = (param_idx..param_idx + status_count)
            .map(|i| format!("?{}", i))
            .collect();
        let in_clause = placeholders.join(", ");
        conditions.push(format!("internal_status IN ({})", in_clause));
        param_idx += status_count;
    }

    // Add archived filter
    if !include_archived {
        conditions.push("archived_at IS NULL".to_string());
    }

    // Add session filter
    if has_session_filter {
        conditions.push(format!("ideation_session_id = ?{}", param_idx));
        param_idx += 1;
    }

    let where_clause = conditions.join(" AND ");
    let limit_idx = param_idx;
    let offset_idx = param_idx + 1;

    format!(
        "SELECT {} FROM tasks WHERE {} ORDER BY created_at DESC LIMIT ?{} OFFSET ?{}",
        TASK_COLUMNS, where_clause, limit_idx, offset_idx
    )
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
