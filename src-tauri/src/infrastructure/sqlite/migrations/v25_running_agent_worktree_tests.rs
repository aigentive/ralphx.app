use crate::infrastructure::sqlite::migrations::{
    helpers::column_exists, v17_running_agents, v25_running_agent_worktree,
};
use crate::infrastructure::sqlite::open_memory_connection;

#[test]
fn add_worktree_path_column() {
    let conn = open_memory_connection().expect("open connection");

    v17_running_agents::migrate(&conn).expect("migrate running_agents table");
    assert!(column_exists(&conn, "running_agents", "pid"));
    assert!(!column_exists(&conn, "running_agents", "worktree_path"));

    v25_running_agent_worktree::migrate(&conn).expect("add worktree_path column");
    assert!(column_exists(&conn, "running_agents", "worktree_path"));
}
