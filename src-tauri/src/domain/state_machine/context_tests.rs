use super::*;

#[test]
fn test_task_context_creation() {
    let ctx = TaskContext::new_test("task-1", "proj-1");
    assert_eq!(ctx.task_id, "task-1");
    assert_eq!(ctx.project_id, "proj-1");
    assert!(!ctx.has_blockers());
}

#[test]
fn test_task_context_blockers() {
    let mut ctx = TaskContext::new_test("task-1", "proj-1");
    assert!(!ctx.has_blockers());

    ctx.add_blocker(Blocker::new("task-2"));
    assert!(ctx.has_blockers());
    assert_eq!(ctx.blockers.len(), 1);

    ctx.clear_blockers();
    assert!(!ctx.has_blockers());
}

#[test]
fn test_task_services_mock() {
    let services = TaskServices::new_mock();
    // Just verify it creates without panicking
    let _ = format!("{:?}", services);
}
