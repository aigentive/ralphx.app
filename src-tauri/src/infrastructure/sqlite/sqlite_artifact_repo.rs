// SQLite-based ArtifactRepository implementation for production use
// Uses rusqlite with connection pooling for thread-safe access

use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::Connection;

use crate::domain::entities::{
    Artifact, ArtifactBucketId, ArtifactContent, ArtifactId, ArtifactMetadata, ArtifactRelation,
    ArtifactRelationId, ArtifactRelationType, ArtifactType, ProcessId, TaskId,
};
use crate::domain::repositories::ArtifactRepository;
use crate::error::{AppError, AppResult};

/// SQLite implementation of ArtifactRepository for production use
/// Uses a mutex-protected connection for thread-safe access
pub struct SqliteArtifactRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteArtifactRepository {
    /// Create a new SQLite artifact repository with the given connection
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    /// Create from an Arc-wrapped mutex connection (for sharing)
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Parse an Artifact from a database row
    fn artifact_from_row(row: &rusqlite::Row<'_>) -> Result<Artifact, rusqlite::Error> {
        let id: String = row.get(0)?;
        let type_str: String = row.get(1)?;
        let name: String = row.get(2)?;
        let content_type: String = row.get(3)?;
        let content_text: Option<String> = row.get(4)?;
        let content_path: Option<String> = row.get(5)?;
        let bucket_id: Option<String> = row.get(6)?;
        let task_id: Option<String> = row.get(7)?;
        let process_id: Option<String> = row.get(8)?;
        let created_by: String = row.get(9)?;
        let version: i32 = row.get(10)?;
        let created_at_str: String = row.get(12)?;

        // Parse artifact type
        let artifact_type = ArtifactType::from_str(&type_str)
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

        // Parse content
        let content = match content_type.as_str() {
            "inline" => ArtifactContent::inline(content_text.unwrap_or_default()),
            "file" => ArtifactContent::file(content_path.unwrap_or_default()),
            _ => {
                return Err(rusqlite::Error::InvalidParameterName(format!(
                    "Unknown content type: {}",
                    content_type
                )))
            }
        };

        // Parse created_at
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        // Build metadata
        let metadata = ArtifactMetadata {
            created_at,
            created_by,
            task_id: task_id.map(|s| TaskId::from_string(s)),
            process_id: process_id.map(ProcessId::from_string),
            version: version as u32,
        };

        Ok(Artifact {
            id: ArtifactId::from_string(id),
            artifact_type,
            name,
            content,
            metadata,
            derived_from: vec![], // Loaded separately via relations
            bucket_id: bucket_id.map(ArtifactBucketId::from_string),
        })
    }

    /// Parse an ArtifactRelation from a database row
    fn relation_from_row(row: &rusqlite::Row<'_>) -> Result<ArtifactRelation, rusqlite::Error> {
        let id: String = row.get(0)?;
        let from_id: String = row.get(1)?;
        let to_id: String = row.get(2)?;
        let rel_type_str: String = row.get(3)?;

        let relation_type = ArtifactRelationType::from_str(&rel_type_str)
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

        Ok(ArtifactRelation {
            id: ArtifactRelationId::from_string(id),
            from_artifact_id: ArtifactId::from_string(from_id),
            to_artifact_id: ArtifactId::from_string(to_id),
            relation_type,
        })
    }
}

#[async_trait]
impl ArtifactRepository for SqliteArtifactRepository {
    async fn create(&self, artifact: Artifact) -> AppResult<Artifact> {
        let conn = self.conn.lock().await;

        let (content_type, content_text, content_path) = match &artifact.content {
            ArtifactContent::Inline { text } => ("inline", Some(text.clone()), None),
            ArtifactContent::File { path } => ("file", None, Some(path.clone())),
        };

        let created_at = artifact.metadata.created_at.to_rfc3339();

        conn.execute(
            "INSERT INTO artifacts (id, type, name, content_type, content_text, content_path,
             bucket_id, task_id, process_id, created_by, version, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            rusqlite::params![
                artifact.id.as_str(),
                artifact.artifact_type.as_str(),
                artifact.name,
                content_type,
                content_text,
                content_path,
                artifact.bucket_id.as_ref().map(|b| b.as_str()),
                artifact.metadata.task_id.as_ref().map(|t| t.as_str()),
                artifact.metadata.process_id.as_ref().map(|p| p.as_str()),
                artifact.metadata.created_by,
                artifact.metadata.version as i32,
                created_at,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(artifact)
    }

    async fn get_by_id(&self, id: &ArtifactId) -> AppResult<Option<Artifact>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT id, type, name, content_type, content_text, content_path,
                    bucket_id, task_id, process_id, created_by, version,
                    previous_version_id, created_at
             FROM artifacts WHERE id = ?1",
            [id.as_str()],
            |row| Self::artifact_from_row(row),
        );

        match result {
            Ok(artifact) => Ok(Some(artifact)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn get_by_bucket(&self, bucket_id: &ArtifactBucketId) -> AppResult<Vec<Artifact>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, type, name, content_type, content_text, content_path,
                        bucket_id, task_id, process_id, created_by, version,
                        previous_version_id, created_at
                 FROM artifacts WHERE bucket_id = ?1
                 ORDER BY created_at DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let artifacts = stmt
            .query_map([bucket_id.as_str()], |row| Self::artifact_from_row(row))
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(artifacts)
    }

    async fn get_by_type(&self, artifact_type: ArtifactType) -> AppResult<Vec<Artifact>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, type, name, content_type, content_text, content_path,
                        bucket_id, task_id, process_id, created_by, version,
                        previous_version_id, created_at
                 FROM artifacts WHERE type = ?1
                 ORDER BY created_at DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let artifacts = stmt
            .query_map([artifact_type.as_str()], |row| Self::artifact_from_row(row))
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(artifacts)
    }

    async fn get_by_task(&self, task_id: &TaskId) -> AppResult<Vec<Artifact>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, type, name, content_type, content_text, content_path,
                        bucket_id, task_id, process_id, created_by, version,
                        previous_version_id, created_at
                 FROM artifacts WHERE task_id = ?1
                 ORDER BY created_at DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let artifacts = stmt
            .query_map([task_id.as_str()], |row| Self::artifact_from_row(row))
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(artifacts)
    }

    async fn get_by_process(&self, process_id: &ProcessId) -> AppResult<Vec<Artifact>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, type, name, content_type, content_text, content_path,
                        bucket_id, task_id, process_id, created_by, version,
                        previous_version_id, created_at
                 FROM artifacts WHERE process_id = ?1
                 ORDER BY created_at DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let artifacts = stmt
            .query_map([process_id.as_str()], |row| Self::artifact_from_row(row))
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(artifacts)
    }

    async fn update(&self, artifact: &Artifact) -> AppResult<()> {
        let conn = self.conn.lock().await;

        let (content_type, content_text, content_path) = match &artifact.content {
            ArtifactContent::Inline { text } => ("inline", Some(text.clone()), None),
            ArtifactContent::File { path } => ("file", None, Some(path.clone())),
        };

        conn.execute(
            "UPDATE artifacts SET type = ?2, name = ?3, content_type = ?4,
             content_text = ?5, content_path = ?6, bucket_id = ?7, task_id = ?8,
             process_id = ?9, created_by = ?10, version = ?11
             WHERE id = ?1",
            rusqlite::params![
                artifact.id.as_str(),
                artifact.artifact_type.as_str(),
                artifact.name,
                content_type,
                content_text,
                content_path,
                artifact.bucket_id.as_ref().map(|b| b.as_str()),
                artifact.metadata.task_id.as_ref().map(|t| t.as_str()),
                artifact.metadata.process_id.as_ref().map(|p| p.as_str()),
                artifact.metadata.created_by,
                artifact.metadata.version as i32,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: &ArtifactId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute("DELETE FROM artifacts WHERE id = ?1", [id.as_str()])
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn get_derived_from(&self, artifact_id: &ArtifactId) -> AppResult<Vec<Artifact>> {
        let conn = self.conn.lock().await;

        // Get artifacts that this artifact was derived from (to_artifact_id in relations)
        let mut stmt = conn
            .prepare(
                "SELECT a.id, a.type, a.name, a.content_type, a.content_text, a.content_path,
                        a.bucket_id, a.task_id, a.process_id, a.created_by, a.version,
                        a.previous_version_id, a.created_at
                 FROM artifacts a
                 INNER JOIN artifact_relations r ON a.id = r.to_artifact_id
                 WHERE r.from_artifact_id = ?1 AND r.relation_type = 'derived_from'
                 ORDER BY a.created_at DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let artifacts = stmt
            .query_map([artifact_id.as_str()], |row| Self::artifact_from_row(row))
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(artifacts)
    }

    async fn get_related(&self, artifact_id: &ArtifactId) -> AppResult<Vec<Artifact>> {
        let conn = self.conn.lock().await;

        // Get all related artifacts (both directions for related_to type)
        let mut stmt = conn
            .prepare(
                "SELECT DISTINCT a.id, a.type, a.name, a.content_type, a.content_text,
                        a.content_path, a.bucket_id, a.task_id, a.process_id, a.created_by,
                        a.version, a.previous_version_id, a.created_at
                 FROM artifacts a
                 INNER JOIN artifact_relations r ON
                    (a.id = r.to_artifact_id AND r.from_artifact_id = ?1) OR
                    (a.id = r.from_artifact_id AND r.to_artifact_id = ?1)
                 WHERE r.relation_type = 'related_to' AND a.id != ?1
                 ORDER BY a.created_at DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let artifacts = stmt
            .query_map([artifact_id.as_str()], |row| Self::artifact_from_row(row))
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(artifacts)
    }

    async fn add_relation(&self, relation: ArtifactRelation) -> AppResult<ArtifactRelation> {
        let conn = self.conn.lock().await;

        conn.execute(
            "INSERT INTO artifact_relations (id, from_artifact_id, to_artifact_id, relation_type)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                relation.id.as_str(),
                relation.from_artifact_id.as_str(),
                relation.to_artifact_id.as_str(),
                relation.relation_type.as_str(),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(relation)
    }

    async fn get_relations(&self, artifact_id: &ArtifactId) -> AppResult<Vec<ArtifactRelation>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, from_artifact_id, to_artifact_id, relation_type
                 FROM artifact_relations
                 WHERE from_artifact_id = ?1 OR to_artifact_id = ?1
                 ORDER BY created_at DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let relations = stmt
            .query_map([artifact_id.as_str()], |row| Self::relation_from_row(row))
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(relations)
    }

    async fn get_relations_by_type(
        &self,
        artifact_id: &ArtifactId,
        relation_type: ArtifactRelationType,
    ) -> AppResult<Vec<ArtifactRelation>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, from_artifact_id, to_artifact_id, relation_type
                 FROM artifact_relations
                 WHERE (from_artifact_id = ?1 OR to_artifact_id = ?1)
                   AND relation_type = ?2
                 ORDER BY created_at DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let relations = stmt
            .query_map(
                rusqlite::params![artifact_id.as_str(), relation_type.as_str()],
                |row| Self::relation_from_row(row),
            )
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(relations)
    }

    async fn delete_relation(&self, from_id: &ArtifactId, to_id: &ArtifactId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "DELETE FROM artifact_relations
             WHERE from_artifact_id = ?1 AND to_artifact_id = ?2",
            rusqlite::params![from_id.as_str(), to_id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

    fn setup_test_db() -> Connection {
        let conn = open_memory_connection().expect("Failed to open memory connection");
        run_migrations(&conn).expect("Failed to run migrations");
        conn
    }

    fn create_test_artifact() -> Artifact {
        Artifact::new_inline("Test PRD", ArtifactType::Prd, "PRD content here", "user")
    }

    fn create_file_artifact() -> Artifact {
        Artifact::new_file(
            "Design Doc",
            ArtifactType::DesignDoc,
            "/docs/design.md",
            "architect",
        )
    }

    // ==================== CREATE TESTS ====================

    #[tokio::test]
    async fn test_create_artifact_inline() {
        let conn = setup_test_db();
        let repo = SqliteArtifactRepository::new(conn);
        let artifact = create_test_artifact();

        let result = repo.create(artifact.clone()).await;
        assert!(result.is_ok());

        let created = result.unwrap();
        assert_eq!(created.id, artifact.id);
        assert_eq!(created.name, "Test PRD");
    }

    #[tokio::test]
    async fn test_create_artifact_file() {
        let conn = setup_test_db();
        let repo = SqliteArtifactRepository::new(conn);
        let artifact = create_file_artifact();

        let result = repo.create(artifact.clone()).await;
        assert!(result.is_ok());

        let created = result.unwrap();
        assert!(created.content.is_file());
    }

    #[tokio::test]
    async fn test_create_artifact_with_bucket() {
        let conn = setup_test_db();
        let repo = SqliteArtifactRepository::new(conn);

        // First create a bucket (would normally be done via bucket repo)
        {
            let c = repo.conn.lock().await;
            c.execute(
                "INSERT INTO artifact_buckets (id, name, config_json, is_system)
                 VALUES ('prd-library', 'PRD Library', '{}', 1)",
                [],
            )
            .unwrap();
        }

        let artifact = create_test_artifact()
            .with_bucket(ArtifactBucketId::from_string("prd-library"));

        let result = repo.create(artifact.clone()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_artifact_with_task() {
        let conn = setup_test_db();
        let repo = SqliteArtifactRepository::new(conn);

        let task_id = TaskId::from_string("task-123".to_string());

        // Create a task first to satisfy foreign key constraint
        {
            let c = repo.conn.lock().await;
            c.execute(
                "INSERT INTO projects (id, name, working_directory, created_at, updated_at)
                 VALUES ('proj-1', 'Test Project', '/test', datetime('now'), datetime('now'))",
                [],
            )
            .unwrap();
            c.execute(
                "INSERT INTO tasks (id, project_id, title, category, internal_status, created_at, updated_at)
                 VALUES ('task-123', 'proj-1', 'Test Task', 'feature', 'backlog', datetime('now'), datetime('now'))",
                [],
            )
            .unwrap();
        }

        let artifact = create_test_artifact().with_task(task_id.clone());

        let result = repo.create(artifact).await;
        assert!(result.is_ok());
    }

    // ==================== GET BY ID TESTS ====================

    #[tokio::test]
    async fn test_get_by_id_found() {
        let conn = setup_test_db();
        let repo = SqliteArtifactRepository::new(conn);
        let artifact = create_test_artifact();

        repo.create(artifact.clone()).await.unwrap();

        let result = repo.get_by_id(&artifact.id).await;
        assert!(result.is_ok());

        let found = result.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Test PRD");
    }

    #[tokio::test]
    async fn test_get_by_id_not_found() {
        let conn = setup_test_db();
        let repo = SqliteArtifactRepository::new(conn);
        let id = ArtifactId::new();

        let result = repo.get_by_id(&id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_get_by_id_preserves_content_inline() {
        let conn = setup_test_db();
        let repo = SqliteArtifactRepository::new(conn);
        let artifact = create_test_artifact();

        repo.create(artifact.clone()).await.unwrap();

        let loaded = repo.get_by_id(&artifact.id).await.unwrap().unwrap();
        if let ArtifactContent::Inline { text } = &loaded.content {
            assert_eq!(text, "PRD content here");
        } else {
            panic!("Expected inline content");
        }
    }

    #[tokio::test]
    async fn test_get_by_id_preserves_content_file() {
        let conn = setup_test_db();
        let repo = SqliteArtifactRepository::new(conn);
        let artifact = create_file_artifact();

        repo.create(artifact.clone()).await.unwrap();

        let loaded = repo.get_by_id(&artifact.id).await.unwrap().unwrap();
        if let ArtifactContent::File { path } = &loaded.content {
            assert_eq!(path, "/docs/design.md");
        } else {
            panic!("Expected file content");
        }
    }

    // ==================== GET BY BUCKET TESTS ====================

    #[tokio::test]
    async fn test_get_by_bucket_empty() {
        let conn = setup_test_db();
        let repo = SqliteArtifactRepository::new(conn);
        let bucket_id = ArtifactBucketId::from_string("nonexistent");

        let result = repo.get_by_bucket(&bucket_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_by_bucket_returns_matching() {
        let conn = setup_test_db();
        let repo = SqliteArtifactRepository::new(conn);

        // Create bucket
        {
            let c = repo.conn.lock().await;
            c.execute(
                "INSERT INTO artifact_buckets (id, name, config_json, is_system)
                 VALUES ('prd-library', 'PRD Library', '{}', 1)",
                [],
            )
            .unwrap();
        }

        let bucket_id = ArtifactBucketId::from_string("prd-library");

        // Create artifacts in bucket
        let a1 = create_test_artifact().with_bucket(bucket_id.clone());
        let mut a2 = create_test_artifact();
        a2.id = ArtifactId::new();
        let a2 = a2.with_bucket(bucket_id.clone());

        // Create artifact not in bucket
        let a3 = create_test_artifact();

        repo.create(a1).await.unwrap();
        repo.create(a2).await.unwrap();
        repo.create(a3).await.unwrap();

        let result = repo.get_by_bucket(&bucket_id).await.unwrap();
        assert_eq!(result.len(), 2);
    }

    // ==================== GET BY TYPE TESTS ====================

    #[tokio::test]
    async fn test_get_by_type_empty() {
        let conn = setup_test_db();
        let repo = SqliteArtifactRepository::new(conn);

        let result = repo.get_by_type(ArtifactType::CodeChange).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_by_type_returns_matching() {
        let conn = setup_test_db();
        let repo = SqliteArtifactRepository::new(conn);

        // Create PRD artifacts
        let a1 = create_test_artifact();
        let mut a2 = create_test_artifact();
        a2.id = ArtifactId::new();

        // Create design doc artifact
        let a3 = create_file_artifact();

        repo.create(a1).await.unwrap();
        repo.create(a2).await.unwrap();
        repo.create(a3).await.unwrap();

        let prds = repo.get_by_type(ArtifactType::Prd).await.unwrap();
        assert_eq!(prds.len(), 2);

        let docs = repo.get_by_type(ArtifactType::DesignDoc).await.unwrap();
        assert_eq!(docs.len(), 1);
    }

    // ==================== GET BY TASK TESTS ====================

    #[tokio::test]
    async fn test_get_by_task_empty() {
        let conn = setup_test_db();
        let repo = SqliteArtifactRepository::new(conn);
        let task_id = TaskId::from_string("task-999".to_string());

        let result = repo.get_by_task(&task_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_by_task_returns_matching() {
        let conn = setup_test_db();
        let repo = SqliteArtifactRepository::new(conn);

        let task_id = TaskId::from_string("task-123".to_string());

        // Create a task first to satisfy foreign key constraint
        {
            let c = repo.conn.lock().await;
            c.execute(
                "INSERT INTO projects (id, name, working_directory, created_at, updated_at)
                 VALUES ('proj-1', 'Test Project', '/test', datetime('now'), datetime('now'))",
                [],
            )
            .unwrap();
            c.execute(
                "INSERT INTO tasks (id, project_id, title, category, internal_status, created_at, updated_at)
                 VALUES ('task-123', 'proj-1', 'Test Task', 'feature', 'backlog', datetime('now'), datetime('now'))",
                [],
            )
            .unwrap();
        }

        let a1 = create_test_artifact().with_task(task_id.clone());
        let mut a2 = create_test_artifact();
        a2.id = ArtifactId::new(); // Give it a different ID

        repo.create(a1).await.unwrap();
        repo.create(a2).await.unwrap();

        let result = repo.get_by_task(&task_id).await.unwrap();
        assert_eq!(result.len(), 1);
    }

    // ==================== GET BY PROCESS TESTS ====================

    #[tokio::test]
    async fn test_get_by_process_empty() {
        let conn = setup_test_db();
        let repo = SqliteArtifactRepository::new(conn);
        let process_id = ProcessId::from_string("process-999");

        let result = repo.get_by_process(&process_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_by_process_returns_matching() {
        let conn = setup_test_db();
        let repo = SqliteArtifactRepository::new(conn);

        let process_id = ProcessId::from_string("research-1");

        let a1 = create_test_artifact().with_process(process_id.clone());
        let a2 = create_test_artifact(); // No process

        repo.create(a1).await.unwrap();
        repo.create(a2).await.unwrap();

        let result = repo.get_by_process(&process_id).await.unwrap();
        assert_eq!(result.len(), 1);
    }

    // ==================== UPDATE TESTS ====================

    #[tokio::test]
    async fn test_update_artifact() {
        let conn = setup_test_db();
        let repo = SqliteArtifactRepository::new(conn);

        let mut artifact = create_test_artifact();
        repo.create(artifact.clone()).await.unwrap();

        artifact.name = "Updated Name".to_string();
        artifact.content = ArtifactContent::inline("Updated content");

        let result = repo.update(&artifact).await;
        assert!(result.is_ok());

        let updated = repo.get_by_id(&artifact.id).await.unwrap().unwrap();
        assert_eq!(updated.name, "Updated Name");
        if let ArtifactContent::Inline { text } = &updated.content {
            assert_eq!(text, "Updated content");
        }
    }

    // ==================== DELETE TESTS ====================

    #[tokio::test]
    async fn test_delete_artifact() {
        let conn = setup_test_db();
        let repo = SqliteArtifactRepository::new(conn);

        let artifact = create_test_artifact();
        repo.create(artifact.clone()).await.unwrap();

        let result = repo.delete(&artifact.id).await;
        assert!(result.is_ok());

        let found = repo.get_by_id(&artifact.id).await.unwrap();
        assert!(found.is_none());
    }

    // ==================== RELATION TESTS ====================

    #[tokio::test]
    async fn test_add_relation_derived_from() {
        let conn = setup_test_db();
        let repo = SqliteArtifactRepository::new(conn);

        let parent = create_test_artifact();
        let mut child = create_test_artifact();
        child.id = ArtifactId::new();

        repo.create(parent.clone()).await.unwrap();
        repo.create(child.clone()).await.unwrap();

        let relation = ArtifactRelation::derived_from(child.id.clone(), parent.id.clone());
        let result = repo.add_relation(relation.clone()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_relation_related_to() {
        let conn = setup_test_db();
        let repo = SqliteArtifactRepository::new(conn);

        let a1 = create_test_artifact();
        let mut a2 = create_test_artifact();
        a2.id = ArtifactId::new();

        repo.create(a1.clone()).await.unwrap();
        repo.create(a2.clone()).await.unwrap();

        let relation = ArtifactRelation::related_to(a1.id.clone(), a2.id.clone());
        let result = repo.add_relation(relation).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_derived_from() {
        let conn = setup_test_db();
        let repo = SqliteArtifactRepository::new(conn);

        let parent1 = create_test_artifact();
        let mut parent2 = create_test_artifact();
        parent2.id = ArtifactId::new();
        let mut child = create_test_artifact();
        child.id = ArtifactId::new();

        repo.create(parent1.clone()).await.unwrap();
        repo.create(parent2.clone()).await.unwrap();
        repo.create(child.clone()).await.unwrap();

        // Child derived from both parents
        repo.add_relation(ArtifactRelation::derived_from(
            child.id.clone(),
            parent1.id.clone(),
        ))
        .await
        .unwrap();
        repo.add_relation(ArtifactRelation::derived_from(
            child.id.clone(),
            parent2.id.clone(),
        ))
        .await
        .unwrap();

        let parents = repo.get_derived_from(&child.id).await.unwrap();
        assert_eq!(parents.len(), 2);
    }

    #[tokio::test]
    async fn test_get_related() {
        let conn = setup_test_db();
        let repo = SqliteArtifactRepository::new(conn);

        let a1 = create_test_artifact();
        let mut a2 = create_test_artifact();
        a2.id = ArtifactId::new();
        let mut a3 = create_test_artifact();
        a3.id = ArtifactId::new();

        repo.create(a1.clone()).await.unwrap();
        repo.create(a2.clone()).await.unwrap();
        repo.create(a3.clone()).await.unwrap();

        // a1 related to a2 and a3
        repo.add_relation(ArtifactRelation::related_to(a1.id.clone(), a2.id.clone()))
            .await
            .unwrap();
        repo.add_relation(ArtifactRelation::related_to(a3.id.clone(), a1.id.clone()))
            .await
            .unwrap();

        let related = repo.get_related(&a1.id).await.unwrap();
        assert_eq!(related.len(), 2);
    }

    #[tokio::test]
    async fn test_get_relations() {
        let conn = setup_test_db();
        let repo = SqliteArtifactRepository::new(conn);

        let a1 = create_test_artifact();
        let mut a2 = create_test_artifact();
        a2.id = ArtifactId::new();

        repo.create(a1.clone()).await.unwrap();
        repo.create(a2.clone()).await.unwrap();

        repo.add_relation(ArtifactRelation::derived_from(a2.id.clone(), a1.id.clone()))
            .await
            .unwrap();

        let relations = repo.get_relations(&a1.id).await.unwrap();
        assert_eq!(relations.len(), 1);
    }

    #[tokio::test]
    async fn test_get_relations_by_type() {
        let conn = setup_test_db();
        let repo = SqliteArtifactRepository::new(conn);

        let a1 = create_test_artifact();
        let mut a2 = create_test_artifact();
        a2.id = ArtifactId::new();
        let mut a3 = create_test_artifact();
        a3.id = ArtifactId::new();

        repo.create(a1.clone()).await.unwrap();
        repo.create(a2.clone()).await.unwrap();
        repo.create(a3.clone()).await.unwrap();

        // Different relation types
        repo.add_relation(ArtifactRelation::derived_from(a2.id.clone(), a1.id.clone()))
            .await
            .unwrap();
        repo.add_relation(ArtifactRelation::related_to(a1.id.clone(), a3.id.clone()))
            .await
            .unwrap();

        let derived = repo
            .get_relations_by_type(&a1.id, ArtifactRelationType::DerivedFrom)
            .await
            .unwrap();
        assert_eq!(derived.len(), 1);

        let related = repo
            .get_relations_by_type(&a1.id, ArtifactRelationType::RelatedTo)
            .await
            .unwrap();
        assert_eq!(related.len(), 1);
    }

    #[tokio::test]
    async fn test_delete_relation() {
        let conn = setup_test_db();
        let repo = SqliteArtifactRepository::new(conn);

        let a1 = create_test_artifact();
        let mut a2 = create_test_artifact();
        a2.id = ArtifactId::new();

        repo.create(a1.clone()).await.unwrap();
        repo.create(a2.clone()).await.unwrap();

        repo.add_relation(ArtifactRelation::related_to(a1.id.clone(), a2.id.clone()))
            .await
            .unwrap();

        // Verify relation exists
        let relations = repo.get_relations(&a1.id).await.unwrap();
        assert_eq!(relations.len(), 1);

        // Delete relation
        repo.delete_relation(&a1.id, &a2.id).await.unwrap();

        // Verify relation deleted
        let relations = repo.get_relations(&a1.id).await.unwrap();
        assert!(relations.is_empty());
    }

    // ==================== SHARED CONNECTION TESTS ====================

    #[tokio::test]
    async fn test_from_shared_connection() {
        let conn = setup_test_db();
        let shared = Arc::new(Mutex::new(conn));

        let repo1 = SqliteArtifactRepository::from_shared(shared.clone());
        let repo2 = SqliteArtifactRepository::from_shared(shared.clone());

        // Create via repo1
        let artifact = create_test_artifact();
        repo1.create(artifact.clone()).await.unwrap();

        // Read via repo2
        let found = repo2.get_by_id(&artifact.id).await.unwrap();
        assert!(found.is_some());
    }
}
