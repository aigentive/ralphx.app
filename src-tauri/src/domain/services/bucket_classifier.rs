// Bucket classifier for categorizing memory chunks

use crate::domain::entities::MemoryBucket;

/// Classifier for determining memory bucket based on content
pub struct BucketClassifier;

impl BucketClassifier {
    /// Classify a markdown chunk into a memory bucket
    ///
    /// This uses heuristics based on keywords and patterns to determine
    /// which bucket the content belongs to:
    /// - architecture_patterns: subsystem relationships, state-machine behavior, invariant rules
    /// - implementation_discoveries: non-obvious code-level findings, framework quirks
    /// - operational_playbooks: reproducible procedures, diagnostics, recovery tactics
    pub fn classify(title: &str, content: &str) -> MemoryBucket {
        let combined = format!("{} {}", title.to_lowercase(), content.to_lowercase());

        // Architecture patterns keywords
        let architecture_score = Self::count_keywords(
            &combined,
            &[
                "state machine",
                "state transition",
                "architecture",
                "subsystem",
                "component",
                "layer",
                "repository",
                "domain",
                "entity",
                "service",
                "invariant",
                "constraint",
                "relationship",
                "dependency",
                "flow",
                "pipeline",
                "handler",
                "middleware",
                "pattern",
                "design",
                "structure",
                "schema",
                "lifecycle",
                "event",
                "message",
                "data model",
            ],
        );

        // Implementation discoveries keywords
        let implementation_score = Self::count_keywords(
            &combined,
            &[
                "gotcha",
                "quirk",
                "workaround",
                "bug",
                "fix",
                "issue",
                "problem",
                "error",
                "warning",
                "migration",
                "upgrade",
                "compatibility",
                "breaking change",
                "deprecated",
                "implementation",
                "code",
                "function",
                "method",
                "api",
                "library",
                "framework",
                "crate",
                "package",
                "module",
                "import",
                "dependency",
            ],
        );

        // Operational playbooks keywords
        let operational_score = Self::count_keywords(
            &combined,
            &[
                "procedure",
                "step",
                "how to",
                "guide",
                "playbook",
                "diagnostic",
                "debug",
                "troubleshoot",
                "recovery",
                "restore",
                "backup",
                "rollback",
                "deploy",
                "setup",
                "configuration",
                "install",
                "run",
                "execute",
                "test",
                "verify",
                "check",
                "monitor",
                "observe",
                "incident",
                "alert",
            ],
        );

        // Return bucket with highest score
        if operational_score > architecture_score && operational_score > implementation_score {
            MemoryBucket::OperationalPlaybooks
        } else if implementation_score > architecture_score {
            MemoryBucket::ImplementationDiscoveries
        } else {
            // Default to architecture patterns
            MemoryBucket::ArchitecturePatterns
        }
    }

    /// Count occurrences of keywords in content
    fn count_keywords(content: &str, keywords: &[&str]) -> usize {
        keywords
            .iter()
            .filter(|keyword| content.contains(*keyword))
            .count()
    }
}

#[cfg(test)]
#[path = "bucket_classifier_tests.rs"]
mod tests;
