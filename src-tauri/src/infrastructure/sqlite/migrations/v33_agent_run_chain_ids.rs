// Migration v33: Add run chain correlation IDs to agent_runs
//
// Adds run_chain_id and parent_run_id columns to the agent_runs table.
// - run_chain_id: UUID linking all agent runs from a single message chain
//   (initial run + all queue continuations via --resume)
// - parent_run_id: references the agent_run that triggered this continuation
//
// Both columns are nullable TEXT with NULL defaults for backward compatibility.

use rusqlite::Connection;

use super::helpers;
use crate::error::AppResult;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    helpers::add_column_if_not_exists(conn, "agent_runs", "run_chain_id", "TEXT")?;
    helpers::add_column_if_not_exists(conn, "agent_runs", "parent_run_id", "TEXT")?;

    helpers::create_index_if_not_exists(
        conn,
        "idx_agent_runs_chain",
        "agent_runs",
        "run_chain_id",
    )?;

    Ok(())
}
