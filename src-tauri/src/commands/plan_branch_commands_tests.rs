use super::*;

#[test]
fn slug_from_name_simple() {
    assert_eq!(slug_from_name("My Project"), "my-project");
}

#[test]
fn slug_from_name_special_chars() {
    assert_eq!(slug_from_name("My App (v2.0)"), "my-app-v2-0");
}

#[test]
fn slug_from_name_collapses_consecutive_hyphens() {
    assert_eq!(slug_from_name("foo---bar"), "foo-bar");
}

#[test]
fn slug_from_name_trims_leading_trailing() {
    assert_eq!(slug_from_name(" Hello World "), "hello-world");
}

#[test]
fn plan_branch_response_from_entity() {
    let pb = PlanBranch::new(
        ArtifactId::from_string("art-1"),
        IdeationSessionId::from_string("sess-1"),
        ProjectId::from_string("proj-1".to_string()),
        "ralphx/my-app/plan-a1b2c3".to_string(),
        "main".to_string(),
    );

    let response = PlanBranchResponse::from(pb);
    assert_eq!(response.plan_artifact_id, "art-1");
    assert_eq!(response.branch_name, "ralphx/my-app/plan-a1b2c3");
    assert_eq!(response.source_branch, "main");
    assert_eq!(response.status, "active");
    assert!(response.merge_task_id.is_none());
    assert!(response.merged_at.is_none());
}
