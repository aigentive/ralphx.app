// Migration v38: Add team mode columns to ideation_sessions
//
// Adds team_mode and team_config_json to ideation_sessions for multi-agent
// ideation support. Null values = solo mode (backwards compatible).

use crate::error::AppResult;
use rusqlite::Connection;

use super::helpers::add_column_if_not_exists;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    // team_mode: 'solo' | 'research' | 'debate' (null = solo for backwards compat)
    add_column_if_not_exists(conn, "ideation_sessions", "team_mode", "TEXT")?;

    // team_config_json: JSON blob with max_teammates, model_ceiling, budget_limit, composition_mode
    add_column_if_not_exists(conn, "ideation_sessions", "team_config_json", "TEXT")?;

    Ok(())
}
