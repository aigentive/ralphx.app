use super::*;
use crate::domain::entities::research::{CustomDepth, ResearchDepthPreset};
use crate::domain::entities::ArtifactType;
use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

fn setup_test_db() -> Connection {
    let conn = open_memory_connection().expect("Failed to open memory connection");
    run_migrations(&conn).expect("Failed to run migrations");
    conn
}

fn create_test_process() -> ResearchProcess {
    let brief = ResearchBrief::new("What architecture should we use?")
        .with_context("Building a new web application")
        .with_constraint("Must be scalable");
    ResearchProcess::new("Architecture Research", brief, "deep-researcher")
        .with_preset(ResearchDepthPreset::Standard)
}

fn create_running_process() -> ResearchProcess {
    let brief = ResearchBrief::new("Which database to choose?");
    let mut process = ResearchProcess::new("Database Research", brief, "deep-researcher")
        .with_preset(ResearchDepthPreset::QuickScan);
    process.start();
    process.advance();
    process.advance();
    process
}

#[tokio::test]
async fn test_create_process() {
    let conn = setup_test_db();
    let repo = SqliteProcessRepository::new(conn);
    let process = create_test_process();

    let result = repo.create(process.clone()).await;
    assert!(result.is_ok());

    let created = result.unwrap();
    assert_eq!(created.id, process.id);
    assert_eq!(created.name, "Architecture Research");
}

#[tokio::test]
async fn test_get_by_id_found() {
    let conn = setup_test_db();
    let repo = SqliteProcessRepository::new(conn);
    let process = create_test_process();

    repo.create(process.clone()).await.unwrap();

    let result = repo.get_by_id(&process.id).await;
    assert!(result.is_ok());

    let found = result.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "Architecture Research");
}

#[tokio::test]
async fn test_get_by_id_not_found() {
    let conn = setup_test_db();
    let repo = SqliteProcessRepository::new(conn);
    let id = ResearchProcessId::new();

    let result = repo.get_by_id(&id).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn test_get_all_empty() {
    let conn = setup_test_db();
    let repo = SqliteProcessRepository::new(conn);

    let result = repo.get_all().await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[tokio::test]
async fn test_get_all_with_processes() {
    let conn = setup_test_db();
    let repo = SqliteProcessRepository::new(conn);

    let process1 = create_test_process();
    let process2 = create_running_process();

    repo.create(process1).await.unwrap();
    repo.create(process2).await.unwrap();

    let result = repo.get_all().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 2);
}

#[tokio::test]
async fn test_get_by_status_pending() {
    let conn = setup_test_db();
    let repo = SqliteProcessRepository::new(conn);

    let pending = create_test_process();
    let running = create_running_process();

    repo.create(pending).await.unwrap();
    repo.create(running).await.unwrap();

    let result = repo.get_by_status(ResearchProcessStatus::Pending).await;
    assert!(result.is_ok());
    let processes = result.unwrap();
    assert_eq!(processes.len(), 1);
    assert_eq!(processes[0].status(), ResearchProcessStatus::Pending);
}

#[tokio::test]
async fn test_get_by_status_running() {
    let conn = setup_test_db();
    let repo = SqliteProcessRepository::new(conn);

    let pending = create_test_process();
    let running = create_running_process();

    repo.create(pending).await.unwrap();
    repo.create(running).await.unwrap();

    let result = repo.get_by_status(ResearchProcessStatus::Running).await;
    assert!(result.is_ok());
    let processes = result.unwrap();
    assert_eq!(processes.len(), 1);
    assert_eq!(processes[0].status(), ResearchProcessStatus::Running);
}

#[tokio::test]
async fn test_get_active() {
    let conn = setup_test_db();
    let repo = SqliteProcessRepository::new(conn);

    let pending = create_test_process();
    let running = create_running_process();

    // Create a completed process
    let brief = ResearchBrief::new("Completed question");
    let mut completed = ResearchProcess::new("Completed Research", brief, "researcher");
    completed.start();
    completed.complete();

    repo.create(pending).await.unwrap();
    repo.create(running).await.unwrap();
    repo.create(completed).await.unwrap();

    let result = repo.get_active().await;
    assert!(result.is_ok());
    let active = result.unwrap();
    assert_eq!(active.len(), 2);
}

#[tokio::test]
async fn test_update_progress() {
    let conn = setup_test_db();
    let repo = SqliteProcessRepository::new(conn);

    let mut process = create_test_process();
    repo.create(process.clone()).await.unwrap();

    // Update progress
    process.start();
    process.advance();
    process.advance();
    process.advance();

    repo.update_progress(&process).await.unwrap();

    let loaded = repo.get_by_id(&process.id).await.unwrap().unwrap();
    assert_eq!(loaded.progress.current_iteration, 3);
    assert_eq!(loaded.status(), ResearchProcessStatus::Running);
    assert!(loaded.started_at.is_some());
}

#[tokio::test]
async fn test_update() {
    let conn = setup_test_db();
    let repo = SqliteProcessRepository::new(conn);

    let mut process = create_test_process();
    repo.create(process.clone()).await.unwrap();

    process.name = "Updated Research Name".to_string();
    process.start();

    repo.update(&process).await.unwrap();

    let loaded = repo.get_by_id(&process.id).await.unwrap().unwrap();
    assert_eq!(loaded.name, "Updated Research Name");
    assert_eq!(loaded.status(), ResearchProcessStatus::Running);
}

#[tokio::test]
async fn test_complete() {
    let conn = setup_test_db();
    let repo = SqliteProcessRepository::new(conn);

    let process = create_running_process();
    repo.create(process.clone()).await.unwrap();

    repo.complete(&process.id).await.unwrap();

    let loaded = repo.get_by_id(&process.id).await.unwrap().unwrap();
    assert_eq!(loaded.status(), ResearchProcessStatus::Completed);
    assert!(loaded.completed_at.is_some());
}

#[tokio::test]
async fn test_fail() {
    let conn = setup_test_db();
    let repo = SqliteProcessRepository::new(conn);

    let process = create_running_process();
    repo.create(process.clone()).await.unwrap();

    repo.fail(&process.id, "Network timeout").await.unwrap();

    let loaded = repo.get_by_id(&process.id).await.unwrap().unwrap();
    assert_eq!(loaded.status(), ResearchProcessStatus::Failed);
    assert!(loaded.completed_at.is_some());
    assert_eq!(
        loaded.progress.error_message,
        Some("Network timeout".to_string())
    );
}

#[tokio::test]
async fn test_delete() {
    let conn = setup_test_db();
    let repo = SqliteProcessRepository::new(conn);

    let process = create_test_process();
    repo.create(process.clone()).await.unwrap();

    repo.delete(&process.id).await.unwrap();

    let found = repo.get_by_id(&process.id).await.unwrap();
    assert!(found.is_none());
}

#[tokio::test]
async fn test_exists_true() {
    let conn = setup_test_db();
    let repo = SqliteProcessRepository::new(conn);

    let process = create_test_process();
    repo.create(process.clone()).await.unwrap();

    let result = repo.exists(&process.id).await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[tokio::test]
async fn test_exists_false() {
    let conn = setup_test_db();
    let repo = SqliteProcessRepository::new(conn);

    let id = ResearchProcessId::new();

    let result = repo.exists(&id).await;
    assert!(result.is_ok());
    assert!(!result.unwrap());
}

#[tokio::test]
async fn test_brief_preserved() {
    let conn = setup_test_db();
    let repo = SqliteProcessRepository::new(conn);

    let brief = ResearchBrief::new("Main question")
        .with_context("Context info")
        .with_scope("Backend only")
        .with_constraints(["Constraint 1", "Constraint 2"]);
    let process = ResearchProcess::new("Brief Test", brief, "researcher");
    repo.create(process.clone()).await.unwrap();

    let loaded = repo.get_by_id(&process.id).await.unwrap().unwrap();

    assert_eq!(loaded.brief.question, "Main question");
    assert_eq!(loaded.brief.context, Some("Context info".to_string()));
    assert_eq!(loaded.brief.scope, Some("Backend only".to_string()));
    assert_eq!(loaded.brief.constraints.len(), 2);
}

#[tokio::test]
async fn test_preset_depth_preserved() {
    let conn = setup_test_db();
    let repo = SqliteProcessRepository::new(conn);

    let brief = ResearchBrief::new("Question");
    let process = ResearchProcess::new("Depth Test", brief, "researcher")
        .with_preset(ResearchDepthPreset::DeepDive);
    repo.create(process.clone()).await.unwrap();

    let loaded = repo.get_by_id(&process.id).await.unwrap().unwrap();

    assert!(loaded.depth.is_preset());
    let resolved = loaded.resolved_depth();
    assert_eq!(resolved.max_iterations, 200);
    assert_eq!(resolved.timeout_hours, 8.0);
}

#[tokio::test]
async fn test_custom_depth_preserved() {
    let conn = setup_test_db();
    let repo = SqliteProcessRepository::new(conn);

    let brief = ResearchBrief::new("Question");
    let process = ResearchProcess::new("Custom Depth Test", brief, "researcher")
        .with_custom_depth(CustomDepth::new(150, 5.0, 30));
    repo.create(process.clone()).await.unwrap();

    let loaded = repo.get_by_id(&process.id).await.unwrap().unwrap();

    assert!(loaded.depth.is_custom());
    let resolved = loaded.resolved_depth();
    assert_eq!(resolved.max_iterations, 150);
    assert_eq!(resolved.timeout_hours, 5.0);
    assert_eq!(resolved.checkpoint_interval, 30);
}

#[tokio::test]
async fn test_output_config_preserved() {
    let conn = setup_test_db();
    let repo = SqliteProcessRepository::new(conn);

    let brief = ResearchBrief::new("Question");
    let output = ResearchOutput::new("custom-bucket")
        .with_artifact_type(ArtifactType::Findings)
        .with_artifact_type(ArtifactType::Recommendations);
    let process = ResearchProcess::new("Output Test", brief, "researcher").with_output(output);
    repo.create(process.clone()).await.unwrap();

    let loaded = repo.get_by_id(&process.id).await.unwrap().unwrap();

    assert_eq!(loaded.output.target_bucket, "custom-bucket");
    assert_eq!(loaded.output.artifact_types.len(), 2);
    assert!(loaded
        .output
        .artifact_types
        .contains(&ArtifactType::Findings));
    assert!(loaded
        .output
        .artifact_types
        .contains(&ArtifactType::Recommendations));
}

#[tokio::test]
async fn test_timestamps_preserved() {
    let conn = setup_test_db();
    let repo = SqliteProcessRepository::new(conn);

    let process = create_test_process();
    let original_created_at = process.created_at;
    repo.create(process.clone()).await.unwrap();

    let loaded = repo.get_by_id(&process.id).await.unwrap().unwrap();

    // Timestamps should match (allowing for RFC3339 precision)
    let diff = (loaded.created_at - original_created_at)
        .num_milliseconds()
        .abs();
    assert!(diff < 1000, "Timestamps differ by {}ms", diff);
}

#[tokio::test]
async fn test_from_shared_connection() {
    let conn = setup_test_db();
    let shared = Arc::new(Mutex::new(conn));

    let repo1 = SqliteProcessRepository::from_shared(shared.clone());
    let repo2 = SqliteProcessRepository::from_shared(shared.clone());

    // Create via repo1
    let process = create_test_process();
    repo1.create(process.clone()).await.unwrap();

    // Read via repo2
    let found = repo2.get_by_id(&process.id).await.unwrap();
    assert!(found.is_some());
}

#[tokio::test]
async fn test_checkpoint_preserved() {
    let conn = setup_test_db();
    let repo = SqliteProcessRepository::new(conn);

    let brief = ResearchBrief::new("Question");
    let mut process = ResearchProcess::new("Checkpoint Test", brief, "researcher");
    process.start();
    let checkpoint_id = crate::domain::entities::ArtifactId::from_string("checkpoint-artifact-1");
    process.checkpoint(checkpoint_id.clone());

    repo.create(process.clone()).await.unwrap();

    let loaded = repo.get_by_id(&process.id).await.unwrap().unwrap();
    assert_eq!(loaded.progress.last_checkpoint, Some(checkpoint_id));
}

#[tokio::test]
async fn test_get_all_ordered_by_created_at_desc() {
    let conn = setup_test_db();
    let repo = SqliteProcessRepository::new(conn);

    // Create processes with slight time differences
    let process1 = create_test_process();
    repo.create(process1.clone()).await.unwrap();

    // Small delay to ensure different timestamps
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    let process2 = create_running_process();
    repo.create(process2.clone()).await.unwrap();

    let all = repo.get_all().await.unwrap();
    assert_eq!(all.len(), 2);
    // Most recent first
    assert_eq!(all[0].id, process2.id);
    assert_eq!(all[1].id, process1.id);
}
