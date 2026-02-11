// Index rewriter service - transforms rule files into canonical index format
// Replaces full content with compact summaries, memory IDs, and retrieval instructions

use std::fs;
use std::path::Path;

use crate::domain::entities::MemoryEntry;
use crate::error::{AppError, AppResult};

/// Service for rewriting rule files to canonical index format
pub struct IndexRewriter;

/// Result of index rewriting operation
#[derive(Debug)]
pub struct RewriteResult {
    /// Path to the rewritten file
    pub file_path: String,
    /// Number of memory entries referenced
    pub memory_count: usize,
}

impl IndexRewriter {
    /// Create a new IndexRewriter instance
    pub fn new() -> Self {
        Self
    }

    /// Rewrite a rule file to canonical index format
    ///
    /// # Arguments
    /// * `file_path` - Path to the rule file to rewrite
    /// * `paths` - Normalized path globs for frontmatter
    /// * `memories` - Memory entries that were ingested from this file
    ///
    /// # Returns
    /// Result with rewrite metadata
    pub fn rewrite_rule_file(
        &self,
        file_path: &str,
        paths: Vec<String>,
        memories: &[MemoryEntry],
    ) -> AppResult<RewriteResult> {
        // Generate canonical index content
        let index_content = self.generate_index_content(file_path, paths, memories)?;

        // Write atomically using temp file + rename
        self.write_atomic(file_path, &index_content)?;

        Ok(RewriteResult {
            file_path: file_path.to_string(),
            memory_count: memories.len(),
        })
    }

    /// Generate canonical index content
    fn generate_index_content(
        &self,
        file_path: &str,
        paths: Vec<String>,
        memories: &[MemoryEntry],
    ) -> AppResult<String> {
        let mut content = String::new();

        // 1. YAML frontmatter with normalized paths
        let normalized_paths = self.normalize_paths(paths);
        content.push_str("---\n");
        content.push_str("paths:\n");
        for path in normalized_paths {
            content.push_str(&format!("  - \"{}\"\n", path));
        }
        content.push_str("---\n\n");

        // 2. Title (derive from file name)
        let title = self.derive_title_from_path(file_path);
        content.push_str(&format!("# Memory Index: {}\n\n", title));

        // 3. Summary section (aggregate summaries from all memories)
        content.push_str("## Summary\n\n");
        if memories.is_empty() {
            content.push_str("(No memory entries ingested)\n\n");
        } else {
            for memory in memories {
                content.push_str(&format!("- {}\n", memory.summary));
            }
            content.push('\n');
        }

        // 4. Memory References section (grouped by bucket)
        content.push_str("## Memory References\n\n");
        if memories.is_empty() {
            content.push_str("(No memory entries)\n\n");
        } else {
            let grouped = self.group_memories_by_bucket(memories);
            for (bucket, entries) in grouped {
                content.push_str(&format!("### {}\n\n", bucket));
                for entry in entries {
                    content.push_str(&format!("- `{}` - {}\n", entry.id, entry.title));
                }
                content.push('\n');
            }
        }

        // 5. Retrieval instructions
        content.push_str("## Retrieval\n\n");
        content.push_str("To retrieve full memory details:\n\n");
        content.push_str("- Use `get_memories_for_paths` with affected file paths\n");
        content.push_str("- Use `get_memory` with specific memory ID for full details\n");
        content.push_str("- Use `search_memories` for keyword-based retrieval\n");

        Ok(content)
    }

    /// Normalize paths: sort alphabetically, remove duplicates, consistent formatting
    fn normalize_paths(&self, mut paths: Vec<String>) -> Vec<String> {
        // Remove duplicates
        paths.sort();
        paths.dedup();
        paths
    }

    /// Derive a human-readable title from a file path
    fn derive_title_from_path(&self, file_path: &str) -> String {
        let path = Path::new(file_path);
        let file_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown");

        // Convert snake_case or kebab-case to Title Case
        file_name
            .replace(['-', '_'], " ")
            .split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Group memories by bucket for organized display
    fn group_memories_by_bucket<'a>(
        &self,
        memories: &'a [MemoryEntry],
    ) -> Vec<(String, Vec<&'a MemoryEntry>)> {
        use std::collections::HashMap;

        let mut buckets: HashMap<String, Vec<&MemoryEntry>> = HashMap::new();

        for memory in memories {
            let bucket_name = memory.bucket.to_string();
            buckets
                .entry(bucket_name)
                .or_insert_with(Vec::new)
                .push(memory);
        }

        // Sort buckets alphabetically for determinism
        let mut result: Vec<(String, Vec<&MemoryEntry>)> = buckets.into_iter().collect();
        result.sort_by(|a, b| a.0.cmp(&b.0));

        result
    }

    /// Write content to file atomically using temp file + rename
    fn write_atomic(&self, file_path: &str, content: &str) -> AppResult<()> {
        let path = Path::new(file_path);

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                AppError::Infrastructure(format!("Failed to create directory: {}", e))
            })?;
        }

        // Write to temporary file first
        let temp_path = format!("{}.tmp", file_path);
        fs::write(&temp_path, content).map_err(|e| {
            AppError::Infrastructure(format!("Failed to write temp file: {}", e))
        })?;

        // Atomic rename
        fs::rename(&temp_path, file_path).map_err(|e| {
            // Clean up temp file on failure
            let _ = fs::remove_file(&temp_path);
            AppError::Infrastructure(format!("Failed to rename temp file: {}", e))
        })?;

        Ok(())
    }
}

impl Default for IndexRewriter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{MemoryBucket, MemoryEntry, ProcessId};

    #[test]
    fn test_normalize_paths() {
        let rewriter = IndexRewriter::new();

        let paths = vec![
            "src/components/**".to_string(),
            "src/api/**".to_string(),
            "src/components/**".to_string(), // duplicate
            "src-tauri/**".to_string(),
        ];

        let normalized = rewriter.normalize_paths(paths);

        assert_eq!(normalized.len(), 3); // duplicates removed
        assert_eq!(normalized[0], "src-tauri/**"); // sorted alphabetically
        assert_eq!(normalized[1], "src/api/**");
        assert_eq!(normalized[2], "src/components/**");
    }

    #[test]
    fn test_derive_title_from_path() {
        let rewriter = IndexRewriter::new();

        assert_eq!(
            rewriter.derive_title_from_path(".claude/rules/task-state-machine.md"),
            "Task State Machine"
        );

        assert_eq!(
            rewriter.derive_title_from_path(".claude/rules/api_layer.md"),
            "Api Layer"
        );

        assert_eq!(
            rewriter.derive_title_from_path("stream-features.md"),
            "Stream Features"
        );
    }

    #[test]
    fn test_generate_index_content_empty() {
        let rewriter = IndexRewriter::new();

        let paths = vec!["src/**".to_string()];
        let memories: Vec<MemoryEntry> = vec![];

        let content = rewriter
            .generate_index_content("test.md", paths, &memories)
            .unwrap();

        assert!(content.contains("---\npaths:\n  - \"src/**\"\n---"));
        assert!(content.contains("## Summary\n\n(No memory entries ingested)"));
        assert!(content.contains("## Memory References\n\n(No memory entries)"));
        assert!(content.contains("## Retrieval"));
    }

    #[test]
    fn test_generate_index_content_with_memories() {
        let rewriter = IndexRewriter::new();

        let project_id = ProcessId::from_string("test-project");
        let memory1 = MemoryEntry::new(
            project_id.clone(),
            MemoryBucket::ArchitecturePatterns,
            "State Machine Pattern".to_string(),
            "State transitions must go through TransitionHandler".to_string(),
            "Details about state machine...".to_string(),
            vec!["src/domain/**".to_string()],
        );

        let memory2 = MemoryEntry::new(
            project_id,
            MemoryBucket::ImplementationDiscoveries,
            "Async Trait Gotcha".to_string(),
            "async_trait macro required for async methods in traits".to_string(),
            "Details about async traits...".to_string(),
            vec!["src/**".to_string()],
        );

        let paths = vec!["src/**".to_string()];
        let memories = vec![memory1, memory2];

        let content = rewriter
            .generate_index_content("test.md", paths, &memories)
            .unwrap();

        assert!(content.contains("State transitions must go through TransitionHandler"));
        assert!(content.contains("async_trait macro required"));
        assert!(content.contains("### architecture_patterns"));
        assert!(content.contains("### implementation_discoveries"));
        assert!(content.contains("`get_memories_for_paths`"));
    }

    #[test]
    fn test_group_memories_by_bucket() {
        let rewriter = IndexRewriter::new();

        let project_id = ProcessId::from_string("test-project");
        let memory1 = MemoryEntry::new(
            project_id.clone(),
            MemoryBucket::ArchitecturePatterns,
            "Pattern 1".to_string(),
            "Summary 1".to_string(),
            "Details 1".to_string(),
            vec![],
        );

        let memory2 = MemoryEntry::new(
            project_id.clone(),
            MemoryBucket::ArchitecturePatterns,
            "Pattern 2".to_string(),
            "Summary 2".to_string(),
            "Details 2".to_string(),
            vec![],
        );

        let memory3 = MemoryEntry::new(
            project_id,
            MemoryBucket::ImplementationDiscoveries,
            "Discovery 1".to_string(),
            "Summary 3".to_string(),
            "Details 3".to_string(),
            vec![],
        );

        let memories = vec![memory1, memory2, memory3];
        let grouped = rewriter.group_memories_by_bucket(&memories);

        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped[0].0, "architecture_patterns");
        assert_eq!(grouped[0].1.len(), 2);
        assert_eq!(grouped[1].0, "implementation_discoveries");
        assert_eq!(grouped[1].1.len(), 1);
    }
}
