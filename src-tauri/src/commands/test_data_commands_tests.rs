use super::*;

#[test]
fn test_profile_parsing() {
    assert_eq!(
        serde_json::from_str::<TestDataProfile>(r#""minimal""#).unwrap(),
        TestDataProfile::Minimal
    );
    assert_eq!(
        serde_json::from_str::<TestDataProfile>(r#""kanban""#).unwrap(),
        TestDataProfile::Kanban
    );
}
