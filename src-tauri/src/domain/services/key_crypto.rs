// Pure crypto functions for API key management.
// No infrastructure dependencies — safe to use from any layer.

/// Generate a new raw API key in the format: rxk_live_{32 random alphanumeric chars}
pub fn generate_raw_key() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();
    let random_part: String = (0..32)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();
    format!("rxk_live_{}", random_part)
}

/// SHA-256 hash a raw key for storage (only hash is stored, never raw key)
pub fn hash_key(raw_key: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(raw_key.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Extract the display prefix from a raw key (first 12 chars, e.g. "rxk_live_a3f2")
pub fn key_prefix(raw_key: &str) -> String {
    raw_key.chars().take(12).collect()
}
