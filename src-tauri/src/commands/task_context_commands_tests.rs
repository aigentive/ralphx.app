use super::*;
use crate::domain::entities::ArtifactType;

#[test]
fn test_content_preview_short() {
    let artifact =
        Artifact::new_inline("Test", ArtifactType::Specification, "Short content", "user");
    let preview = create_content_preview(&artifact);
    assert_eq!(preview, "Short content");
}

#[test]
fn test_content_preview_long() {
    let long_content = "x".repeat(600);
    let artifact =
        Artifact::new_inline("Test", ArtifactType::Specification, long_content, "user");
    let preview = create_content_preview(&artifact);
    assert_eq!(preview.len(), 503); // 500 + "..."
    assert!(preview.ends_with("..."));
}

#[test]
fn test_content_preview_file() {
    let artifact = Artifact::new_file(
        "Test",
        ArtifactType::Specification,
        "/path/to/file.md",
        "user",
    );
    let preview = create_content_preview(&artifact);
    assert!(preview.contains("/path/to/file.md"));
}
