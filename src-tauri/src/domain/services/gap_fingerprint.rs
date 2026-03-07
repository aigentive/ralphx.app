/// 4-layer normalization pipeline for gap description fingerprinting.
/// Used to detect duplicate/convergent gaps across verification rounds.
use sha2::{Digest, Sha256};
use std::collections::HashSet;

const STOP_WORDS: &[&str] = &[
    "a", "an", "the", "is", "are", "was", "were", "be", "been", "has", "have", "had", "do",
    "does", "did", "will", "would", "should", "could", "can", "may", "might", "on", "in", "at",
    "for", "of", "to", "by", "with", "from", "that", "this", "these", "those", "it", "its",
    "and", "but", "or", "so", "yet",
];
// NEVER strip: "no", "not", "missing", "lacks", "without", "absent"

fn stem(word: &str) -> String {
    for (suffix, replacement) in &[
        ("ation", ""),
        ("ating", ""),
        ("ting", ""),
        ("ing", ""),
        ("ment", ""),
        ("ness", ""),
        ("able", ""),
        ("ible", ""),
        ("ed", ""),
        ("er", ""),
        ("es", ""),
        ("s", ""),
    ] {
        if word.len() > suffix.len() + 2 && word.ends_with(suffix) {
            return word[..word.len() - suffix.len()].to_string() + replacement;
        }
    }
    word.to_string()
}

/// Compute a 12-character hex fingerprint for a gap description.
///
/// Pipeline:
/// 1. Lowercase + strip non-alphanumeric (keep whitespace)
/// 2. Stop-word removal (preserves negation words like "not", "missing")
/// 3. Suffix stemming (10 rules, longest-first)
/// 4. Sort + SHA-256 (first 12 hex chars)
pub fn gap_fingerprint(description: &str) -> String {
    // Layer 1: lowercase + strip punctuation
    let cleaned: String = description
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect();

    // Layer 2: stop-word stripping
    let words: Vec<&str> = cleaned
        .split_whitespace()
        .filter(|w| !STOP_WORDS.contains(w))
        .collect();

    // Layer 3: suffix stemming
    let mut stemmed: Vec<String> = words.iter().map(|w| stem(w)).collect();

    // Layer 4: sort + sha256 (first 12 hex chars)
    stemmed.sort();
    let joined = stemmed.join(" ");
    let mut hasher = Sha256::new();
    hasher.update(joined.as_bytes());
    let hash = hasher.finalize();
    format!("{:x}", hash)[..12].to_string()
}

/// Compute Jaccard similarity between two sets of strings.
///
/// Returns 1.0 for empty sets (two empty sets are considered identical).
pub fn jaccard_similarity(set_a: &HashSet<String>, set_b: &HashSet<String>) -> f64 {
    let intersection = set_a.intersection(set_b).count();
    let union = set_a.union(set_b).count();
    if union == 0 {
        return 1.0;
    }
    intersection as f64 / union as f64
}

/// Compute aggregate gap score: critical*10 + high*3 + medium*1
pub fn gap_score(gaps: &[crate::domain::entities::ideation::VerificationGap]) -> u32 {
    gaps.iter().fold(0u32, |acc, g| {
        acc + match g.severity.as_str() {
            "critical" => 10,
            "high" => 3,
            "medium" => 1,
            _ => 0,
        }
    })
}

#[cfg(test)]
#[path = "gap_fingerprint_tests.rs"]
mod tests;
