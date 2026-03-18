// Memory rule binding entity for tracking rule file sync state

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::ProcessId;

/// Memory rule binding for tracking rule file state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRuleBinding {
    pub project_id: ProcessId,
    pub scope_key: String,
    pub rule_file_path: String,
    /// Glob patterns from YAML frontmatter
    pub paths: Vec<String>,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub last_content_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl MemoryRuleBinding {
    /// Create a new memory rule binding
    pub fn new(
        project_id: ProcessId,
        scope_key: impl Into<String>,
        rule_file_path: impl Into<String>,
        paths: Vec<String>,
    ) -> Self {
        let now = Utc::now();

        Self {
            project_id,
            scope_key: scope_key.into(),
            rule_file_path: rule_file_path.into(),
            paths,
            last_synced_at: None,
            last_content_hash: None,
            created_at: now,
            updated_at: now,
        }
    }
}
