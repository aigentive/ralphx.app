// Newtype pattern for type-safe IDs
// Prevents mixing up TaskId and ProjectId at compile time

use serde::{Deserialize, Serialize};

/// A unique identifier for a Task
/// Uses newtype pattern to prevent accidentally using a ProjectId where TaskId is expected
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub String);

/// A unique identifier for a Project
/// Uses newtype pattern to prevent accidentally using a TaskId where ProjectId is expected
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectId(pub String);

impl TaskId {
    /// Creates a new TaskId with a random UUID v4
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Creates a TaskId from an existing string
    /// Useful for database deserialization
    pub fn from_string(s: String) -> Self {
        Self(s)
    }

    /// Returns the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ProjectId {
    /// Creates a new ProjectId with a random UUID v4
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Creates a ProjectId from an existing string
    /// Useful for database deserialization
    pub fn from_string(s: String) -> Self {
        Self(s)
    }

    /// Returns the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ProjectId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ProjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    // ===== TaskId Tests =====

    #[test]
    fn task_id_new_generates_valid_uuid() {
        let id = TaskId::new();
        // UUID v4 format: 8-4-4-4-12 hex chars with hyphens
        assert_eq!(id.as_str().len(), 36);
        assert!(id.as_str().chars().filter(|c| *c == '-').count() == 4);
        // Verify it parses as valid UUID
        assert!(uuid::Uuid::parse_str(id.as_str()).is_ok());
    }

    #[test]
    fn task_id_new_generates_unique_ids() {
        let ids: HashSet<String> = (0..100).map(|_| TaskId::new().0).collect();
        assert_eq!(ids.len(), 100, "All generated TaskIds should be unique");
    }

    #[test]
    fn task_id_from_string_preserves_value() {
        let original = "my-custom-id".to_string();
        let id = TaskId::from_string(original.clone());
        assert_eq!(id.as_str(), "my-custom-id");
        assert_eq!(id.0, original);
    }

    #[test]
    fn task_id_equality_works() {
        let id1 = TaskId::from_string("abc".to_string());
        let id2 = TaskId::from_string("abc".to_string());
        let id3 = TaskId::from_string("xyz".to_string());

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn task_id_clone_works() {
        let id1 = TaskId::new();
        let id2 = id1.clone();
        assert_eq!(id1, id2);
    }

    #[test]
    fn task_id_hash_works() {
        let id = TaskId::from_string("test".to_string());
        let mut set = HashSet::new();
        set.insert(id.clone());
        assert!(set.contains(&id));
    }

    #[test]
    fn task_id_display_works() {
        let id = TaskId::from_string("display-test".to_string());
        assert_eq!(format!("{}", id), "display-test");
    }

    #[test]
    fn task_id_debug_works() {
        let id = TaskId::from_string("debug-test".to_string());
        let debug_str = format!("{:?}", id);
        assert!(debug_str.contains("debug-test"));
    }

    #[test]
    fn task_id_serializes_to_json() {
        let id = TaskId::from_string("serialize-test".to_string());
        let json = serde_json::to_string(&id).expect("Should serialize");
        assert_eq!(json, "\"serialize-test\"");
    }

    #[test]
    fn task_id_deserializes_from_json() {
        let json = "\"deserialize-test\"";
        let id: TaskId = serde_json::from_str(json).expect("Should deserialize");
        assert_eq!(id.as_str(), "deserialize-test");
    }

    #[test]
    fn task_id_default_creates_new() {
        let id = TaskId::default();
        assert!(uuid::Uuid::parse_str(id.as_str()).is_ok());
    }

    // ===== ProjectId Tests =====

    #[test]
    fn project_id_new_generates_valid_uuid() {
        let id = ProjectId::new();
        assert_eq!(id.as_str().len(), 36);
        assert!(id.as_str().chars().filter(|c| *c == '-').count() == 4);
        assert!(uuid::Uuid::parse_str(id.as_str()).is_ok());
    }

    #[test]
    fn project_id_new_generates_unique_ids() {
        let ids: HashSet<String> = (0..100).map(|_| ProjectId::new().0).collect();
        assert_eq!(ids.len(), 100, "All generated ProjectIds should be unique");
    }

    #[test]
    fn project_id_from_string_preserves_value() {
        let original = "project-custom-id".to_string();
        let id = ProjectId::from_string(original.clone());
        assert_eq!(id.as_str(), "project-custom-id");
        assert_eq!(id.0, original);
    }

    #[test]
    fn project_id_equality_works() {
        let id1 = ProjectId::from_string("proj-abc".to_string());
        let id2 = ProjectId::from_string("proj-abc".to_string());
        let id3 = ProjectId::from_string("proj-xyz".to_string());

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn project_id_clone_works() {
        let id1 = ProjectId::new();
        let id2 = id1.clone();
        assert_eq!(id1, id2);
    }

    #[test]
    fn project_id_hash_works() {
        let id = ProjectId::from_string("proj-test".to_string());
        let mut set = HashSet::new();
        set.insert(id.clone());
        assert!(set.contains(&id));
    }

    #[test]
    fn project_id_display_works() {
        let id = ProjectId::from_string("proj-display".to_string());
        assert_eq!(format!("{}", id), "proj-display");
    }

    #[test]
    fn project_id_debug_works() {
        let id = ProjectId::from_string("proj-debug".to_string());
        let debug_str = format!("{:?}", id);
        assert!(debug_str.contains("proj-debug"));
    }

    #[test]
    fn project_id_serializes_to_json() {
        let id = ProjectId::from_string("proj-serialize".to_string());
        let json = serde_json::to_string(&id).expect("Should serialize");
        assert_eq!(json, "\"proj-serialize\"");
    }

    #[test]
    fn project_id_deserializes_from_json() {
        let json = "\"proj-deserialize\"";
        let id: ProjectId = serde_json::from_str(json).expect("Should deserialize");
        assert_eq!(id.as_str(), "proj-deserialize");
    }

    #[test]
    fn project_id_default_creates_new() {
        let id = ProjectId::default();
        assert!(uuid::Uuid::parse_str(id.as_str()).is_ok());
    }

    // ===== Type Safety Tests =====

    #[test]
    fn task_id_and_project_id_are_different_types() {
        // This test verifies compile-time type safety
        // The newtype pattern ensures we can't mix up TaskId and ProjectId
        let task_id = TaskId::from_string("same-value".to_string());
        let project_id = ProjectId::from_string("same-value".to_string());

        // They have the same inner value but are different types
        assert_eq!(task_id.as_str(), project_id.as_str());

        // This function only accepts TaskId
        fn use_task_id(id: &TaskId) -> &str {
            id.as_str()
        }

        // This function only accepts ProjectId
        fn use_project_id(id: &ProjectId) -> &str {
            id.as_str()
        }

        // These compile and work
        assert_eq!(use_task_id(&task_id), "same-value");
        assert_eq!(use_project_id(&project_id), "same-value");

        // But you can't pass TaskId to use_project_id or vice versa (compile error)
        // use_task_id(&project_id); // Would fail to compile
        // use_project_id(&task_id); // Would fail to compile
    }
}
