use std::ffi::OsStr;
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
        if has_safe_absolute_shape(&path)
            && path.file_name() == Some(OsStr::new(tool_name))
        {
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
    use super::safe_cli_path_from_shell_output;

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
}
