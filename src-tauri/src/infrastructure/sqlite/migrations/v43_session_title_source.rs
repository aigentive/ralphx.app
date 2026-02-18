// Migration v43: Add title_source column to ideation_sessions
//
// Tracks whether the session title was set by the session-namer agent ('auto')
// or by the user via manual rename ('user'). Used to decide whether to re-trigger
// the session-namer at plan acceptance time.
// Null values = 'auto' for backwards compatibility with existing sessions.

use crate::error::AppResult;
use rusqlite::Connection;

use super::helpers::add_column_if_not_exists;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    // title_source: 'auto' (set by session-namer) | 'user' (set by manual rename)
    // NULL treated as 'auto' for backwards compatibility
    add_column_if_not_exists(conn, "ideation_sessions", "title_source", "TEXT")
}
