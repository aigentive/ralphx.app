use rusqlite::Connection;

fn setup_base_table(conn: &Connection) {
    conn.execute_batch(
        "CREATE TABLE ideation_sessions (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'active',
            title TEXT,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')),
            updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))
        );",
    )
    .unwrap();
}

#[test]
fn test_v62_adds_verification_generation_column() {
    let conn = Connection::open_in_memory().unwrap();
    setup_base_table(&conn);

    super::v63_auto_verify_generation::migrate(&conn).unwrap();

    conn.execute(
        "INSERT INTO ideation_sessions (id, project_id, status, verification_generation)
         VALUES ('s1', 'p1', 'active', 0)",
        [],
    )
    .unwrap();
    let gen: i32 = conn
        .query_row(
            "SELECT verification_generation FROM ideation_sessions WHERE id = 's1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(gen, 0);
}

#[test]
fn test_v62_default_value_is_zero() {
    let conn = Connection::open_in_memory().unwrap();
    setup_base_table(&conn);

    super::v63_auto_verify_generation::migrate(&conn).unwrap();

    // Insert without specifying verification_generation — should default to 0
    conn.execute(
        "INSERT INTO ideation_sessions (id, project_id, status)
         VALUES ('s2', 'p1', 'active')",
        [],
    )
    .unwrap();
    let gen: i32 = conn
        .query_row(
            "SELECT verification_generation FROM ideation_sessions WHERE id = 's2'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(gen, 0);
}

#[test]
fn test_v62_idempotent() {
    let conn = Connection::open_in_memory().unwrap();
    setup_base_table(&conn);

    super::v63_auto_verify_generation::migrate(&conn).unwrap();
    // Running again should not fail (add_column_if_not_exists is idempotent)
    super::v63_auto_verify_generation::migrate(&conn).unwrap();
}
