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
        fs::write(&temp_path, content)
            .map_err(|e| AppError::Infrastructure(format!("Failed to write temp file: {}", e)))?;

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
#[path = "index_rewriter_tests.rs"]
mod tests;
