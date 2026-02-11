use crate::error::AppResult;
use rusqlite::Connection;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    let buckets: &[(&str, &str, &str)] = &[
        (
            "research-outputs",
            "Research Outputs",
            r#"{"accepted_types":["research_document","findings","recommendations"],"writers":["deep-researcher","orchestrator"],"readers":["all"]}"#,
        ),
        (
            "work-context",
            "Work Context",
            r#"{"accepted_types":["context","task_spec","previous_work"],"writers":["orchestrator","system"],"readers":["all"]}"#,
        ),
        (
            "code-changes",
            "Code Changes",
            r#"{"accepted_types":["code_change","diff","test_result"],"writers":["worker"],"readers":["all"]}"#,
        ),
        (
            "prd-library",
            "PRD Library",
            r#"{"accepted_types":["prd","specification","design_doc"],"writers":["orchestrator","user"],"readers":["all"]}"#,
        ),
    ];

    for (id, name, config) in buckets {
        conn.execute(
            "INSERT OR IGNORE INTO artifact_buckets (id, name, config_json, is_system)
             VALUES (?1, ?2, ?3, 1)",
            rusqlite::params![id, name, config],
        )
        .map_err(|e| crate::error::AppError::Database(e.to_string()))?;
    }

    Ok(())
}
