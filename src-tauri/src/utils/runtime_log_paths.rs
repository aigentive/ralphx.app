use std::path::PathBuf;

const MAC_APP_SUPPORT_LOG_DIR: &str = "Library/Application Support/com.ralphx.app/logs";

/// RalphX-owned log directory for backend/runtime logs.
///
/// Dev builds keep logs in the source checkout `.artifacts/logs`; release builds
/// keep them under the macOS application support directory. Target project
/// worktrees must never be used as the fallback for RalphX runtime logs.
pub fn app_log_dir() -> PathBuf {
    if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.artifacts/logs")
    } else {
        let home = std::env::var("HOME").expect("HOME environment variable not set");
        PathBuf::from(home).join(MAC_APP_SUPPORT_LOG_DIR)
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
    app_log_dir().join("merge-validation").join(task_id)
}
