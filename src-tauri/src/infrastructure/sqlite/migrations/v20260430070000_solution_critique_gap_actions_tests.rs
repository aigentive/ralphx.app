use rusqlite::Connection;

use super::helpers::{column_exists, index_exists, table_exists};
use super::v20260430070000_solution_critique_gap_actions::migrate;

#[test]
fn creates_solution_critique_gap_actions_table_and_indexes() {
    let conn = Connection::open_in_memory().unwrap();

    migrate(&conn).unwrap();

    assert!(table_exists(&conn, "solution_critique_gap_actions"));
    for column in [
        "id",
        "session_id",
        "project_id",
        "target_type",
        "target_id",
        "critique_artifact_id",
        "context_artifact_id",
        "gap_id",
        "gap_fingerprint",
        "action",
        "note",
        "actor_kind",
        "verification_generation",
        "promoted_round",
        "created_at",
    ] {
        assert!(column_exists(
            &conn,
            "solution_critique_gap_actions",
            column
        ));
    }
    assert!(index_exists(
        &conn,
        "idx_solution_critique_gap_actions_target"
    ));
    assert!(index_exists(&conn, "idx_solution_critique_gap_actions_gap"));

    migrate(&conn).unwrap();
}
