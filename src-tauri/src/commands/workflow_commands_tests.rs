use super::*;

fn setup_test_state() -> AppState {
    AppState::new_test()
}

#[tokio::test]
async fn test_create_workflow() {
    let state = setup_test_state();

    let workflow = WorkflowSchema::new(
        "Test Workflow",
        vec![
            WorkflowColumn::new("backlog", "Backlog", InternalStatus::Backlog),
            WorkflowColumn::new("done", "Done", InternalStatus::Approved),
        ],
    );

    let created = state.workflow_repo.create(workflow).await.unwrap();
    assert_eq!(created.name, "Test Workflow");
    assert_eq!(created.columns.len(), 2);
}

#[tokio::test]
async fn test_get_workflow_by_id() {
    let state = setup_test_state();

    let workflow = WorkflowSchema::new(
        "Find Me",
        vec![WorkflowColumn::new("col", "Column", InternalStatus::Ready)],
    );
    let id = workflow.id.clone();

    state.workflow_repo.create(workflow).await.unwrap();

    let found = state.workflow_repo.get_by_id(&id).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "Find Me");
}

#[tokio::test]
async fn test_list_workflows() {
    let state = setup_test_state();

    state
        .workflow_repo
        .create(WorkflowSchema::new(
            "WF 1",
            vec![WorkflowColumn::new("a", "A", InternalStatus::Backlog)],
        ))
        .await
        .unwrap();
    state
        .workflow_repo
        .create(WorkflowSchema::new(
            "WF 2",
            vec![WorkflowColumn::new("b", "B", InternalStatus::Ready)],
        ))
        .await
        .unwrap();

    let all = state.workflow_repo.get_all().await.unwrap();
    assert_eq!(all.len(), 2);
}

#[tokio::test]
async fn test_delete_workflow() {
    let state = setup_test_state();

    let workflow = WorkflowSchema::new(
        "To Delete",
        vec![WorkflowColumn::new("col", "Col", InternalStatus::Backlog)],
    );
    let id = workflow.id.clone();

    state.workflow_repo.create(workflow).await.unwrap();
    state.workflow_repo.delete(&id).await.unwrap();

    let found = state.workflow_repo.get_by_id(&id).await.unwrap();
    assert!(found.is_none());
}

#[tokio::test]
async fn test_set_default_workflow() {
    let state = setup_test_state();

    let wf1 = WorkflowSchema::default_ralphx();
    let wf2 = WorkflowSchema::new(
        "Second",
        vec![WorkflowColumn::new("x", "X", InternalStatus::Backlog)],
    );
    let wf2_id = wf2.id.clone();

    state.workflow_repo.create(wf1).await.unwrap();
    state.workflow_repo.create(wf2).await.unwrap();

    state.workflow_repo.set_default(&wf2_id).await.unwrap();

    let default = state.workflow_repo.get_default().await.unwrap();
    assert!(default.is_some());
    assert_eq!(default.unwrap().id, wf2_id);
}

#[tokio::test]
async fn test_workflow_response_serialization() {
    let workflow = WorkflowSchema::new(
        "Response Test",
        vec![
            WorkflowColumn::new("col1", "Column 1", InternalStatus::Backlog).with_color("#ff0000"),
        ],
    )
    .with_description("A test workflow");

    let response = WorkflowResponse::from(workflow);

    assert_eq!(response.name, "Response Test");
    assert_eq!(response.description, Some("A test workflow".to_string()));
    assert_eq!(response.columns.len(), 1);
    assert_eq!(response.columns[0].color, Some("#ff0000".to_string()));

    // Verify JSON serialization uses snake_case (Rust default)
    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"name\":\"Response Test\""));
    assert!(
        json.contains("\"is_default\""),
        "Expected snake_case field is_default"
    );
}

#[tokio::test]
async fn test_column_input_to_column() {
    let input = WorkflowColumnInput {
        id: "test-col".to_string(),
        name: "Test Column".to_string(),
        maps_to: "ready".to_string(),
        color: Some("#00ff00".to_string()),
        icon: None,
        skip_review: Some(true),
        auto_advance: None,
        agent_profile: Some("fast-worker".to_string()),
    };

    let column = input.to_column().unwrap();

    assert_eq!(column.id, "test-col");
    assert_eq!(column.name, "Test Column");
    assert_eq!(column.maps_to, InternalStatus::Ready);
    assert_eq!(column.color, Some("#00ff00".to_string()));

    let behavior = column.behavior.unwrap();
    assert_eq!(behavior.skip_review, Some(true));
    assert_eq!(behavior.agent_profile, Some("fast-worker".to_string()));
}

#[tokio::test]
async fn test_column_input_invalid_status() {
    let input = WorkflowColumnInput {
        id: "test".to_string(),
        name: "Test".to_string(),
        maps_to: "invalid_status".to_string(),
        color: None,
        icon: None,
        skip_review: None,
        auto_advance: None,
        agent_profile: None,
    };

    let result = input.to_column();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid internal status"));
}

#[tokio::test]
async fn test_get_builtin_workflows() {
    let result = get_builtin_workflows().await.unwrap();

    assert_eq!(result.len(), 2);

    let names: Vec<&str> = result.iter().map(|w| w.name.as_str()).collect();
    assert!(names.contains(&"RalphX Default"));
    assert!(names.contains(&"Jira Compatible"));
}

#[tokio::test]
async fn test_get_active_workflow_columns_with_default() {
    let state = setup_test_state();

    // Create and set a default workflow
    let workflow = WorkflowSchema::new(
        "My Default",
        vec![
            WorkflowColumn::new("a", "A", InternalStatus::Backlog),
            WorkflowColumn::new("b", "B", InternalStatus::Approved),
        ],
    )
    .as_default();
    let _id = workflow.id.clone();

    state.workflow_repo.create(workflow).await.unwrap();

    let default = state.workflow_repo.get_default().await.unwrap();
    assert!(default.is_some());
    assert_eq!(default.unwrap().columns.len(), 2);
}
