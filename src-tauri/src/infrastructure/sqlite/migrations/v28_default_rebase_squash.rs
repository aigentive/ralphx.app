// Migration v28: Change default merge_strategy from 'rebase' to 'rebase_squash'
//
// Updates all existing projects that still have the old default value
// (merge_strategy = 'rebase') to the new default (merge_strategy = 'rebase_squash').
// Projects explicitly set to 'merge' or 'squash' remain untouched.

use rusqlite::Connection;

use crate::error::AppResult;

/// Migration v28: Update merge_strategy default from 'rebase' to 'rebase_squash'
pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute(
        "UPDATE projects SET merge_strategy = 'rebase_squash' WHERE merge_strategy = 'rebase';",
        [],
    )
    .map_err(|e| crate::error::AppError::Database(e.to_string()))?;
    Ok(())
}
