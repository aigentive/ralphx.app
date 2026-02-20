use super::*;

#[test]
fn test_archive_job_type_serialization() {
    assert_eq!(
        ArchiveJobType::MemorySnapshot.to_string(),
        "memory_snapshot"
    );
    assert_eq!(ArchiveJobType::RuleSnapshot.to_string(), "rule_snapshot");
    assert_eq!(ArchiveJobType::FullRebuild.to_string(), "full_rebuild");
}

#[test]
fn test_archive_job_type_parsing() {
    assert_eq!(
        "memory_snapshot".parse::<ArchiveJobType>().unwrap(),
        ArchiveJobType::MemorySnapshot
    );
    assert_eq!(
        "rule_snapshot".parse::<ArchiveJobType>().unwrap(),
        ArchiveJobType::RuleSnapshot
    );
    assert_eq!(
        "full_rebuild".parse::<ArchiveJobType>().unwrap(),
        ArchiveJobType::FullRebuild
    );
    assert!("invalid".parse::<ArchiveJobType>().is_err());
}

#[test]
fn test_archive_job_status_serialization() {
    assert_eq!(ArchiveJobStatus::Pending.to_string(), "pending");
    assert_eq!(ArchiveJobStatus::Running.to_string(), "running");
    assert_eq!(ArchiveJobStatus::Done.to_string(), "done");
    assert_eq!(ArchiveJobStatus::Failed.to_string(), "failed");
}

#[test]
fn test_archive_job_status_parsing() {
    assert_eq!(
        "pending".parse::<ArchiveJobStatus>().unwrap(),
        ArchiveJobStatus::Pending
    );
    assert_eq!(
        "running".parse::<ArchiveJobStatus>().unwrap(),
        ArchiveJobStatus::Running
    );
    assert_eq!(
        "done".parse::<ArchiveJobStatus>().unwrap(),
        ArchiveJobStatus::Done
    );
    assert_eq!(
        "failed".parse::<ArchiveJobStatus>().unwrap(),
        ArchiveJobStatus::Failed
    );
    assert!("invalid".parse::<ArchiveJobStatus>().is_err());
}

#[test]
fn test_memory_archive_job_lifecycle() {
    let project_id = ProjectId::from_string("test-project".to_string());
    let payload = ArchiveJobPayload::memory_snapshot("mem_123");
    let mut job = MemoryArchiveJob::new(project_id, ArchiveJobType::MemorySnapshot, payload);

    // Initially pending
    assert_eq!(job.status, ArchiveJobStatus::Pending);
    assert!(job.can_claim());
    assert!(job.started_at.is_none());
    assert!(job.completed_at.is_none());

    // Start job
    job.start();
    assert_eq!(job.status, ArchiveJobStatus::Running);
    assert!(!job.can_claim());
    assert!(job.started_at.is_some());
    assert!(job.completed_at.is_none());

    // Complete job
    job.complete();
    assert_eq!(job.status, ArchiveJobStatus::Done);
    assert!(!job.can_claim());
    assert!(job.completed_at.is_some());
    assert!(job.error_message.is_none());
}

#[test]
fn test_memory_archive_job_failure() {
    let project_id = ProjectId::from_string("test-project".to_string());
    let payload = ArchiveJobPayload::memory_snapshot("mem_123");
    let mut job = MemoryArchiveJob::new(project_id, ArchiveJobType::MemorySnapshot, payload);

    job.start();
    job.fail("Test error");

    assert_eq!(job.status, ArchiveJobStatus::Failed);
    assert!(job.can_claim()); // Failed jobs can be retried
    assert!(job.completed_at.is_some());
    assert_eq!(job.error_message.as_deref(), Some("Test error"));
}

#[test]
fn test_archive_job_payload_json_roundtrip() {
    let payload = ArchiveJobPayload::memory_snapshot("mem_123");
    let json = payload.to_json().unwrap();
    let parsed = ArchiveJobPayload::from_json(&json).unwrap();

    match parsed {
        ArchiveJobPayload::MemorySnapshot(p) => assert_eq!(p.memory_id, "mem_123"),
        _ => panic!("Wrong payload type"),
    }
}
