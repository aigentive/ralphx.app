use super::*;

#[test]
fn test_tokenize_empty() {
    let result = tokenize_for_similarity("");
    assert!(result.is_empty());
}

#[test]
fn test_tokenize_stop_words_removed() {
    let result = tokenize_for_similarity("the quick brown fox");
    assert!(!result.contains("the"));
    assert!(result.contains("quick") || result.iter().any(|s| s.starts_with("quick")));
}

#[test]
fn test_tokenize_negation_preserved() {
    let result = tokenize_for_similarity("not missing without");
    // negation words should NOT be removed
    assert!(result.contains("not") || result.iter().any(|s| s == "not"));
    assert!(result.contains("miss") || result.iter().any(|s| s.contains("miss")));
}

#[test]
fn test_jaccard_empty_sets() {
    let a = std::collections::HashSet::new();
    let b = std::collections::HashSet::new();
    assert_eq!(jaccard_similarity(&a, &b), 1.0);
}

#[test]
fn test_jaccard_identical() {
    let a: std::collections::HashSet<String> =
        ["foo", "bar"].iter().map(|s| s.to_string()).collect();
    let b = a.clone();
    assert_eq!(jaccard_similarity(&a, &b), 1.0);
}

#[test]
fn test_jaccard_disjoint() {
    let a: std::collections::HashSet<String> = ["foo"].iter().map(|s| s.to_string()).collect();
    let b: std::collections::HashSet<String> = ["bar"].iter().map(|s| s.to_string()).collect();
    assert_eq!(jaccard_similarity(&a, &b), 0.0);
}

#[test]
fn test_jaccard_partial_overlap() {
    let a: std::collections::HashSet<String> =
        ["foo", "bar"].iter().map(|s| s.to_string()).collect();
    let b: std::collections::HashSet<String> =
        ["bar", "baz"].iter().map(|s| s.to_string()).collect();
    // intersection = {bar}, union = {foo, bar, baz} → 1/3
    let similarity = jaccard_similarity(&a, &b);
    assert!((similarity - 1.0 / 3.0).abs() < 1e-9);
}
