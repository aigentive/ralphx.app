use super::*;

#[test]
fn test_memory_bucket_serialization() {
    assert_eq!(
        MemoryBucket::ArchitecturePatterns.to_string(),
        "architecture_patterns"
    );
    assert_eq!(
        MemoryBucket::ImplementationDiscoveries.to_string(),
        "implementation_discoveries"
    );
    assert_eq!(
        MemoryBucket::OperationalPlaybooks.to_string(),
        "operational_playbooks"
    );
}

#[test]
fn test_memory_bucket_parsing() {
    assert_eq!(
        "architecture_patterns".parse::<MemoryBucket>().unwrap(),
        MemoryBucket::ArchitecturePatterns
    );
    assert_eq!(
        "implementation_discoveries"
            .parse::<MemoryBucket>()
            .unwrap(),
        MemoryBucket::ImplementationDiscoveries
    );
    assert_eq!(
        "operational_playbooks".parse::<MemoryBucket>().unwrap(),
        MemoryBucket::OperationalPlaybooks
    );
    assert!("invalid".parse::<MemoryBucket>().is_err());
}

#[test]
fn test_memory_status_serialization() {
    assert_eq!(MemoryStatus::Active.to_string(), "active");
    assert_eq!(MemoryStatus::Obsolete.to_string(), "obsolete");
    assert_eq!(MemoryStatus::Archived.to_string(), "archived");
}

#[test]
fn test_memory_status_parsing() {
    assert_eq!(
        "active".parse::<MemoryStatus>().unwrap(),
        MemoryStatus::Active
    );
    assert_eq!(
        "obsolete".parse::<MemoryStatus>().unwrap(),
        MemoryStatus::Obsolete
    );
    assert_eq!(
        "archived".parse::<MemoryStatus>().unwrap(),
        MemoryStatus::Archived
    );
    assert!("invalid".parse::<MemoryStatus>().is_err());
}

#[test]
fn test_memory_entry_lifecycle() {
    let project_id = ProjectId::from_string("test-project".to_string());
    let mut entry = MemoryEntry::new(
        project_id,
        MemoryBucket::ImplementationDiscoveries,
        "Test Memory".to_string(),
        "Brief summary".to_string(),
        "# Full Details\n\nMore info here".to_string(),
        vec!["src/**/*.rs".to_string()],
        "hash123".to_string(),
    );

    assert_eq!(entry.status, MemoryStatus::Active);

    entry.mark_obsolete();
    assert_eq!(entry.status, MemoryStatus::Obsolete);

    entry.mark_archived();
    assert_eq!(entry.status, MemoryStatus::Archived);
}

#[test]
fn test_scope_paths_json_roundtrip() {
    let project_id = ProjectId::from_string("test-project".to_string());
    let entry = MemoryEntry::new(
        project_id,
        MemoryBucket::ArchitecturePatterns,
        "Test".to_string(),
        "Summary".to_string(),
        "Details".to_string(),
        vec!["src/**/*.rs".to_string(), "tests/**/*.rs".to_string()],
        "hash".to_string(),
    );

    let json = entry.scope_paths_to_json().unwrap();
    let parsed = MemoryEntry::scope_paths_from_json(&json).unwrap();

    assert_eq!(entry.scope_paths, parsed);
}
