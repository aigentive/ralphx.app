use std::path::PathBuf;

use sha2::{Digest, Sha256};

/// RalphX-owned log directory for backend/runtime logs.
///
/// Dev builds keep logs in the source checkout `.artifacts/logs`; release builds
/// keep them under the platform application data directory. Target project
/// worktrees must never be used as the fallback for RalphX runtime logs.
pub fn app_log_dir() -> PathBuf {
    if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.artifacts/logs")
    } else {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("com.ralphx.app")
            .join("logs")
    }
}

/// RalphX-owned directory for MCP proxy JSONL trace files.
pub fn mcp_proxy_trace_dir() -> PathBuf {
    app_log_dir().join("mcp-proxy")
}

pub fn codex_prompt_debug_dir() -> PathBuf {
    app_log_dir().join("codex-prompts")
}

pub fn merge_validation_log_dir(task_id: &str) -> PathBuf {
    app_log_dir()
        .join("merge-validation")
        .join(hashed_log_component("task", task_id))
}

pub fn codex_prompt_debug_file(mode: &str) -> PathBuf {
    let mode = match mode {
        "exec" => "exec",
        "resume" => "resume",
        _ => "unknown",
    };
    app_log_dir().join("codex-prompts").join(format!(
        "{}-{}-{}.txt",
        chrono::Utc::now().format("%Y%m%dT%H%M%S%.3fZ"),
        mode,
        uuid::Uuid::new_v4()
    ))
}

fn hashed_log_component(prefix: &str, value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    let mut encoded = String::with_capacity(24);
    for byte in &digest[..12] {
        use std::fmt::Write as _;
        let _ = write!(&mut encoded, "{byte:02x}");
    }
    format!("{prefix}-{encoded}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_validation_log_dir_hashes_task_id_components() {
        let path = merge_validation_log_dir("../task/with\\separators");
        let suffix = path
            .file_name()
            .and_then(|value| value.to_str())
            .expect("log dir suffix");

        assert!(path.starts_with(app_log_dir().join("merge-validation")));
        assert!(suffix.starts_with("task-"));
        assert!(!suffix.contains(".."));
        assert!(!suffix.contains('/'));
        assert!(!suffix.contains('\\'));
    }

    #[test]
    fn codex_prompt_debug_file_maps_unknown_modes_to_fixed_component() {
        let path = codex_prompt_debug_file("../resume");
        let filename = path
            .file_name()
            .and_then(|value| value.to_str())
            .expect("prompt debug filename");

        assert!(path.starts_with(codex_prompt_debug_dir()));
        assert!(filename.contains("-unknown-"));
        assert!(!filename.contains(".."));
        assert!(!filename.contains('/'));
        assert!(!filename.contains('\\'));
    }
}
