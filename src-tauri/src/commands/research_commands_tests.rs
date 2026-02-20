use super::*;

fn setup_test_state() -> AppState {
    AppState::new_test()
}

fn create_test_process() -> ResearchProcess {
    let brief = ResearchBrief::new("What architecture should we use?");
    ResearchProcess::new("Test Research", brief, "deep-researcher")
        .with_preset(ResearchDepthPreset::Standard)
}

#[tokio::test]
async fn test_create_research_process() {
    let state = setup_test_state();

    let process = create_test_process();
    let created = state.process_repo.create(process).await.unwrap();

    assert_eq!(created.name, "Test Research");
    assert_eq!(created.agent_profile_id, "deep-researcher");
}

#[tokio::test]
async fn test_get_research_process_by_id() {
    let state = setup_test_state();

    let process = create_test_process();
    let id = process.id.clone();

    state.process_repo.create(process).await.unwrap();

    let found = state.process_repo.get_by_id(&id).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "Test Research");
}

#[tokio::test]
async fn test_get_all_research_processes() {
    let state = setup_test_state();

    state
        .process_repo
        .create(create_test_process())
        .await
        .unwrap();

    let brief2 = ResearchBrief::new("Another question");
    let process2 = ResearchProcess::new("Another Research", brief2, "researcher");
    state.process_repo.create(process2).await.unwrap();

    let all = state.process_repo.get_all().await.unwrap();
    assert_eq!(all.len(), 2);
}

#[tokio::test]
async fn test_pause_and_resume_research() {
    let state = setup_test_state();

    let mut process = create_test_process();
    process.start();
    let id = process.id.clone();

    state.process_repo.create(process).await.unwrap();

    // Get and pause
    let mut found = state.process_repo.get_by_id(&id).await.unwrap().unwrap();
    found.pause();
    state.process_repo.update(&found).await.unwrap();

    // Verify paused
    let found = state.process_repo.get_by_id(&id).await.unwrap().unwrap();
    assert_eq!(found.status(), ResearchProcessStatus::Paused);

    // Resume
    let mut found = state.process_repo.get_by_id(&id).await.unwrap().unwrap();
    found.resume();
    state.process_repo.update(&found).await.unwrap();

    // Verify running
    let found = state.process_repo.get_by_id(&id).await.unwrap().unwrap();
    assert_eq!(found.status(), ResearchProcessStatus::Running);
}

#[tokio::test]
async fn test_complete_research_process() {
    let state = setup_test_state();

    let mut process = create_test_process();
    process.start();
    let id = process.id.clone();

    state.process_repo.create(process).await.unwrap();

    state.process_repo.complete(&id).await.unwrap();

    let found = state.process_repo.get_by_id(&id).await.unwrap().unwrap();
    assert_eq!(found.status(), ResearchProcessStatus::Completed);
}

#[tokio::test]
async fn test_fail_research_process() {
    let state = setup_test_state();

    let mut process = create_test_process();
    process.start();
    let id = process.id.clone();

    state.process_repo.create(process).await.unwrap();

    state.process_repo.fail(&id, "Test error").await.unwrap();

    let found = state.process_repo.get_by_id(&id).await.unwrap().unwrap();
    assert_eq!(found.status(), ResearchProcessStatus::Failed);
}

#[tokio::test]
async fn test_get_processes_by_status() {
    let state = setup_test_state();

    // Create pending process
    let process1 = create_test_process();
    state.process_repo.create(process1).await.unwrap();

    // Create running process
    let mut process2 = create_test_process();
    process2.start();
    state.process_repo.create(process2).await.unwrap();

    // Get pending only
    let pending = state
        .process_repo
        .get_by_status(ResearchProcessStatus::Pending)
        .await
        .unwrap();
    assert_eq!(pending.len(), 1);

    // Get running only
    let running = state
        .process_repo
        .get_by_status(ResearchProcessStatus::Running)
        .await
        .unwrap();
    assert_eq!(running.len(), 1);
}

#[tokio::test]
async fn test_research_process_response_serialization() {
    let mut process = create_test_process();
    process.start();
    process.progress.current_iteration = 10;

    let response = ResearchProcessResponse::from(process);

    assert_eq!(response.name, "Test Research");
    assert_eq!(response.status, "running");
    assert_eq!(response.current_iteration, 10);
    assert!(response.depth_preset.is_some());
    assert_eq!(response.depth_preset.as_ref().unwrap(), "standard");

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"name\":\"Test Research\""));
}

#[tokio::test]
async fn test_get_research_presets() {
    let result = get_research_presets().await.unwrap();

    assert_eq!(result.len(), 4);

    let ids: Vec<&str> = result.iter().map(|p| p.id.as_str()).collect();
    assert!(ids.contains(&"quick-scan"));
    assert!(ids.contains(&"standard"));
    assert!(ids.contains(&"deep-dive"));
    assert!(ids.contains(&"exhaustive"));

    let standard = result.iter().find(|p| p.id == "standard").unwrap();
    assert_eq!(standard.max_iterations, 50);
    assert_eq!(standard.timeout_hours, 2.0);
}
