use crate::infrastructure::agents::claude::node_utils::find_node_binary;
use std::path::PathBuf;

/// Temporarily set an env var, restoring the original value on drop.
struct EnvGuard {
    key: &'static str,
    original: Option<String>,
}

impl EnvGuard {
    fn set(key: &'static str, val: &str) -> Self {
        let original = std::env::var(key).ok();
        std::env::set_var(key, val);
        EnvGuard { key, original }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(val) => std::env::set_var(self.key, val),
            None => std::env::remove_var(self.key),
        }
    }
}

#[test]
fn test_ralphx_node_path_override_uses_existing_file() {
    // /bin/sh is a stand-in for "an executable that definitely exists".
    let _guard = EnvGuard::set("RALPHX_NODE_PATH", "/bin/sh");
    let result = find_node_binary();
    assert_eq!(result, PathBuf::from("/bin/sh"));
}

#[test]
fn test_ralphx_node_path_nonexistent_falls_through() {
    // RALPHX_NODE_PATH points to a nonexistent file — should fall through gracefully.
    let _guard = EnvGuard::set("RALPHX_NODE_PATH", "/nonexistent/__no_such_binary__");
    // Must not panic; must return a non-empty path via other resolution steps.
    let result = find_node_binary();
    assert!(
        !result.as_os_str().is_empty(),
        "find_node_binary() should never return an empty path"
    );
    // Verify the nonexistent path was NOT returned.
    assert_ne!(result, PathBuf::from("/nonexistent/__no_such_binary__"));
}

#[test]
fn test_find_node_binary_returns_nonempty_path() {
    // In any dev/CI environment, Node must be discoverable via one of the resolution steps.
    let result = find_node_binary();
    assert!(
        !result.as_os_str().is_empty(),
        "find_node_binary() should never return an empty path"
    );
}
