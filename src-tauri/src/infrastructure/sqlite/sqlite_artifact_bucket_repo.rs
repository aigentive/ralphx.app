// SQLite-based ArtifactBucketRepository implementation for production use
// Uses rusqlite with connection pooling for thread-safe access

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use rusqlite::Connection;

use crate::domain::entities::{ArtifactBucket, ArtifactBucketId, ArtifactType};
use crate::domain::repositories::ArtifactBucketRepository;
use crate::error::{AppError, AppResult};

/// Configuration stored in config_json column
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct BucketConfig {
    accepted_types: Vec<String>,
    writers: Vec<String>,
    readers: Vec<String>,
}

/// SQLite implementation of ArtifactBucketRepository for production use
/// Uses a mutex-protected connection for thread-safe access
pub struct SqliteArtifactBucketRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteArtifactBucketRepository {
    /// Create a new SQLite artifact bucket repository with the given connection
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    /// Create from an Arc-wrapped mutex connection (for sharing)
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Parse an ArtifactBucket from a database row
    fn bucket_from_row(row: &rusqlite::Row<'_>) -> Result<ArtifactBucket, rusqlite::Error> {
        let id: String = row.get(0)?;
        let name: String = row.get(1)?;
        let config_json: String = row.get(2)?;
        let is_system: i32 = row.get(3)?;

        // Parse config JSON
        let config: BucketConfig = serde_json::from_str(&config_json)
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

        // Parse accepted types
        let accepted_types: Vec<ArtifactType> = config
            .accepted_types
            .iter()
            .filter_map(|s| s.parse().ok())
            .collect();

        Ok(ArtifactBucket {
            id: ArtifactBucketId::from_string(id),
            name,
            accepted_types,
            writers: config.writers,
            readers: config.readers,
            is_system: is_system != 0,
        })
    }

    /// Convert bucket to config JSON for storage
    fn bucket_to_config_json(bucket: &ArtifactBucket) -> AppResult<String> {
        let config = BucketConfig {
            accepted_types: bucket.accepted_types.iter().map(|t| t.as_str().to_string()).collect(),
            writers: bucket.writers.clone(),
            readers: bucket.readers.clone(),
        };
        serde_json::to_string(&config)
            .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))
    }

    /// Seeds the 4 built-in system buckets if they don't exist.
    /// Returns the number of buckets seeded.
    pub async fn seed_builtin_buckets(&self) -> AppResult<usize> {
        let system_buckets = ArtifactBucket::system_buckets();
        let mut seeded_count = 0;

        for bucket in system_buckets {
            // Check if bucket already exists
            if !self.exists(&bucket.id).await? {
                self.create(bucket).await?;
                seeded_count += 1;
            }
        }

        Ok(seeded_count)
    }
}

#[async_trait]
impl ArtifactBucketRepository for SqliteArtifactBucketRepository {
    async fn create(&self, bucket: ArtifactBucket) -> AppResult<ArtifactBucket> {
        let conn = self.conn.lock().await;

        let config_json = Self::bucket_to_config_json(&bucket)?;

        conn.execute(
            "INSERT INTO artifact_buckets (id, name, config_json, is_system)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                bucket.id.as_str(),
                bucket.name,
                config_json,
                if bucket.is_system { 1 } else { 0 },
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(bucket)
    }

    async fn get_by_id(&self, id: &ArtifactBucketId) -> AppResult<Option<ArtifactBucket>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT id, name, config_json, is_system
             FROM artifact_buckets WHERE id = ?1",
            [id.as_str()],
            |row| Self::bucket_from_row(row),
        );

        match result {
            Ok(bucket) => Ok(Some(bucket)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn get_all(&self) -> AppResult<Vec<ArtifactBucket>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, name, config_json, is_system
                 FROM artifact_buckets ORDER BY name ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let buckets = stmt
            .query_map([], |row| Self::bucket_from_row(row))
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(buckets)
    }

    async fn get_system_buckets(&self) -> AppResult<Vec<ArtifactBucket>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, name, config_json, is_system
                 FROM artifact_buckets WHERE is_system = 1 ORDER BY name ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let buckets = stmt
            .query_map([], |row| Self::bucket_from_row(row))
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(buckets)
    }

    async fn update(&self, bucket: &ArtifactBucket) -> AppResult<()> {
        let conn = self.conn.lock().await;

        let config_json = Self::bucket_to_config_json(bucket)?;

        conn.execute(
            "UPDATE artifact_buckets SET name = ?2, config_json = ?3, is_system = ?4
             WHERE id = ?1",
            rusqlite::params![
                bucket.id.as_str(),
                bucket.name,
                config_json,
                if bucket.is_system { 1 } else { 0 },
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: &ArtifactBucketId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        // Check if it's a system bucket
        let is_system: Option<i32> = conn
            .query_row(
                "SELECT is_system FROM artifact_buckets WHERE id = ?1",
                [id.as_str()],
                |row| row.get(0),
            )
            .ok();

        if is_system == Some(1) {
            return Err(AppError::Validation(
                "Cannot delete system bucket".to_string(),
            ));
        }

        conn.execute("DELETE FROM artifact_buckets WHERE id = ?1", [id.as_str()])
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn exists(&self, id: &ArtifactBucketId) -> AppResult<bool> {
        let conn = self.conn.lock().await;

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM artifact_buckets WHERE id = ?1",
                [id.as_str()],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count > 0)
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

    fn create_test_bucket() -> ArtifactBucket {
        ArtifactBucket::new("Test Bucket")
            .accepts(ArtifactType::Prd)
            .accepts(ArtifactType::DesignDoc)
            .with_writer("user")
            .with_writer("orchestrator")
    }

    fn create_system_bucket() -> ArtifactBucket {
        ArtifactBucket::system("test-system-bucket", "Test System Bucket")
            .accepts(ArtifactType::ResearchDocument)
            .with_writer("researcher")
    }

    // ==================== CREATE TESTS ====================

    #[tokio::test]
    async fn test_create_bucket() {
        let conn = setup_test_db();
        let repo = SqliteArtifactBucketRepository::new(conn);
        let bucket = create_test_bucket();

        let result = repo.create(bucket.clone()).await;
        assert!(result.is_ok());

        let created = result.unwrap();
        assert_eq!(created.id, bucket.id);
        assert_eq!(created.name, "Test Bucket");
    }

    #[tokio::test]
    async fn test_create_system_bucket() {
        let conn = setup_test_db();
        let repo = SqliteArtifactBucketRepository::new(conn);
        let bucket = create_system_bucket();

        let result = repo.create(bucket.clone()).await;
        assert!(result.is_ok());

        let created = result.unwrap();
        assert!(created.is_system);
    }

    // ==================== GET BY ID TESTS ====================

    #[tokio::test]
    async fn test_get_by_id_found() {
        let conn = setup_test_db();
        let repo = SqliteArtifactBucketRepository::new(conn);
        let bucket = create_test_bucket();

        repo.create(bucket.clone()).await.unwrap();

        let result = repo.get_by_id(&bucket.id).await;
        assert!(result.is_ok());

        let found = result.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Test Bucket");
    }

    #[tokio::test]
    async fn test_get_by_id_not_found() {
        let conn = setup_test_db();
        let repo = SqliteArtifactBucketRepository::new(conn);
        let id = ArtifactBucketId::new();

        let result = repo.get_by_id(&id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_get_by_id_preserves_accepted_types() {
        let conn = setup_test_db();
        let repo = SqliteArtifactBucketRepository::new(conn);
        let bucket = create_test_bucket();

        repo.create(bucket.clone()).await.unwrap();

        let loaded = repo.get_by_id(&bucket.id).await.unwrap().unwrap();
        assert!(loaded.accepts_type(ArtifactType::Prd));
        assert!(loaded.accepts_type(ArtifactType::DesignDoc));
        assert!(!loaded.accepts_type(ArtifactType::CodeChange));
    }

    #[tokio::test]
    async fn test_get_by_id_preserves_writers() {
        let conn = setup_test_db();
        let repo = SqliteArtifactBucketRepository::new(conn);
        let bucket = create_test_bucket();

        repo.create(bucket.clone()).await.unwrap();

        let loaded = repo.get_by_id(&bucket.id).await.unwrap().unwrap();
        assert!(loaded.can_write("user"));
        assert!(loaded.can_write("orchestrator"));
        assert!(!loaded.can_write("worker"));
    }

    // ==================== GET ALL TESTS ====================

    #[tokio::test]
    async fn test_get_all_empty() {
        let conn = setup_test_db();
        let repo = SqliteArtifactBucketRepository::new(conn);

        let result = repo.get_all().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_all_with_buckets() {
        let conn = setup_test_db();
        let repo = SqliteArtifactBucketRepository::new(conn);

        let bucket1 = create_test_bucket();
        let bucket2 = create_system_bucket();

        repo.create(bucket1).await.unwrap();
        repo.create(bucket2).await.unwrap();

        let result = repo.get_all().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_get_all_returns_sorted_by_name() {
        let conn = setup_test_db();
        let repo = SqliteArtifactBucketRepository::new(conn);

        let mut bucket1 = create_test_bucket();
        bucket1.name = "Zebra Bucket".to_string();

        let mut bucket2 = create_test_bucket();
        bucket2.id = ArtifactBucketId::new();
        bucket2.name = "Alpha Bucket".to_string();

        repo.create(bucket1).await.unwrap();
        repo.create(bucket2).await.unwrap();

        let result = repo.get_all().await.unwrap();
        assert_eq!(result[0].name, "Alpha Bucket");
        assert_eq!(result[1].name, "Zebra Bucket");
    }

    // ==================== GET SYSTEM BUCKETS TESTS ====================

    #[tokio::test]
    async fn test_get_system_buckets_empty() {
        let conn = setup_test_db();
        let repo = SqliteArtifactBucketRepository::new(conn);

        // Create only non-system bucket
        let bucket = create_test_bucket();
        repo.create(bucket).await.unwrap();

        let result = repo.get_system_buckets().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_system_buckets_returns_only_system() {
        let conn = setup_test_db();
        let repo = SqliteArtifactBucketRepository::new(conn);

        let custom = create_test_bucket();
        let system = create_system_bucket();

        repo.create(custom).await.unwrap();
        repo.create(system.clone()).await.unwrap();

        let result = repo.get_system_buckets().await;
        assert!(result.is_ok());

        let buckets = result.unwrap();
        assert_eq!(buckets.len(), 1);
        assert!(buckets[0].is_system);
        assert_eq!(buckets[0].id, system.id);
    }

    // ==================== UPDATE TESTS ====================

    #[tokio::test]
    async fn test_update_bucket() {
        let conn = setup_test_db();
        let repo = SqliteArtifactBucketRepository::new(conn);

        let mut bucket = create_test_bucket();
        repo.create(bucket.clone()).await.unwrap();

        bucket.name = "Updated Name".to_string();
        bucket.accepted_types.push(ArtifactType::CodeChange);

        let result = repo.update(&bucket).await;
        assert!(result.is_ok());

        let updated = repo.get_by_id(&bucket.id).await.unwrap().unwrap();
        assert_eq!(updated.name, "Updated Name");
        assert!(updated.accepts_type(ArtifactType::CodeChange));
    }

    // ==================== DELETE TESTS ====================

    #[tokio::test]
    async fn test_delete_custom_bucket() {
        let conn = setup_test_db();
        let repo = SqliteArtifactBucketRepository::new(conn);

        let bucket = create_test_bucket();
        repo.create(bucket.clone()).await.unwrap();

        let result = repo.delete(&bucket.id).await;
        assert!(result.is_ok());

        let found = repo.get_by_id(&bucket.id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_delete_system_bucket_fails() {
        let conn = setup_test_db();
        let repo = SqliteArtifactBucketRepository::new(conn);

        let bucket = create_system_bucket();
        repo.create(bucket.clone()).await.unwrap();

        let result = repo.delete(&bucket.id).await;
        assert!(result.is_err());

        // Bucket should still exist
        let found = repo.get_by_id(&bucket.id).await.unwrap();
        assert!(found.is_some());
    }

    // ==================== EXISTS TESTS ====================

    #[tokio::test]
    async fn test_exists_true() {
        let conn = setup_test_db();
        let repo = SqliteArtifactBucketRepository::new(conn);

        let bucket = create_test_bucket();
        repo.create(bucket.clone()).await.unwrap();

        let result = repo.exists(&bucket.id).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_exists_false() {
        let conn = setup_test_db();
        let repo = SqliteArtifactBucketRepository::new(conn);
        let id = ArtifactBucketId::new();

        let result = repo.exists(&id).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    // ==================== SHARED CONNECTION TESTS ====================

    #[tokio::test]
    async fn test_from_shared_connection() {
        let conn = setup_test_db();
        let shared = Arc::new(Mutex::new(conn));

        let repo1 = SqliteArtifactBucketRepository::from_shared(shared.clone());
        let repo2 = SqliteArtifactBucketRepository::from_shared(shared.clone());

        // Create via repo1
        let bucket = create_test_bucket();
        repo1.create(bucket.clone()).await.unwrap();

        // Read via repo2
        let found = repo2.get_by_id(&bucket.id).await.unwrap();
        assert!(found.is_some());
    }

    // ==================== SEEDING TESTS ====================

    #[tokio::test]
    async fn test_seed_builtin_buckets_creates_all_four() {
        let conn = setup_test_db();
        let repo = SqliteArtifactBucketRepository::new(conn);

        let count = repo.seed_builtin_buckets().await.unwrap();
        assert_eq!(count, 4);

        let all = repo.get_all().await.unwrap();
        assert_eq!(all.len(), 4);

        let system = repo.get_system_buckets().await.unwrap();
        assert_eq!(system.len(), 4);
    }

    #[tokio::test]
    async fn test_seed_builtin_buckets_is_idempotent() {
        let conn = setup_test_db();
        let repo = SqliteArtifactBucketRepository::new(conn);

        // Seed twice
        let count1 = repo.seed_builtin_buckets().await.unwrap();
        let count2 = repo.seed_builtin_buckets().await.unwrap();

        // First seed creates 4, second creates 0
        assert_eq!(count1, 4);
        assert_eq!(count2, 0);

        // Still only 4 buckets
        let all = repo.get_all().await.unwrap();
        assert_eq!(all.len(), 4);
    }

    #[tokio::test]
    async fn test_seed_builtin_buckets_creates_research_outputs() {
        let conn = setup_test_db();
        let repo = SqliteArtifactBucketRepository::new(conn);

        repo.seed_builtin_buckets().await.unwrap();

        let research_id = ArtifactBucketId::from_string("research-outputs");
        let bucket = repo.get_by_id(&research_id).await.unwrap();
        assert!(bucket.is_some());

        let bucket = bucket.unwrap();
        assert_eq!(bucket.name, "Research Outputs");
        assert!(bucket.is_system);
        assert!(bucket.accepts_type(ArtifactType::ResearchDocument));
        assert!(bucket.accepts_type(ArtifactType::Findings));
        assert!(bucket.can_write("deep-researcher"));
    }

    #[tokio::test]
    async fn test_seed_builtin_buckets_creates_work_context() {
        let conn = setup_test_db();
        let repo = SqliteArtifactBucketRepository::new(conn);

        repo.seed_builtin_buckets().await.unwrap();

        let work_id = ArtifactBucketId::from_string("work-context");
        let bucket = repo.get_by_id(&work_id).await.unwrap();
        assert!(bucket.is_some());

        let bucket = bucket.unwrap();
        assert_eq!(bucket.name, "Work Context");
        assert!(bucket.is_system);
        assert!(bucket.accepts_type(ArtifactType::Context));
        assert!(bucket.accepts_type(ArtifactType::TaskSpec));
        assert!(bucket.can_write("orchestrator"));
    }

    #[tokio::test]
    async fn test_seed_builtin_buckets_creates_code_changes() {
        let conn = setup_test_db();
        let repo = SqliteArtifactBucketRepository::new(conn);

        repo.seed_builtin_buckets().await.unwrap();

        let code_id = ArtifactBucketId::from_string("code-changes");
        let bucket = repo.get_by_id(&code_id).await.unwrap();
        assert!(bucket.is_some());

        let bucket = bucket.unwrap();
        assert_eq!(bucket.name, "Code Changes");
        assert!(bucket.is_system);
        assert!(bucket.accepts_type(ArtifactType::CodeChange));
        assert!(bucket.accepts_type(ArtifactType::Diff));
        assert!(bucket.can_write("worker"));
    }

    #[tokio::test]
    async fn test_seed_builtin_buckets_creates_prd_library() {
        let conn = setup_test_db();
        let repo = SqliteArtifactBucketRepository::new(conn);

        repo.seed_builtin_buckets().await.unwrap();

        let prd_id = ArtifactBucketId::from_string("prd-library");
        let bucket = repo.get_by_id(&prd_id).await.unwrap();
        assert!(bucket.is_some());

        let bucket = bucket.unwrap();
        assert_eq!(bucket.name, "PRD Library");
        assert!(bucket.is_system);
        assert!(bucket.accepts_type(ArtifactType::Prd));
        assert!(bucket.accepts_type(ArtifactType::Specification));
        assert!(bucket.can_write("orchestrator"));
        assert!(bucket.can_write("user"));
    }

    #[tokio::test]
    async fn test_seed_builtin_buckets_preserves_existing() {
        let conn = setup_test_db();
        let repo = SqliteArtifactBucketRepository::new(conn);

        // Create a custom bucket first
        let custom = create_test_bucket();
        repo.create(custom).await.unwrap();

        // Seed built-ins
        repo.seed_builtin_buckets().await.unwrap();

        // Should have 5 buckets total (1 custom + 4 system)
        let all = repo.get_all().await.unwrap();
        assert_eq!(all.len(), 5);
    }
}
