use super::*;

use super::*;

#[test]
fn session_relationship_default_is_follow_on() {
    assert_eq!(
        SessionRelationship::default(),
        SessionRelationship::FollowOn
    );
}

#[test]
fn session_relationship_from_str_parses_all_variants() {
    assert_eq!(
        "follow_on".parse::<SessionRelationship>().unwrap(),
        SessionRelationship::FollowOn
    );
    assert_eq!(
        "alternative".parse::<SessionRelationship>().unwrap(),
        SessionRelationship::Alternative
    );
    assert_eq!(
        "dependency".parse::<SessionRelationship>().unwrap(),
        SessionRelationship::Dependency
    );
}

#[test]
fn session_relationship_from_str_returns_error_for_unknown() {
    let result = "unknown".parse::<SessionRelationship>();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Unknown session relationship: unknown");
}

#[test]
fn session_relationship_display_formats_correctly() {
    assert_eq!(SessionRelationship::FollowOn.to_string(), "follow_on");
    assert_eq!(SessionRelationship::Alternative.to_string(), "alternative");
    assert_eq!(SessionRelationship::Dependency.to_string(), "dependency");
}

#[test]
fn session_link_new_creates_with_defaults() {
    let parent = IdeationSessionId::from_string("parent-123");
    let child = IdeationSessionId::from_string("child-456");
    let link = SessionLink::new(parent.clone(), child.clone(), SessionRelationship::FollowOn);

    assert_eq!(link.parent_session_id, parent);
    assert_eq!(link.child_session_id, child);
    assert_eq!(link.relationship, SessionRelationship::FollowOn);
    assert!(link.notes.is_none());
    assert!(link.created_at <= Utc::now());
}

#[test]
fn session_link_with_notes_includes_notes() {
    let parent = IdeationSessionId::from_string("parent-123");
    let child = IdeationSessionId::from_string("child-456");
    let link = SessionLink::with_notes(
        parent.clone(),
        child.clone(),
        SessionRelationship::Alternative,
        "Exploring different approach",
    );

    assert_eq!(link.parent_session_id, parent);
    assert_eq!(link.child_session_id, child);
    assert_eq!(link.relationship, SessionRelationship::Alternative);
    assert_eq!(link.notes.as_deref(), Some("Exploring different approach"));
}

#[test]
fn session_link_serializes_to_json() {
    let parent = IdeationSessionId::from_string("parent-123");
    let child = IdeationSessionId::from_string("child-456");
    let link = SessionLink::new(parent, child, SessionRelationship::FollowOn);

    let json = serde_json::to_value(&link).expect("Should serialize");
    assert_eq!(json["relationship"], "follow_on");
    assert_eq!(json["parent_session_id"], "parent-123");
    assert_eq!(json["child_session_id"], "child-456");
}
