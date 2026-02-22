use super::*;
use crate::domain::entities::ArchiveJobPayload;
use crate::infrastructure::sqlite::connection::open_memory_connection;
use crate::infrastructure::sqlite::migrations::run_migrations;

async fn setup_test_repo() -> SqliteMemoryArchiveRepository {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    // Insert a test project (required for foreign key)
    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('test-project', 'Test Project', '/test/path')",
        [],
    )
    .unwrap();
    SqliteMemoryArchiveRepository::new(conn)
}

#[tokio::test]
async fn test_create_and_get_job() {
    let repo = setup_test_repo().await;
    let project_id = ProjectId::from_string("test-project".to_string());
    let payload = ArchiveJobPayload::memory_snapshot("mem_123");
    let job = MemoryArchiveJob::new(project_id.clone(), ArchiveJobType::MemorySnapshot, payload);

    let created_job = repo.create(job.clone()).await.unwrap();
    assert_eq!(created_job.id, job.id);

    let retrieved_job = repo.get_by_id(&job.id).await.unwrap();
    assert!(retrieved_job.is_some());
    assert_eq!(retrieved_job.unwrap().id, job.id);
}

#[tokio::test]
async fn test_claim_next() {
    let repo = setup_test_repo().await;
    let project_id = ProjectId::from_string("test-project".to_string());

    // Create two jobs
    let job1 = MemoryArchiveJob::new(
        project_id.clone(),
        ArchiveJobType::MemorySnapshot,
        ArchiveJobPayload::memory_snapshot("mem_1"),
    );
    let job2 = MemoryArchiveJob::new(
        project_id.clone(),
        ArchiveJobType::RuleSnapshot,
        ArchiveJobPayload::rule_snapshot("rule_1"),
    );

    repo.create(job1.clone()).await.unwrap();
    // Add a small delay to ensure different timestamps
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    repo.create(job2.clone()).await.unwrap();

    // Claim the first job (oldest)
    let claimed = repo.claim_next().await.unwrap();
    assert!(claimed.is_some());
    let claimed_job = claimed.unwrap();
    assert_eq!(claimed_job.id, job1.id);
    assert_eq!(claimed_job.status, ArchiveJobStatus::Running);

    // Claim the second job
    let claimed2 = repo.claim_next().await.unwrap();
    assert!(claimed2.is_some());
    assert_eq!(claimed2.unwrap().id, job2.id);

    // No more claimable jobs
    let claimed3 = repo.claim_next().await.unwrap();
    assert!(claimed3.is_none());
}

#[tokio::test]
async fn test_update_job_status() {
    let repo = setup_test_repo().await;
    let project_id = ProjectId::from_string("test-project".to_string());
    let mut job = MemoryArchiveJob::new(
        project_id,
        ArchiveJobType::MemorySnapshot,
        ArchiveJobPayload::memory_snapshot("mem_123"),
    );

    repo.create(job.clone()).await.unwrap();

    job.start();
    repo.update(&job).await.unwrap();

    let updated = repo.get_by_id(&job.id).await.unwrap().unwrap();
    assert_eq!(updated.status, ArchiveJobStatus::Running);
    assert!(updated.started_at.is_some());

    job.complete();
    repo.update(&job).await.unwrap();

    let completed = repo.get_by_id(&job.id).await.unwrap().unwrap();
    assert_eq!(completed.status, ArchiveJobStatus::Done);
    assert!(completed.completed_at.is_some());
}

#[tokio::test]
async fn test_get_by_status() {
    let repo = setup_test_repo().await;
    let project_id = ProjectId::from_string("test-project".to_string());

    let mut job1 = MemoryArchiveJob::new(
        project_id.clone(),
        ArchiveJobType::MemorySnapshot,
        ArchiveJobPayload::memory_snapshot("mem_1"),
    );
    let job2 = MemoryArchiveJob::new(
        project_id,
        ArchiveJobType::RuleSnapshot,
        ArchiveJobPayload::rule_snapshot("rule_1"),
    );

    repo.create(job1.clone()).await.unwrap();
    repo.create(job2).await.unwrap();

    // Mark job1 as running
    job1.start();
    repo.update(&job1).await.unwrap();

    let pending_jobs = repo.get_by_status(ArchiveJobStatus::Pending).await.unwrap();
    assert_eq!(pending_jobs.len(), 1);

    let running_jobs = repo.get_by_status(ArchiveJobStatus::Running).await.unwrap();
    assert_eq!(running_jobs.len(), 1);
}

#[tokio::test]
async fn test_count_claimable() {
    let repo = setup_test_repo().await;
    let project_id = ProjectId::from_string("test-project".to_string());

    assert_eq!(repo.count_claimable().await.unwrap(), 0);

    let job1 = MemoryArchiveJob::new(
        project_id.clone(),
        ArchiveJobType::MemorySnapshot,
        ArchiveJobPayload::memory_snapshot("mem_1"),
    );
    let mut job2 = MemoryArchiveJob::new(
        project_id,
        ArchiveJobType::RuleSnapshot,
        ArchiveJobPayload::rule_snapshot("rule_1"),
    );

    repo.create(job1).await.unwrap();
    repo.create(job2.clone()).await.unwrap();

    assert_eq!(repo.count_claimable().await.unwrap(), 2);

    // Mark job2 as running
    job2.start();
    repo.update(&job2).await.unwrap();

    assert_eq!(repo.count_claimable().await.unwrap(), 1);

    // Fail job2 - should be claimable again
    job2.fail("Test error");
    repo.update(&job2).await.unwrap();

    assert_eq!(repo.count_claimable().await.unwrap(), 2);
}
