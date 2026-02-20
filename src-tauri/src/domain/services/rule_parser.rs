// Rule parser service for extracting YAML frontmatter and parsing markdown content

use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// YAML frontmatter extracted from rule file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleFrontmatter {
    /// Glob patterns for path scoping (from `paths:` key in YAML)
    #[serde(default)]
    pub paths: Vec<String>,
}

/// A semantic chunk extracted from markdown content
#[derive(Debug, Clone)]
pub struct MarkdownChunk {
    /// Header/title of the chunk (e.g., "## Migration Strategy")
    pub title: String,
    /// Full markdown content of this chunk
    pub content: String,
    /// Header level (1 = #, 2 = ##, etc.)
    pub level: usize,
}

/// Parsed rule file
#[derive(Debug, Clone)]
pub struct ParsedRuleFile {
    /// Frontmatter (paths globs)
    pub frontmatter: RuleFrontmatter,
    /// Semantic chunks (sections)
    pub chunks: Vec<MarkdownChunk>,
    /// Raw markdown content (without frontmatter)
    pub raw_content: String,
}

/// Rule parser for extracting frontmatter and chunking markdown
pub struct RuleParser;

impl RuleParser {
    /// Parse a rule file from disk
    pub fn parse_file(file_path: impl AsRef<Path>) -> AppResult<ParsedRuleFile> {
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| AppError::Infrastructure(format!("Failed to read rule file: {}", e)))?;

        Self::parse_content(&content)
    }

    /// Parse rule content from a string
    pub fn parse_content(content: &str) -> AppResult<ParsedRuleFile> {
        // Extract frontmatter if present
        let (frontmatter, raw_content) = Self::extract_frontmatter(content)?;

        // Parse markdown into semantic chunks
        let chunks = Self::chunk_markdown(&raw_content);

        Ok(ParsedRuleFile {
            frontmatter,
            chunks,
            raw_content,
        })
    }

    /// Extract YAML frontmatter from markdown content
    fn extract_frontmatter(content: &str) -> AppResult<(RuleFrontmatter, String)> {
        // Check if content starts with ---
        if !content.trim_start().starts_with("---") {
            // No frontmatter, return empty frontmatter and full content
            return Ok((RuleFrontmatter { paths: vec![] }, content.to_string()));
        }

        // Find the closing ---
        let lines: Vec<&str> = content.lines().collect();
        let mut frontmatter_end = 0;
        let mut found_closing = false;

        for (i, line) in lines.iter().enumerate().skip(1) {
            if line.trim() == "---" {
                frontmatter_end = i;
                found_closing = true;
                break;
            }
        }

        if !found_closing {
            // Malformed frontmatter, treat as no frontmatter
            return Ok((RuleFrontmatter { paths: vec![] }, content.to_string()));
        }

        // Extract frontmatter YAML
        let frontmatter_yaml = lines[1..frontmatter_end].join("\n");

        // Parse YAML
        let frontmatter: RuleFrontmatter = serde_yaml::from_str(&frontmatter_yaml)
            .unwrap_or_else(|_| RuleFrontmatter { paths: vec![] });

        // Extract remaining content (after frontmatter)
        let remaining_content = lines[frontmatter_end + 1..].join("\n");

        Ok((frontmatter, remaining_content))
    }

    /// Parse markdown into semantic chunks based on headers
    fn chunk_markdown(content: &str) -> Vec<MarkdownChunk> {
        let mut chunks = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        let mut current_title = String::from("Introduction");
        let mut current_level = 1;
        let mut current_content = Vec::new();
        let mut in_chunk = false;

        for line in lines {
            // Check if line is a header
            if let Some(header_info) = Self::parse_header(line) {
                // Save previous chunk if exists
                if in_chunk {
                    chunks.push(MarkdownChunk {
                        title: current_title.clone(),
                        content: current_content.join("\n"),
                        level: current_level,
                    });
                }

                // Start new chunk
                current_title = header_info.0;
                current_level = header_info.1;
                current_content = vec![line.to_string()];
                in_chunk = true;
            } else {
                // Add line to current chunk
                if in_chunk {
                    current_content.push(line.to_string());
                } else {
                    // Content before first header (introduction)
                    current_content.push(line.to_string());
                    in_chunk = true;
                }
            }
        }

        // Save final chunk
        if in_chunk && !current_content.is_empty() {
            chunks.push(MarkdownChunk {
                title: current_title,
                content: current_content.join("\n"),
                level: current_level,
            });
        }

        chunks
    }

    /// Parse a markdown header line, returning (title, level) if it's a header
    fn parse_header(line: &str) -> Option<(String, usize)> {
        let trimmed = line.trim_start();

        // Count leading # symbols
        let hash_count = trimmed.chars().take_while(|&c| c == '#').count();

        if hash_count == 0 || hash_count > 6 {
            return None;
        }

        // Extract title (text after the # symbols)
        let title = trimmed[hash_count..].trim_start().trim_end().to_string();

        if title.is_empty() {
            return None;
        }

        Some((title, hash_count))
    }
}

#[cfg(test)]
#[path = "rule_parser_tests.rs"]
mod tests;
