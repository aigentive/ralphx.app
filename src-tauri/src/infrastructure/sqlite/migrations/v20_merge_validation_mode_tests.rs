// V20 migration tests - merge_validation_mode column

use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

#[test]
fn test_v20_column_exists() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    let columns: Vec<String> = conn
        .prepare("PRAGMA table_info(projects)")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .flatten()
        .collect();

    assert!(columns.contains(&"merge_validation_mode".to_string()));
}

#[test]
fn test_v20_defaults_to_block() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
        [],
    )
    .unwrap();

    let mode: String = conn
        .query_row(
            "SELECT merge_validation_mode FROM projects WHERE id = 'proj-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(mode, "block");
}

#[test]
fn test_v20_accepts_valid_values() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    for (id, mode) in [("p1", "block"), ("p2", "warn"), ("p3", "off")] {
        conn.execute(
            "INSERT INTO projects (id, name, working_directory, merge_validation_mode) VALUES (?1, 'Test', '/path', ?2)",
            rusqlite::params![id, mode],
        )
        .unwrap();

        let stored: String = conn
            .query_row(
                "SELECT merge_validation_mode FROM projects WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(stored, mode);
    }
}

#[test]
fn test_v20_idempotent() {
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

    assert!(columns.contains(&"merge_validation_mode".to_string()));
}
