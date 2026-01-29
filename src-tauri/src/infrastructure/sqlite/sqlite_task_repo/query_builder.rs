// Query builder helpers for complex conditional queries

use super::queries::TASK_COLUMNS;

/// Build paginated query based on status and archived filters
pub(super) fn build_paginated_query(has_status: bool, include_archived: bool) -> String {
    let base = format!("SELECT {} FROM tasks WHERE project_id = ?1", TASK_COLUMNS);

    match (has_status, include_archived) {
        (true, true) => {
            format!(
                "{} AND internal_status = ?2 ORDER BY created_at DESC LIMIT ?3 OFFSET ?4",
                base
            )
        }
        (true, false) => format!(
            "{} AND internal_status = ?2 AND archived_at IS NULL ORDER BY created_at DESC LIMIT ?3 OFFSET ?4",
            base
        ),
        (false, true) => format!("{} ORDER BY created_at DESC LIMIT ?2 OFFSET ?3", base),
        (false, false) => format!(
            "{} AND archived_at IS NULL ORDER BY created_at DESC LIMIT ?2 OFFSET ?3",
            base
        ),
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
