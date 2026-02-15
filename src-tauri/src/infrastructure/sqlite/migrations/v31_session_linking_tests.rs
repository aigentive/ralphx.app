// V31 migration tests - session linking schema

use super::helpers;
use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

/// Helper to create a project (required FK parent for sessions)
fn create_project(conn: &rusqlite::Connection, id: &str) {
    conn.execute(
        "INSERT INTO projects (id, name, working_directory)
         VALUES (?1, 'Test Project', '/tmp/test')",
        [id],
    )
    .unwrap();
}

/// Helper to create an ideation session
fn create_session(
    conn: &rusqlite::Connection,
    id: &str,
    project_id: &str,
    parent_session_id: Option<&str>,
) {
    conn.execute(
        "INSERT INTO ideation_sessions (id, project_id, parent_session_id, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, 'active',
                 strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'),
                 strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
        rusqlite::params![id, project_id, parent_session_id],
    )
    .unwrap();
}

#[test]
fn test_v31_parent_session_id_column_exists() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    assert!(helpers::column_exists(
        &conn,
        "ideation_sessions",
        "parent_session_id"
    ));
}

#[test]
fn test_v31_parent_session_id_is_nullable() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    create_project(&conn, "p-1");
    create_session(&conn, "sess-1", "p-1", None);

    let result: Option<String> = conn
        .query_row(
            "SELECT parent_session_id FROM ideation_sessions WHERE id = 'sess-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(result.is_none());
}

#[test]
fn test_v31_parent_session_id_can_be_set() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    create_project(&conn, "p-1");
    create_session(&conn, "parent-sess", "p-1", None);
    create_session(&conn, "child-sess", "p-1", Some("parent-sess"));

    let result: Option<String> = conn
        .query_row(
            "SELECT parent_session_id FROM ideation_sessions WHERE id = 'child-sess'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(result, Some("parent-sess".to_string()));
}

#[test]
fn test_v31_session_links_table_exists() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    assert!(helpers::table_exists(&conn, "session_links"));
}

#[test]
fn test_v31_session_links_basic_insert() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    create_project(&conn, "p-1");
    create_session(&conn, "parent-sess", "p-1", None);
    create_session(&conn, "child-sess", "p-1", None);

    conn.execute(
        "INSERT INTO session_links (id, parent_session_id, child_session_id, relationship)
         VALUES ('link-1', 'parent-sess', 'child-sess', 'follow_on')",
        [],
    )
    .unwrap();

    let (parent_id, child_id): (String, String) = conn
        .query_row(
            "SELECT parent_session_id, child_session_id FROM session_links WHERE id = 'link-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();

    assert_eq!(parent_id, "parent-sess");
    assert_eq!(child_id, "child-sess");
}

#[test]
fn test_v31_self_reference_check_constraint() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    create_project(&conn, "p-1");
    create_session(&conn, "sess-1", "p-1", None);

    // Attempting to create a self-referencing link should fail
    let result = conn.execute(
        "INSERT INTO session_links (id, parent_session_id, child_session_id, relationship)
         VALUES ('link-self', 'sess-1', 'sess-1', 'follow_on')",
        [],
    );

    assert!(result.is_err());
}

#[test]
fn test_v31_unique_constraint_on_parent_child_pair() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    create_project(&conn, "p-1");
    create_session(&conn, "parent-sess", "p-1", None);
    create_session(&conn, "child-sess", "p-1", None);

    // First insert should succeed
    conn.execute(
        "INSERT INTO session_links (id, parent_session_id, child_session_id, relationship)
         VALUES ('link-1', 'parent-sess', 'child-sess', 'follow_on')",
        [],
    )
    .unwrap();

    // Second insert with same parent/child should fail (different link ID)
    let result = conn.execute(
        "INSERT INTO session_links (id, parent_session_id, child_session_id, relationship)
         VALUES ('link-2', 'parent-sess', 'child-sess', 'follow_on')",
        [],
    );

    assert!(result.is_err());
}

#[test]
fn test_v31_on_delete_cascade_for_session_links() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    create_project(&conn, "p-1");
    create_session(&conn, "parent-sess", "p-1", None);
    create_session(&conn, "child-sess", "p-1", None);

    // Create a session link
    conn.execute(
        "INSERT INTO session_links (id, parent_session_id, child_session_id, relationship)
         VALUES ('link-1', 'parent-sess', 'child-sess', 'follow_on')",
        [],
    )
    .unwrap();

    // Delete the parent session
    conn.execute("DELETE FROM ideation_sessions WHERE id = 'parent-sess'", [])
        .unwrap();

    // The session_link entry should also be deleted
    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM session_links WHERE id = 'link-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(count, 0);
}

#[test]
fn test_v31_on_delete_set_null_for_parent_session_id() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    create_project(&conn, "p-1");
    create_session(&conn, "parent-sess", "p-1", None);
    create_session(&conn, "child-sess", "p-1", Some("parent-sess"));

    // Verify parent_session_id is set
    let parent_id: Option<String> = conn
        .query_row(
            "SELECT parent_session_id FROM ideation_sessions WHERE id = 'child-sess'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(parent_id, Some("parent-sess".to_string()));

    // Delete the parent session
    conn.execute("DELETE FROM ideation_sessions WHERE id = 'parent-sess'", [])
        .unwrap();

    // The child session's parent_session_id should now be NULL
    let parent_id: Option<String> = conn
        .query_row(
            "SELECT parent_session_id FROM ideation_sessions WHERE id = 'child-sess'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(parent_id.is_none());
}

#[test]
fn test_v31_parent_session_id_index_exists() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    assert!(helpers::index_exists(&conn, "idx_ideation_sessions_parent"));
}

#[test]
fn test_v31_session_links_parent_index_exists() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    assert!(helpers::index_exists(&conn, "idx_session_links_parent"));
}

#[test]
fn test_v31_session_links_child_index_exists() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    assert!(helpers::index_exists(&conn, "idx_session_links_child"));
}

#[test]
fn test_v31_session_links_default_relationship() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    create_project(&conn, "p-1");
    create_session(&conn, "parent-sess", "p-1", None);
    create_session(&conn, "child-sess", "p-1", None);

    // Insert without specifying relationship
    conn.execute(
        "INSERT INTO session_links (id, parent_session_id, child_session_id)
         VALUES ('link-1', 'parent-sess', 'child-sess')",
        [],
    )
    .unwrap();

    let relationship: String = conn
        .query_row(
            "SELECT relationship FROM session_links WHERE id = 'link-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(relationship, "follow_on");
}

#[test]
fn test_v31_session_links_notes_nullable() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    create_project(&conn, "p-1");
    create_session(&conn, "parent-sess", "p-1", None);
    create_session(&conn, "child-sess", "p-1", None);

    // Insert without notes
    conn.execute(
        "INSERT INTO session_links (id, parent_session_id, child_session_id, relationship)
         VALUES ('link-1', 'parent-sess', 'child-sess', 'follow_on')",
        [],
    )
    .unwrap();

    let notes: Option<String> = conn
        .query_row(
            "SELECT notes FROM session_links WHERE id = 'link-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(notes.is_none());
}

#[test]
fn test_v31_session_links_with_notes() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    create_project(&conn, "p-1");
    create_session(&conn, "parent-sess", "p-1", None);
    create_session(&conn, "child-sess", "p-1", None);

    let note_text = "This session follows up on review gaps";
    conn.execute(
        "INSERT INTO session_links (id, parent_session_id, child_session_id, relationship, notes)
         VALUES ('link-1', 'parent-sess', 'child-sess', 'follow_on', ?1)",
        [note_text],
    )
    .unwrap();

    let notes: String = conn
        .query_row(
            "SELECT notes FROM session_links WHERE id = 'link-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(notes, note_text);
}

#[test]
fn test_v31_idempotent() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    // Running migrations again should not error
    run_migrations(&conn).unwrap();

    assert!(helpers::column_exists(
        &conn,
        "ideation_sessions",
        "parent_session_id"
    ));
    assert!(helpers::table_exists(&conn, "session_links"));
}
