// V27 migration tests - merge_strategy column

use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

#[test]
fn test_v27_column_exists() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    let columns: Vec<String> = conn
        .prepare("PRAGMA table_info(projects)")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .flatten()
        .collect();

    assert!(columns.contains(&"merge_strategy".to_string()));
}

#[test]
fn test_v27_defaults_to_rebase() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
        [],
    )
    .unwrap();

    let strategy: String = conn
        .query_row(
            "SELECT merge_strategy FROM projects WHERE id = 'proj-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(strategy, "rebase");
}

#[test]
fn test_v27_accepts_valid_values() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    for (id, strategy) in [("p1", "rebase"), ("p2", "merge")] {
        let path = format!("/path/{}", id);
        conn.execute(
            "INSERT INTO projects (id, name, working_directory, merge_strategy) VALUES (?1, 'Test', ?3, ?2)",
            rusqlite::params![id, strategy, path],
        )
        .unwrap();

        let stored: String = conn
            .query_row(
                "SELECT merge_strategy FROM projects WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(stored, strategy);
    }
}

#[test]
fn test_v27_idempotent() {
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

    assert!(columns.contains(&"merge_strategy".to_string()));
}
