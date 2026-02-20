use super::*;

#[test]
fn activity_event_filter_input_to_domain_empty() {
    let input = ActivityEventFilterInput::default();
    let filter = input.to_domain_filter();
    assert!(filter.is_empty());
}

#[test]
fn activity_event_filter_input_to_domain_with_event_types() {
    let input = ActivityEventFilterInput {
        event_types: Some(vec!["thinking".to_string(), "text".to_string()]),
        roles: None,
        statuses: None,
        task_id: None,
        session_id: None,
    };
    let filter = input.to_domain_filter();
    assert!(!filter.is_empty());
    assert!(filter.event_types.is_some());
    assert_eq!(filter.event_types.unwrap().len(), 2);
}

#[test]
fn activity_event_filter_input_to_domain_with_roles() {
    let input = ActivityEventFilterInput {
        event_types: None,
        roles: Some(vec!["agent".to_string()]),
        statuses: None,
        task_id: None,
        session_id: None,
    };
    let filter = input.to_domain_filter();
    assert!(!filter.is_empty());
    assert!(filter.roles.is_some());
}

#[test]
fn activity_event_filter_input_to_domain_with_statuses() {
    let input = ActivityEventFilterInput {
        event_types: None,
        roles: None,
        statuses: Some(vec!["executing".to_string()]),
        task_id: None,
        session_id: None,
    };
    let filter = input.to_domain_filter();
    assert!(!filter.is_empty());
    assert!(filter.statuses.is_some());
}

#[test]
fn activity_event_filter_input_to_domain_ignores_invalid() {
    let input = ActivityEventFilterInput {
        event_types: Some(vec!["invalid_type".to_string()]),
        roles: Some(vec!["invalid_role".to_string()]),
        statuses: Some(vec!["invalid_status".to_string()]),
        task_id: None,
        session_id: None,
    };
    let filter = input.to_domain_filter();
    // Invalid values are filtered out, leaving an empty filter
    assert!(filter.is_empty());
}

#[test]
fn activity_event_filter_input_to_domain_with_task_id() {
    let input = ActivityEventFilterInput {
        event_types: None,
        roles: None,
        statuses: None,
        task_id: Some("test-task-123".to_string()),
        session_id: None,
    };
    let filter = input.to_domain_filter();
    assert!(!filter.is_empty());
    assert!(filter.task_id.is_some());
    assert_eq!(filter.task_id.unwrap().as_str(), "test-task-123");
}

#[test]
fn activity_event_filter_input_to_domain_with_session_id() {
    let input = ActivityEventFilterInput {
        event_types: None,
        roles: None,
        statuses: None,
        task_id: None,
        session_id: Some("test-session-456".to_string()),
    };
    let filter = input.to_domain_filter();
    assert!(!filter.is_empty());
    assert!(filter.session_id.is_some());
    assert_eq!(filter.session_id.unwrap().as_str(), "test-session-456");
}
