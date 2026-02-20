use super::*;

#[tokio::test]
async fn test_add_and_get_blockers() {
    let repo = MemoryTaskDependencyRepository::new();
    let t1 = TaskId::new();
    let t2 = TaskId::new();

    repo.add_dependency(&t1, &t2).await.unwrap();

    let blockers = repo.get_blockers(&t1).await.unwrap();
    assert_eq!(blockers.len(), 1);
    assert_eq!(blockers[0].to_string(), t2.to_string());
}

#[tokio::test]
async fn test_get_blocked_by() {
    let repo = MemoryTaskDependencyRepository::new();
    let t1 = TaskId::new();
    let t2 = TaskId::new();

    repo.add_dependency(&t1, &t2).await.unwrap();

    let blocked = repo.get_blocked_by(&t2).await.unwrap();
    assert_eq!(blocked.len(), 1);
    assert_eq!(blocked[0].to_string(), t1.to_string());
}

#[tokio::test]
async fn test_remove_dependency() {
    let repo = MemoryTaskDependencyRepository::new();
    let t1 = TaskId::new();
    let t2 = TaskId::new();

    repo.add_dependency(&t1, &t2).await.unwrap();
    repo.remove_dependency(&t1, &t2).await.unwrap();

    let blockers = repo.get_blockers(&t1).await.unwrap();
    assert!(blockers.is_empty());
}

#[tokio::test]
async fn test_clear_dependencies() {
    let repo = MemoryTaskDependencyRepository::new();
    let t1 = TaskId::new();
    let t2 = TaskId::new();
    let t3 = TaskId::new();

    repo.add_dependency(&t1, &t2).await.unwrap();
    repo.add_dependency(&t3, &t1).await.unwrap();

    repo.clear_dependencies(&t1).await.unwrap();

    let blockers = repo.get_blockers(&t1).await.unwrap();
    let blocked = repo.get_blocked_by(&t1).await.unwrap();

    assert!(blockers.is_empty());
    assert!(blocked.is_empty());
}

#[tokio::test]
async fn test_has_dependency() {
    let repo = MemoryTaskDependencyRepository::new();
    let t1 = TaskId::new();
    let t2 = TaskId::new();
    let t3 = TaskId::new();

    repo.add_dependency(&t1, &t2).await.unwrap();

    assert!(repo.has_dependency(&t1, &t2).await.unwrap());
    assert!(!repo.has_dependency(&t1, &t3).await.unwrap());
}

#[tokio::test]
async fn test_count_blockers_and_blocked() {
    let repo = MemoryTaskDependencyRepository::new();
    let t1 = TaskId::new();
    let t2 = TaskId::new();
    let t3 = TaskId::new();

    repo.add_dependency(&t1, &t2).await.unwrap();
    repo.add_dependency(&t1, &t3).await.unwrap();

    assert_eq!(repo.count_blockers(&t1).await.unwrap(), 2);
    assert_eq!(repo.count_blocked_by(&t2).await.unwrap(), 1);
}
