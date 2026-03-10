use rusqlite::Connection;

use crate::error::AppResult;

/// Grant the admin bit (permissions | 4) to existing keys with the old default value
/// (permissions = 3) that are not revoked.
///
/// Background: commit 1ef08526 added `require_admin_key` middleware to key management
/// routes, which requires bit 4 (admin). Keys created before this change have
/// permissions = 3 (read=1, write=2) and need the admin bit added so they continue
/// to work for managing their own keys via the settings UI.
///
/// Only the exact old default (3) is upgraded — keys with custom permissions
/// (1, 2, or other values) are left unchanged since those were intentionally set.
///
/// # Rollback SQL
/// ```sql
/// UPDATE api_keys SET permissions = permissions & ~4 WHERE permissions = 7;
/// ```
pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "UPDATE api_keys
         SET permissions = permissions | 4
         WHERE permissions = 3
           AND revoked_at IS NULL;",
    )?;
    Ok(())
}
