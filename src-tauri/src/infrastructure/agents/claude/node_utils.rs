use std::path::{Component, Path, PathBuf};

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
/// 5. login shell `command -v node` (covers nvm/asdf/volta shell init)
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
        if has_safe_absolute_shape(&p) {
            return p;
        }
    }

    // 2. PATH-based resolution (works in terminal contexts; fails in GUI apps).
    if let Ok(path) = which::which("node") {
        return path;
    }

    // 3. Homebrew ARM (Apple Silicon default).
    let homebrew_arm = PathBuf::from("/opt/homebrew/bin/node");
    // Fixed Homebrew path.
    // codeql[rust/path-injection]
    if homebrew_arm.exists() {
        return homebrew_arm;
    }

    // 4. Homebrew Intel.
    let homebrew_intel = PathBuf::from("/usr/local/bin/node");
    // Fixed Homebrew path.
    // codeql[rust/path-injection]
    if homebrew_intel.exists() {
        return homebrew_intel;
    }

    // 5. Login shell lookup for nvm/asdf/volta paths initialized by shell startup files.
    if let Some(shell_node) = find_login_shell_node() {
        return shell_node;
    }

    // 6. Last resort — bare "node", rely on whatever PATH the process inherits.
    PathBuf::from("node")
}

fn has_safe_absolute_shape(path: &Path) -> bool {
    if !path.is_absolute() {
        return false;
    }

    let mut normal_components = 0usize;
    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => {}
            Component::Normal(_) => normal_components += 1,
            Component::ParentDir | Component::CurDir => return false,
        }
    }

    normal_components > 0
}

fn find_login_shell_node() -> Option<PathBuf> {
    let output = std::process::Command::new("/bin/zsh")
        .args(["-lc", "command -v node"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let path = String::from_utf8(output.stdout).ok()?;
    let candidate = PathBuf::from(path.trim());
    has_safe_absolute_shape(&candidate).then_some(candidate)
}
