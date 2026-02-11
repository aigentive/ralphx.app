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
mod tests {
    use super::*;

    #[test]
    fn test_classify_architecture_pattern() {
        let title = "State Machine Design";
        let content = "The task state machine handles transitions between states. Each state has specific invariants and constraints.";

        let bucket = BucketClassifier::classify(title, content);
        assert_eq!(bucket, MemoryBucket::ArchitecturePatterns);
    }

    #[test]
    fn test_classify_implementation_discovery() {
        let title = "Async Bug Fix";
        let content = "Found a gotcha with tokio async runtime. The issue was caused by a breaking change in the library upgrade.";

        let bucket = BucketClassifier::classify(title, content);
        assert_eq!(bucket, MemoryBucket::ImplementationDiscoveries);
    }

    #[test]
    fn test_classify_operational_playbook() {
        let title = "Deployment Procedure";
        let content = "Step-by-step guide for deployment: 1. Backup database 2. Run migrations 3. Deploy new version 4. Verify health checks.";

        let bucket = BucketClassifier::classify(title, content);
        assert_eq!(bucket, MemoryBucket::OperationalPlaybooks);
    }

    #[test]
    fn test_default_to_architecture() {
        let title = "Random Content";
        let content = "This is some random content without specific keywords.";

        let bucket = BucketClassifier::classify(title, content);
        // Should default to architecture patterns
        assert_eq!(bucket, MemoryBucket::ArchitecturePatterns);
    }

    #[test]
    fn test_case_insensitive() {
        let title = "STATE MACHINE";
        let content = "THE STATE MACHINE HANDLES TRANSITIONS.";

        let bucket = BucketClassifier::classify(title, content);
        assert_eq!(bucket, MemoryBucket::ArchitecturePatterns);
    }
}
