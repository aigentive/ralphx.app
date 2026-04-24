use rusqlite::Connection;

use super::helpers;
use super::v20260424113000_design_system_store;

fn setup_test_db() -> Connection {
    let conn = Connection::open_in_memory().expect("Failed to create in-memory database");
    conn.execute_batch(
        "CREATE TABLE projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL
        );

        CREATE TABLE chat_conversations (
            id TEXT PRIMARY KEY
        );

        CREATE TABLE chat_messages (
            id TEXT PRIMARY KEY
        );",
    )
    .expect("Failed to create test schema");
    conn
}

#[test]
fn test_design_system_store_tables_created() {
    let conn = setup_test_db();

    v20260424113000_design_system_store::migrate(&conn).unwrap();

    for table in [
        "design_systems",
        "design_system_sources",
        "design_schema_versions",
        "design_styleguide_items",
        "design_styleguide_feedback",
        "design_runs",
    ] {
        assert!(helpers::table_exists(&conn, table), "{table} should exist");
    }

    assert!(helpers::column_exists(
        &conn,
        "design_systems",
        "storage_root_ref"
    ));
    assert!(helpers::column_exists(
        &conn,
        "design_system_sources",
        "selected_paths_json"
    ));
    assert!(helpers::column_exists(
        &conn,
        "design_styleguide_items",
        "source_refs_json"
    ));
    assert!(helpers::column_exists(
        &conn,
        "design_runs",
        "output_artifact_ids_json"
    ));
}

#[test]
fn test_design_system_store_indexes_created() {
    let conn = setup_test_db();

    v20260424113000_design_system_store::migrate(&conn).unwrap();

    for index in [
        "idx_design_systems_project_active",
        "idx_design_system_sources_system",
        "idx_design_schema_versions_system",
        "idx_design_styleguide_items_system_schema",
        "idx_design_styleguide_feedback_open",
        "idx_design_runs_system",
    ] {
        assert!(helpers::index_exists(&conn, index), "{index} should exist");
    }
}

#[test]
fn test_design_system_store_unique_schema_version_per_system() {
    let conn = setup_test_db();

    v20260424113000_design_system_store::migrate(&conn).unwrap();

    conn.execute(
        "INSERT INTO projects (id, name) VALUES ('p1', 'Project')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO design_systems (
            id, primary_project_id, name, status, storage_root_ref, created_at, updated_at
         ) VALUES ('ds1', 'p1', 'System', 'draft', 'root-hash', '2026-04-24T00:00:00Z', '2026-04-24T00:00:00Z')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO design_schema_versions (
            id, design_system_id, version, schema_artifact_id, manifest_artifact_id,
            styleguide_artifact_id, status, created_at
         ) VALUES ('sv1', 'ds1', 'v1', 'schema-a', 'manifest-a', 'styleguide-a', 'draft', '2026-04-24T00:00:00Z')",
        [],
    )
    .unwrap();

    let duplicate = conn.execute(
        "INSERT INTO design_schema_versions (
            id, design_system_id, version, schema_artifact_id, manifest_artifact_id,
            styleguide_artifact_id, status, created_at
         ) VALUES ('sv2', 'ds1', 'v1', 'schema-b', 'manifest-b', 'styleguide-b', 'draft', '2026-04-24T00:00:00Z')",
        [],
    );

    assert!(
        duplicate.is_err(),
        "schema version labels must be unique per design system"
    );
}

#[test]
fn test_design_system_store_idempotent() {
    let conn = setup_test_db();

    v20260424113000_design_system_store::migrate(&conn).unwrap();
    v20260424113000_design_system_store::migrate(&conn).unwrap();

    assert!(helpers::table_exists(&conn, "design_systems"));
    assert!(helpers::table_exists(&conn, "design_runs"));
    assert!(helpers::index_exists(
        &conn,
        "idx_design_styleguide_items_system_schema"
    ));
}
