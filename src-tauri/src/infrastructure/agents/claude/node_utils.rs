use std::path::PathBuf;

#[cfg(test)]
#[path = "node_utils_tests.rs"]
mod tests;

/// Resolve the Node.js binary path for macOS GUI app contexts.
///
/// macOS apps launched from Finder/Dock have a stripped PATH (/usr/bin:/bin only),
/// so `which::which` may fail. Falls back to common install paths.
///
/// Priority:
/// 1. `RALPHX_NODE_PATH` env var (explicit override)
/// 2. `which::which("node")` (PATH-based lookup)
/// 3. `/opt/homebrew/bin/node` (Homebrew ARM / Apple Silicon)
/// 4. `/usr/local/bin/node` (Homebrew Intel)
/// 5. nvm latest (reads `~/.nvm/versions/node`, picks highest version)
/// 6. `"node"` (last resort — rely on whatever PATH the process has)
///
/// # Errors
///
/// This function never errors — it always returns a path, falling back to the bare
/// `"node"` string if nothing else is found.
pub fn find_node_binary() -> PathBuf {
    // 1. Explicit override via env var — caller controls exact binary.
    if let Ok(path) = std::env::var("RALPHX_NODE_PATH") {
        let p = PathBuf::from(&path);
        if p.exists() {
            return p;
        }
    }

    // 2. PATH-based resolution (works in terminal contexts; fails in GUI apps).
    if let Ok(path) = which::which("node") {
        return path;
    }

    // 3. Homebrew ARM (Apple Silicon default).
    let homebrew_arm = PathBuf::from("/opt/homebrew/bin/node");
    if homebrew_arm.exists() {
        return homebrew_arm;
    }

    // 4. Homebrew Intel.
    let homebrew_intel = PathBuf::from("/usr/local/bin/node");
    if homebrew_intel.exists() {
        return homebrew_intel;
    }

    // 5. nvm latest.
    if let Some(nvm_node) = find_nvm_latest() {
        return nvm_node;
    }

    // 6. Last resort — bare "node", rely on whatever PATH the process inherits.
    PathBuf::from("node")
}

/// Find the latest Node binary installed via nvm.
///
/// Reads `~/.nvm/versions/node/`, sorts version directories lexicographically
/// in descending order (e.g. `v22.x.x` > `v20.x.x` > `v18.x.x`), and returns
/// the `bin/node` path from the highest version found.
fn find_nvm_latest() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let versions_dir = PathBuf::from(home).join(".nvm/versions/node");

    let mut entries: Vec<_> = std::fs::read_dir(&versions_dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .collect();

    // Sort descending — "v22.0.0" > "v20.11.0" > "v18.0.0".
    entries.sort_by_key(|e| std::cmp::Reverse(e.file_name()));

    for entry in entries {
        let node_bin = entry.path().join("bin/node");
        if node_bin.exists() {
            return Some(node_bin);
        }
    }

    None
}
