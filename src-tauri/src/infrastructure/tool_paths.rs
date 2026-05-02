use std::ffi::{OsStr, OsString};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

pub(crate) fn resolve_gh_cli_path() -> PathBuf {
    resolve_cli_path("gh", &["/opt/homebrew/bin/gh", "/usr/local/bin/gh"])
}

pub(crate) fn resolve_git_cli_path() -> PathBuf {
    resolve_cli_path(
        "git",
        &[
            "/opt/homebrew/bin/git",
            "/usr/local/bin/git",
            "/usr/bin/git",
        ],
    )
}

pub(crate) fn resolve_node_cli_path() -> PathBuf {
    find_env_override_path("RALPHX_NODE_PATH").unwrap_or_else(|| {
        resolve_cli_path("node", &["/opt/homebrew/bin/node", "/usr/local/bin/node"])
    })
}

pub(crate) fn resolve_shell_cli_path() -> PathBuf {
    resolve_cli_path("sh", &["/bin/sh", "/usr/bin/sh"])
}

pub(crate) fn resolve_rm_cli_path() -> PathBuf {
    resolve_cli_path("rm", &["/bin/rm", "/usr/bin/rm"])
}

pub(crate) fn resolve_ps_cli_path() -> PathBuf {
    resolve_cli_path("ps", &["/bin/ps", "/usr/bin/ps"])
}

pub(crate) fn resolve_lsof_cli_path() -> PathBuf {
    resolve_cli_path("lsof", &["/usr/sbin/lsof", "/usr/bin/lsof"])
}

pub(crate) fn resolve_pgrep_cli_path() -> PathBuf {
    resolve_cli_path("pgrep", &["/usr/bin/pgrep"])
}

pub(crate) fn resolve_pkill_cli_path() -> PathBuf {
    resolve_cli_path("pkill", &["/usr/bin/pkill"])
}

pub(crate) fn agent_subprocess_env_path() -> OsString {
    agent_subprocess_env_path_from_parts(
        std::env::var_os("PATH").as_deref(),
        dirs::home_dir().as_deref(),
    )
}

pub(crate) fn agent_subprocess_env_path_from_parts(
    existing_path: Option<&OsStr>,
    home_dir: Option<&Path>,
) -> OsString {
    let mut entries = Vec::new();
    if let Some(existing_path) = existing_path {
        entries.extend(std::env::split_paths(existing_path));
    }

    entries.extend(
        [
            "/opt/homebrew/bin",
            "/opt/homebrew/sbin",
            "/usr/local/bin",
            "/usr/local/sbin",
            "/usr/bin",
            "/bin",
            "/usr/sbin",
            "/sbin",
        ]
        .into_iter()
        .map(PathBuf::from),
    );

    if let Some(home_dir) = home_dir {
        entries.extend(
            [
                ".local/bin",
                "bin",
                ".cargo/bin",
                ".rbenv/bin",
                ".rbenv/shims",
                ".asdf/bin",
                ".asdf/shims",
                ".pyenv/bin",
                ".pyenv/shims",
                ".nodenv/bin",
                ".nodenv/shims",
                ".volta/bin",
            ]
            .into_iter()
            .map(|relative| home_dir.join(relative)),
        );
    }

    let mut seen = std::collections::HashSet::new();
    entries.retain(|entry| seen.insert(entry.as_os_str().to_os_string()));
    std::env::join_paths(entries).unwrap_or_else(|_| {
        existing_path
            .map(OsStr::to_os_string)
            .unwrap_or_else(|| OsString::from("/usr/bin:/bin:/usr/sbin:/sbin"))
    })
}

#[cfg(windows)]
pub(crate) fn resolve_taskkill_cli_path() -> PathBuf {
    resolve_cli_path("taskkill", &[])
}

#[cfg(windows)]
pub(crate) fn resolve_tasklist_cli_path() -> PathBuf {
    resolve_cli_path("tasklist", &[])
}

pub(crate) fn find_claude_cli_path() -> Option<PathBuf> {
    find_cli_path(
        "claude",
        &[
            "/opt/homebrew/bin/claude",
            "/usr/local/bin/claude",
            "/usr/bin/claude",
        ],
    )
}

pub(crate) fn find_codex_cli_path() -> Option<PathBuf> {
    find_cli_path(
        "codex",
        &[
            "/opt/homebrew/bin/codex",
            "/usr/local/bin/codex",
            "/usr/bin/codex",
        ],
    )
}

fn resolve_cli_path(tool_name: &'static str, fixed_candidates: &[&'static str]) -> PathBuf {
    find_cli_path(tool_name, fixed_candidates).unwrap_or_else(|| PathBuf::from(tool_name))
}

fn find_cli_path(tool_name: &'static str, fixed_candidates: &[&'static str]) -> Option<PathBuf> {
    if let Ok(path) = which::which(tool_name) {
        if has_safe_absolute_shape(&path) && path.file_name() == Some(OsStr::new(tool_name)) {
            return Some(path);
        }
    }

    for candidate in fixed_candidates {
        let path = PathBuf::from(candidate);
        // Fixed, app-owned candidate list for GUI launches with stripped PATH.
        // codeql[rust/path-injection]
        if path.exists() {
            return Some(path);
        }
    }

    find_login_shell_cli(tool_name)
}

fn find_env_override_path(env_var: &'static str) -> Option<PathBuf> {
    let path = PathBuf::from(std::env::var(env_var).ok()?);
    has_safe_absolute_shape(&path).then_some(path)
}

fn find_login_shell_cli(tool_name: &'static str) -> Option<PathBuf> {
    let command = format!("command -v {tool_name}");
    let output = Command::new("/bin/zsh")
        .args(["-lc", command.as_str()])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    safe_cli_path_from_shell_output(tool_name, &stdout)
}

fn safe_cli_path_from_shell_output(tool_name: &str, output: &str) -> Option<PathBuf> {
    output.lines().rev().find_map(|line| {
        let candidate = PathBuf::from(line.trim());
        if has_safe_absolute_shape(&candidate)
            && candidate.file_name() == Some(OsStr::new(tool_name))
        {
            Some(candidate)
        } else {
            None
        }
    })
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

#[cfg(test)]
mod tests {
    use super::{agent_subprocess_env_path_from_parts, safe_cli_path_from_shell_output};
    use std::ffi::OsStr;
    use std::path::Path;

    #[test]
    fn shell_output_accepts_safe_absolute_matching_tool_path() {
        let path = safe_cli_path_from_shell_output(
            "claude",
            "startup noise\n/Users/example/.local/bin/claude\n",
        );

        assert_eq!(
            path.as_deref().and_then(|value| value.to_str()),
            Some("/Users/example/.local/bin/claude")
        );
    }

    #[test]
    fn shell_output_rejects_relative_or_mismatched_paths() {
        assert!(safe_cli_path_from_shell_output("claude", "../bin/claude").is_none());
        assert!(safe_cli_path_from_shell_output("claude", "/tmp/codex").is_none());
    }

    #[test]
    fn agent_subprocess_path_preserves_existing_path_and_adds_common_dev_bins() {
        let path = agent_subprocess_env_path_from_parts(
            Some(OsStr::new("/existing/bin:/usr/bin")),
            Some(Path::new("/Users/example")),
        );
        let path = path.to_string_lossy();

        assert!(path.contains("/existing/bin"));
        assert!(path.contains("/opt/homebrew/bin"));
        assert!(path.contains("/usr/local/bin"));
        assert!(path.contains("/Users/example/.cargo/bin"));
        assert!(path.contains("/Users/example/.asdf/shims"));
    }
}
