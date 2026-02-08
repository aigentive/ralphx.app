// V19 migration tests - project analysis columns

use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

#[test]
fn test_v19_columns_exist() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    let columns: Vec<String> = conn
        .prepare("PRAGMA table_info(projects)")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .flatten()
        .collect();

    assert!(columns.contains(&"detected_analysis".to_string()));
    assert!(columns.contains(&"custom_analysis".to_string()));
    assert!(columns.contains(&"analyzed_at".to_string()));
}

#[test]
fn test_v19_columns_default_to_null() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
        [],
    )
    .unwrap();

    let (detected, custom, analyzed_at): (Option<String>, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT detected_analysis, custom_analysis, analyzed_at FROM projects WHERE id = 'proj-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();

    assert!(detected.is_none(), "detected_analysis should default to NULL");
    assert!(custom.is_none(), "custom_analysis should default to NULL");
    assert!(analyzed_at.is_none(), "analyzed_at should default to NULL");
}

#[test]
fn test_v19_set_and_get_analysis() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    let analysis_json = r#"[{"path":".","label":"Node.js","validate":["npm run typecheck"]}]"#;

    conn.execute(
        "INSERT INTO projects (id, name, working_directory, detected_analysis, analyzed_at)
         VALUES ('proj-1', 'Test', '/path', ?1, '2026-02-08T09:00:00+00:00')",
        [analysis_json],
    )
    .unwrap();

    let (detected, analyzed_at): (Option<String>, Option<String>) = conn
        .query_row(
            "SELECT detected_analysis, analyzed_at FROM projects WHERE id = 'proj-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();

    assert_eq!(detected, Some(analysis_json.to_string()));
    assert_eq!(analyzed_at, Some("2026-02-08T09:00:00+00:00".to_string()));
}

#[test]
fn test_v19_custom_analysis_independent() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    let custom_json = r#"[{"path":".","label":"Custom","validate":["make test"]}]"#;

    conn.execute(
        "INSERT INTO projects (id, name, working_directory, custom_analysis)
         VALUES ('proj-1', 'Test', '/path', ?1)",
        [custom_json],
    )
    .unwrap();

    let (detected, custom): (Option<String>, Option<String>) = conn
        .query_row(
            "SELECT detected_analysis, custom_analysis FROM projects WHERE id = 'proj-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();

    assert!(detected.is_none(), "detected_analysis should remain NULL");
    assert_eq!(custom, Some(custom_json.to_string()));
}

#[test]
fn test_v19_idempotent() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    run_migrations(&conn).unwrap();

    let columns: Vec<String> = conn
        .prepare("PRAGMA table_info(projects)")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .flatten()
        .collect();

    assert!(columns.contains(&"detected_analysis".to_string()));
    assert!(columns.contains(&"custom_analysis".to_string()));
    assert!(columns.contains(&"analyzed_at".to_string()));
}
